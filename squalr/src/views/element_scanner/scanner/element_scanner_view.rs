use crate::app_context::AppContext;
use crate::views::element_scanner::results::element_scanner_results_view::ElementScannerResultsView;
use crate::views::element_scanner::results::view_data::element_scanner_results_view_data::ElementScannerResultsViewData;
use crate::views::element_scanner::scanner::element_scanner_footer_view::ElementScannerFooterView;
use crate::views::element_scanner::scanner::element_scanner_toolbar_view::ElementScannerToolbarView;
use crate::views::element_scanner::scanner::view_data::element_scanner_view_data::ElementScannerViewData;
use eframe::egui::{Align, Key, Layout, Response, Sense, Ui, UiBuilder, Widget};
use epaint::{Rect, vec2};
use squalr_engine_api::dependency_injection::dependency::Dependency;
use std::sync::Arc;

#[derive(Clone)]
pub struct ElementScannerView {
    _app_context: Arc<AppContext>,
    _element_scanner_view_data: Dependency<ElementScannerViewData>,
    _element_scanner_results_view_data: Dependency<ElementScannerResultsViewData>,
    element_scanner_toolbar_view: ElementScannerToolbarView,
    element_scanner_results_view: ElementScannerResultsView,
    element_scanner_footer_view: ElementScannerFooterView,
}

impl ElementScannerView {
    pub const WINDOW_ID: &'static str = "window_element_scanner";

    pub fn new(app_context: Arc<AppContext>) -> Self {
        let element_scanner_view_data = app_context
            .dependency_container
            .register(ElementScannerViewData::new());
        let element_scanner_results_view_data = app_context
            .dependency_container
            .register(ElementScannerResultsViewData::new());
        ElementScannerViewData::poll_scan_state(
            element_scanner_view_data.clone(),
            app_context.engine_unprivileged_state.clone(),
        );
        ElementScannerResultsViewData::poll_scan_results(
            element_scanner_results_view_data.clone(),
            app_context.engine_unprivileged_state.clone(),
        );
        let element_scanner_toolbar_view = ElementScannerToolbarView::new(app_context.clone());
        let element_scanner_results_view = ElementScannerResultsView::new(app_context.clone());
        let element_scanner_footer_view = ElementScannerFooterView::new(app_context.clone());

        Self {
            _app_context: app_context,
            _element_scanner_view_data: element_scanner_view_data,
            _element_scanner_results_view_data: element_scanner_results_view_data,
            element_scanner_toolbar_view,
            element_scanner_results_view,
            element_scanner_footer_view,
        }
    }
}

impl Widget for ElementScannerView {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        if user_interface.input(|input_state| input_state.key_pressed(Key::Escape)) {
            ElementScannerViewData::cancel_scan(
                self._element_scanner_view_data.clone(),
                self._app_context.engine_unprivileged_state.clone(),
            );
        }

        let response = user_interface
            .allocate_ui_with_layout(user_interface.available_size(), Layout::top_down(Align::Min), |user_interface| {
                user_interface.add(self.element_scanner_toolbar_view.clone());

                let footer_height = self.element_scanner_footer_view.get_height();
                let full_rectangle = user_interface.available_rect_before_wrap();
                let content_rectangle = Rect::from_min_max(full_rectangle.min, full_rectangle.max - vec2(0.0, footer_height));
                let content_response = user_interface.allocate_rect(content_rectangle, Sense::empty());
                let mut content_user_interface = user_interface.new_child(
                    UiBuilder::new()
                        .max_rect(content_response.rect)
                        .layout(Layout::left_to_right(Align::Min)),
                );

                content_user_interface.add(self.element_scanner_results_view.clone());

                user_interface.add(self.element_scanner_footer_view.clone());
            })
            .response;

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_context::AppContext;
    use crate::models::docking::docking_manager::DockingManager;
    use crate::models::docking::hierarchy::dock_node::DockNode;
    use crate::ui::theme::Theme;
    use crate::views::disassembler::view_data::disassembler_view_data::DisassemblerViewData;
    use crate::views::memory_viewer::view_data::memory_viewer_view_data::MemoryViewerViewData;
    use crate::views::pointer_scanner::view_data::pointer_scanner_view_data::PointerScannerViewData;
    use crate::views::struct_viewer::view_data::struct_viewer_view_data::StructViewerViewData;
    use crossbeam_channel::unbounded;
    use squalr_engine_api::engine::engine_api_unprivileged_bindings::EngineApiUnprivilegedBindings;
    use squalr_engine_api::commands::privileged_command::PrivilegedCommand;
    use squalr_engine_api::commands::privileged_command_response::PrivilegedCommandResponse;
    use squalr_engine_api::commands::unprivileged_command::UnprivilegedCommand;
    use squalr_engine_api::commands::unprivileged_command_response::UnprivilegedCommandResponse;
    use squalr_engine_api::engine::engine_unprivileged_state::EngineUnprivilegedState;
    use squalr_engine_api::events::engine_event::EngineEvent;
    use std::sync::RwLock;

