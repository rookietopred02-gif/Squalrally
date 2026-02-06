use arc_swap::ArcSwap;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::MutexGuard;
use std::time::Instant;

pub struct WriteGuard<'lifetime, T: Clone + Send + Sync + 'static> {
    arc_swap: &'lifetime ArcSwap<T>,
    uncomitted_value_ref: Arc<T>,
    committed: bool,
    // Ensures only one writer exists per dependency type at a time.
    _write_lock: MutexGuard<'static, ()>,
    trace: Option<WriteGuardTrace>,
}

struct WriteGuardTrace {
    type_name: &'static str,
    context: &'static str,
    acquired_at: Instant,
    thread_id: std::thread::ThreadId,
}

impl<'lifetime, T: Clone + Send + Sync + 'static> WriteGuard<'lifetime, T> {
    pub fn new(
        arc_swap: &'lifetime ArcSwap<T>,
        write_lock: MutexGuard<'static, ()>,
        trace_enabled: bool,
        type_name: &'static str,
        context: &'static str,
    ) -> Self {
        // IMPORTANT:
        // Do not take a reference to a temporary `Guard` returned by `ArcSwap::load()`.
        // If the guard is dropped before cloning the `Arc`, another thread can `store()`
        // a new value and free the old `Arc`, leading to use-after-free UB.
        let uncomitted_value = arc_swap.load_full();

        let trace = if trace_enabled {
            let trace_path = std::env::temp_dir().join("squalr_lock_trace.log");
            let acquired_at = Instant::now();
            let thread_id = std::thread::current().id();
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&trace_path)
            {
                let _ = writeln!(
                    file,
                    "ACQUIRE write lock: type={} context={} thread={:?} at={:?}",
                    type_name, context, thread_id, acquired_at
                );
                let _ = file.flush();
            }

            Some(WriteGuardTrace {
                type_name,
                context,
                acquired_at,
                thread_id,
            })
        } else {
            None
        };

        Self {
            arc_swap,
            uncomitted_value_ref: uncomitted_value,
            committed: false,
            _write_lock: write_lock,
            trace,
        }
    }

    /// Commit now (still commits on Drop unless you mark committed = true).
    pub fn commit(&mut self) {
        self.arc_swap.store(self.uncomitted_value_ref.clone());
        self.committed = true;
    }

    /// Prevent commit on Drop
    pub fn abort(&mut self) {
        self.committed = true;
    }
}

impl<'lifetime, T: Clone + Send + Sync + 'static> Deref for WriteGuard<'lifetime, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.uncomitted_value_ref.as_ref()
    }
}

impl<'lifetime, T: Clone + Send + Sync + 'static> DerefMut for WriteGuard<'lifetime, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Clones T only if Arc is shared.
        Arc::make_mut(&mut self.uncomitted_value_ref)
    }
}

impl<'lifetime, T: Clone + Send + Sync + 'static> Drop for WriteGuard<'lifetime, T> {
    fn drop(&mut self) {
        if !self.committed {
            self.arc_swap.store(self.uncomitted_value_ref.clone());
        }

        if let Some(trace) = self.trace.take() {
            let trace_path = std::env::temp_dir().join("squalr_lock_trace.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&trace_path)
            {
                let held_for = trace.acquired_at.elapsed();
                let _ = writeln!(
                    file,
                    "RELEASE write lock: type={} context={} thread={:?} held_for={:?}\n",
                    trace.type_name, trace.context, trace.thread_id, held_for
                );
                let _ = file.flush();
            }
        }
    }
}
