use crate::dependency_injection::dependency_container::DependencyContainer;
use crate::dependency_injection::write_guard::WriteGuard;
use anyhow::Result;
use anyhow::anyhow;
use arc_swap::ArcSwap;
use arc_swap::Guard;
use std::any::TypeId;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, OnceLock};
use std::sync::Mutex;
use std::time::Instant;

/// A clone-safe wrapper for injected and lock-free dependencies. Requires clonable types to achieve lock-free.
pub struct Dependency<T: Clone + Send + Sync + 'static> {
    container: DependencyContainer,
    instance: Arc<OnceLock<Arc<ArcSwap<T>>>>,
}

impl<T: Clone + Send + Sync + 'static> Clone for Dependency<T> {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            instance: self.instance.clone(),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Dependency<T> {
    fn get_write_mutex_for_type() -> &'static Mutex<()> {
        // Ensure exclusive writers per dependency type to avoid lost updates when callbacks and the UI
        // mutate the same dependency concurrently (ArcSwap is last-writer-wins otherwise).
        //
        // IMPORTANT: A `static` inside a generic method is **shared across all T** (not per-T),
        // which can cause UI hangs/deadlocks when code writes to multiple dependencies in one frame.
        // We therefore maintain a per-type mutex map keyed by `TypeId`.
        static WRITE_MUTEXES: OnceLock<Mutex<HashMap<TypeId, &'static Mutex<()>>>> = OnceLock::new();
        let write_mutexes = WRITE_MUTEXES.get_or_init(|| Mutex::new(HashMap::new()));

        let write_mutex: &'static Mutex<()> = {
            let mut map = match write_mutexes.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };

            *map.entry(TypeId::of::<T>())
                .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
        };

        write_mutex
    }

    #[cfg(test)]
    pub(crate) fn debug_write_mutex_ptr() -> *const Mutex<()> {
        Self::get_write_mutex_for_type() as *const Mutex<()>
    }

    pub fn new(container: DependencyContainer) -> Self {
        Self {
            container,
            instance: Arc::new(OnceLock::new()),
        }
    }

    /// Get the Arc<RwLock<T>> that lives inside OnceLock
    /// This reference lives as long as &self, so guard lifetimes work.
    fn get_shared_lock(&self) -> Result<&ArcSwap<T>> {
        if let Some(shared_lock) = self.instance.get() {
            return Ok(shared_lock.as_ref());
        }

        // Important: do NOT cache failures. If resolution fails during startup ordering, we want a
        // later retry to succeed once the dependency is registered.
        let resolved = self.container.get_existing::<T>()?;
        let _ = self.instance.set(resolved);

        self.instance
            .get()
            .map(|shared_lock| shared_lock.as_ref())
            .ok_or_else(|| anyhow!("Failed to resolve dependency {}", std::any::type_name::<T>()))
    }

    /// Acquire a read guard.
    pub fn read(
        &self,
        error_context: &'static str,
    ) -> Option<Guard<Arc<T>>> {
        match self.get_shared_lock() {
            Ok(shared_lock) => Some(shared_lock.load()),
            Err(error) => {
                log::error!("Failed to acquire read on dependency: {}, context: {}", error, error_context);
                None
            }
        }
    }

    /// Acquire a write guard.
    pub fn write(
        &self,
        error_context: &'static str,
    ) -> Option<WriteGuard<'_, T>> {
        let write_mutex = Self::get_write_mutex_for_type();
        let trace_locks_enabled = std::env::var_os("SQUALR_TRACE_LOCKS").is_some();

        let write_lock = if !trace_locks_enabled {
            match write_mutex.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            }
        } else {
            match write_mutex.try_lock() {
                Ok(guard) => guard,
                Err(_) => {
                    let trace_path = std::env::temp_dir().join("squalr_lock_trace.log");
                    let start = Instant::now();
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&trace_path)
                    {
                        let backtrace = std::backtrace::Backtrace::force_capture();
                        let _ = writeln!(
                            file,
                            "CONTENDED write lock: type={} context={} thread={:?}\nwait_start={:?}\nbacktrace={}\n",
                            std::any::type_name::<T>(),
                            error_context,
                            std::thread::current().id(),
                            start,
                            backtrace
                        );
                        let _ = file.flush();
                    }

                    match write_mutex.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    }
                }
            }
        };

        match self.get_shared_lock() {
            Ok(shared_lock) => Some(WriteGuard::new(
                shared_lock,
                write_lock,
                trace_locks_enabled,
                std::any::type_name::<T>(),
                error_context,
            )),
            Err(error) => {
                log::error!("Failed to acquire write on dependency: {}, context: {}", error, error_context);
                None
            }
        }
    }

    /// Attempt to acquire a write guard without blocking the calling thread.
    ///
    /// This is primarily used on the UI thread to avoid "App Hang" scenarios when a background
    /// worker is holding the dependency writer mutex. Callers can retry on a later frame.
    pub fn try_write(
        &self,
        error_context: &'static str,
    ) -> Option<WriteGuard<'_, T>> {
        let write_mutex = Self::get_write_mutex_for_type();
        let trace_locks_enabled = std::env::var_os("SQUALR_TRACE_LOCKS").is_some();

        let write_lock = match write_mutex.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                if trace_locks_enabled {
                    let trace_path = std::env::temp_dir().join("squalr_lock_trace.log");
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&trace_path)
                    {
                        let backtrace = std::backtrace::Backtrace::force_capture();
                        let _ = writeln!(
                            file,
                            "TRY_WRITE_FAILED: type={} context={} thread={:?}\nbacktrace={}\n",
                            std::any::type_name::<T>(),
                            error_context,
                            std::thread::current().id(),
                            backtrace
                        );
                        let _ = file.flush();
                    }
                }

                return None;
            }
        };

        match self.get_shared_lock() {
            Ok(shared_lock) => Some(WriteGuard::new(
                shared_lock,
                write_lock,
                trace_locks_enabled,
                std::any::type_name::<T>(),
                error_context,
            )),
            Err(error) => {
                log::error!("Failed to acquire try_write on dependency: {}, context: {}", error, error_context);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Dependency;

    #[derive(Clone)]
    struct DepA;

    #[derive(Clone)]
    struct DepB;

    #[test]
    fn write_mutex_is_per_dependency_type() {
        let a1 = Dependency::<DepA>::debug_write_mutex_ptr() as usize;
        let a2 = Dependency::<DepA>::debug_write_mutex_ptr() as usize;
        let b1 = Dependency::<DepB>::debug_write_mutex_ptr() as usize;

        assert_eq!(a1, a2, "mutex for same type should be stable");
        assert_ne!(a1, b1, "mutex must be per dependency type");
    }
}