    struct MockUnprivilegedBindings;

    impl EngineApiUnprivilegedBindings for MockUnprivilegedBindings {
        fn dispatch_privileged_command(
            &self,
            _engine_command: PrivilegedCommand,
            _callback: Box<dyn FnOnce(PrivilegedCommandResponse) + Send + Sync + 'static>,
        ) -> Result<(), String> {
            Err("Mock bindings: privileged commands not supported in this test".to_string())
        }

        fn dispatch_unprivileged_command(
            &self,
            _engine_command: UnprivilegedCommand,
            _engine_unprivileged_state: &Arc<EngineUnprivilegedState>,
            _callback: Box<dyn FnOnce(UnprivilegedCommandResponse) + Send + Sync + 'static>,
        ) -> Result<(), String> {
            // We don't need engine callbacks for this test; it's enough that UI state updates locally.
            Err("Mock bindings: unprivileged commands not supported in this test".to_string())
        }

        fn subscribe_to_engine_events(&self) -> Result<crossbeam_channel::Receiver<EngineEvent>, String> {
            let (_sender, receiver) = unbounded();
            Ok(receiver)
        }
    }

    fn run_frame_with_input(
        ctx: &eframe::egui::Context,
        element_scanner_view: ElementScannerView,
        mut input: eframe::egui::RawInput,
    ) {
        input.screen_rect = input.screen_rect.or(Some(eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(0.0, 0.0),
            eframe::egui::vec2(800.0, 600.0),
        )));

        ctx.begin_frame(input);
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(element_scanner_view);
        });
        let _ = ctx.end_frame();
    }

    #[test]
    fn escape_cancels_scan_and_does_not_hang() {
        let ctx = eframe::egui::Context::default();
        let theme = Arc::new(Theme::new(&ctx));
        let docking_root = DockNode::Window {
            window_identifier: "dummy".to_string(),
            is_visible: true,
        };
        let docking_manager = Arc::new(std::sync::RwLock::new(DockingManager::new(docking_root)));
        let engine_state = EngineUnprivilegedState::new(Arc::new(RwLock::new(MockUnprivilegedBindings)));
        let app_context = Arc::new(AppContext::new(ctx.clone(), theme, docking_manager, engine_state));

        // Register dependencies required by ElementScannerView.
        app_context.dependency_container.register(StructViewerViewData::new());
        app_context.dependency_container.register(MemoryViewerViewData::new());
        app_context.dependency_container.register(DisassemblerViewData::new());
        app_context.dependency_container.register(PointerScannerViewData::new());

        let element_scanner_view = ElementScannerView::new(app_context.clone());
        let dep = app_context
            .dependency_container
            .get_dependency::<ElementScannerViewData>();

        // Put the scanner into an in-progress state with a task id, so cancel_scan has an effect.
        if let Some(mut view_data) = dep.try_write("Seed scan state for escape cancel test") {
            view_data.view_state = crate::views::element_scanner::scanner::element_scanner_view_state::ElementScannerViewState::ScanInProgress;
            view_data.scan_task_id = Some("dummy-task".to_string());
            view_data.scan_progress = 0.5;
            view_data.last_error_message = None;
        }

        let mut input = eframe::egui::RawInput::default();
        input.events.push(eframe::egui::Event::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
            modifiers: eframe::egui::Modifiers::NONE,
            physical_key: None,
        });
        input.events.push(eframe::egui::Event::Key {
            key: Key::Escape,
            pressed: false,
            repeat: false,
            modifiers: eframe::egui::Modifiers::NONE,
            physical_key: None,
        });

        run_frame_with_input(&ctx, element_scanner_view, input);

        let data = dep.read("Assert scan canceled after escape").expect("read view data");
        assert_eq!(data.scan_task_id, None);
        assert_eq!(data.scan_progress, 0.0);
        assert!(matches!(
            data.view_state,
            crate::views::element_scanner::scanner::element_scanner_view_state::ElementScannerViewState::NoResults
        ));
        assert_eq!(data.last_error_message.as_deref(), Some("Scan canceled."));
    }
}
