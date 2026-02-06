use crate::models::docking::docking_manager::DockingManager;
use crate::models::docking::settings::dockable_window_settings::DockableWindowSettings;
use crate::views::main_window::main_window_view::MainWindowView;
use crate::views::memory_viewer::memory_viewer_view::MemoryViewerView;
use crate::{app_context::AppContext, ui::theme::Theme};
use eframe::egui::{CentralPanel, Context, Frame, ScrollArea, TextEdit, Visuals};
use epaint::{CornerRadius, Rgba, vec2};
use squalr_engine_api::{dependency_injection::dependency_container::DependencyContainer, engine::engine_unprivileged_state::EngineUnprivilegedState};
use std::sync::RwLock;
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub struct App {
    app_context: Arc<AppContext>,
    main_window_view: MainWindowView,
    corner_radius: CornerRadius,
    last_panic: Option<String>,
}

impl App {
    pub fn new(
        context: &Context,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        _dependency_container: &DependencyContainer,
        app_title: String,
    ) -> Self {
        let theme = Arc::new(Theme::new(context));
        // Create built in docked windows.
        let main_dock_root = DockableWindowSettings::get_dock_layout_settings();
        let docking_manager = Arc::new(RwLock::new(DockingManager::new(main_dock_root)));
        let app_context = Arc::new(AppContext::new(context.clone(), theme, docking_manager, engine_unprivileged_state));
        let corner_radius = CornerRadius::same(8);
        let main_window_view = MainWindowView::new(app_context.clone(), Rc::new(app_title), corner_radius);

        Self {
            app_context,
            main_window_view,
            corner_radius,
            last_panic: None,
        }
    }
}

impl eframe::App for App {
    fn clear_color(
        &self,
        _visuals: &Visuals,
    ) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array()
    }

    fn update(
        &mut self,
        context: &Context,
        _frame: &mut eframe::Frame,
    ) {
        let main_window_view = self.main_window_view.clone();
        let app_frame = Frame::new()
            .corner_radius(self.corner_radius)
            .stroke(context.style().visuals.widgets.noninteractive.fg_stroke)
            .outer_margin(2.0);

        // Never allow a panic to cross the UI boundary: in "windows_subsystem=windows" release builds this
        // can look like a silent crash/exit. Instead, capture the panic and render a diagnostic overlay.
        let update_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            CentralPanel::default()
                .frame(app_frame)
                .show(context, move |user_interface| {
                    user_interface.style_mut().spacing.item_spacing = vec2(0.0, 0.0);
                    user_interface.add(main_window_view);
                });

            MemoryViewerView::show_popout_window(self.app_context.clone());
        }));

        if let Err(payload) = update_result {
            let panic_message = if let Some(message) = payload.downcast_ref::<&str>() {
                (*message).to_string()
            } else if let Some(message) = payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "Unknown panic payload".to_string()
            };

            let backtrace = std::backtrace::Backtrace::force_capture();
            let report = format!("{panic_message}\n\n{backtrace}");
            log::error!("UI panic trapped: {panic_message}");

            let ui_panic_log_path = std::env::temp_dir().join("squalr_ui_panic.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&ui_panic_log_path)
            {
                use std::io::Write;
                let _ = writeln!(file, "================ UI panic ================");
                let _ = writeln!(file, "{report}");
                let _ = writeln!(file, "Log: {}", ui_panic_log_path.display());
            }

            self.last_panic = Some(report);
        }

        if let Some(report) = self.last_panic.clone() {
            // Keep the app alive and show a diagnostic overlay.
            CentralPanel::default().show(context, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Squalr recovered from an internal UI error");
                    ui.label("Please copy the report below and file an issue. The app continues running.");

                    ui.separator();

                    ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        let mut text = report.clone();
                        ui.add(
                            TextEdit::multiline(&mut text)
                                .desired_rows(12)
                                .font(eframe::egui::TextStyle::Monospace)
                                .interactive(false),
                        );
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Copy report").clicked() {
                            ui.ctx().copy_text(report.clone());
                        }
                        if ui.button("Dismiss").clicked() {
                            self.last_panic = None;
                        }
                    });
                });
            });
        }
    }
}
