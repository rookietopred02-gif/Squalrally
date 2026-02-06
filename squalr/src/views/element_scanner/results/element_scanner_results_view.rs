use crate::{
    app_context::AppContext,
    ui::{draw::icon_draw::IconDraw, ui_trace, widgets::controls::check_state::CheckState},
    views::{
        disassembler::view_data::disassembler_view_data::DisassemblerViewData,
        element_scanner::{
            results::{
                element_scanner_result_entry_view::ElementScannerResultEntryView,
                element_scanner_results_action_bar_view::ElementScannerResultsActionBarView,
                view_data::{
                    element_scanner_result_frame_action::ElementScannerResultFrameAction, element_scanner_results_view_data::ElementScannerResultsViewData,
                },
            },
            scanner::{element_scanner_view_state::ElementScannerViewState, view_data::element_scanner_view_data::ElementScannerViewData},
        },
        memory_viewer::view_data::memory_viewer_view_data::MemoryViewerViewData,
        pointer_scanner::view_data::pointer_scanner_view_data::PointerScannerViewData,
        struct_viewer::view_data::struct_viewer_view_data::StructViewerViewData,
    },
};
use eframe::egui::{Align, Align2, CursorIcon, Direction, Layout, Response, ScrollArea, Sense, Spinner, Ui, Widget, Window};
use epaint::{Margin, Rect, Vec2, pos2, vec2};
use squalr_engine_api::{dependency_injection::dependency::Dependency, structures::scan_results::scan_result::ScanResult};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone)]
pub struct ElementScannerResultsView {
    app_context: Arc<AppContext>,
    element_scanner_view_data: Dependency<ElementScannerViewData>,
    element_scanner_results_view_data: Dependency<ElementScannerResultsViewData>,
    struct_viewer_view_data: Dependency<StructViewerViewData>,
    memory_viewer_view_data: Dependency<MemoryViewerViewData>,
    disassembler_view_data: Dependency<DisassemblerViewData>,
    pointer_scanner_view_data: Dependency<PointerScannerViewData>,
}

impl ElementScannerResultsView {
    pub const WINDOW_ID: &'static str = "window_element_scanner_results";

    pub fn new(app_context: Arc<AppContext>) -> Self {
        let element_scanner_view_data = app_context
            .dependency_container
            .get_dependency::<ElementScannerViewData>();
        let element_scanner_results_view_data = app_context
            .dependency_container
            .get_dependency::<ElementScannerResultsViewData>();
        let struct_viewer_view_data = app_context
            .dependency_container
            .get_dependency::<StructViewerViewData>();
        let memory_viewer_view_data = app_context
            .dependency_container
            .get_dependency::<MemoryViewerViewData>();
        let disassembler_view_data = app_context
            .dependency_container
            .get_dependency::<DisassemblerViewData>();
        let pointer_scanner_view_data = app_context
            .dependency_container
            .get_dependency::<PointerScannerViewData>();

        Self {
            app_context,
            element_scanner_view_data,
            element_scanner_results_view_data,
            struct_viewer_view_data,
            memory_viewer_view_data,
            disassembler_view_data,
            pointer_scanner_view_data,
        }
    }
}
impl Widget for ElementScannerResultsView {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        const FAUX_BAR_THICKNESS: f32 = 3.0;
        const BAR_THICKNESS: f32 = 4.0;
        const MINIMUM_COLUMN_PIXEL_WIDTH: f32 = 80.0;
        const MINIMUM_SPLITTER_PIXEL_GAP: f32 = 40.0;
        const AUTO_PAGE_SIZE_ROW_HEIGHT: f32 = 32.0;
        const AUTO_PAGE_SIZE_ROW_BUFFER: u32 = 4;
        const AUTO_PAGE_SIZE_MAX: u32 = 1_000_000;

        let theme = &self.app_context.theme;
        let mut new_value_splitter_ratio: Option<f32> = None;
        let mut new_previous_value_splitter_ratio: Option<f32> = None;

        // If a prior frame couldn't apply an action due to lock contention, retry it first.
        let mut element_sanner_result_frame_action: ElementScannerResultFrameAction = self
            .element_scanner_results_view_data
            .read("Element scanner results pending frame action")
            .map(|view_data| view_data.pending_frame_action.clone())
            .unwrap_or(ElementScannerResultFrameAction::None);

        let mut should_open_change_value_dialog = false;
        let mut should_select_all = false;
        let mut should_copy_selected_addresses = false;
        let mut should_copy_selected_rows = false;
        let mut copy_text: Option<String> = None;
        let mut paste_selection_range: Option<(i32, i32)> = None;
        let mut browse_memory_address: Option<u64> = None;
        let mut disassemble_address: Option<u64> = None;
        let mut pointer_scan_address: Option<u64> = None;

        let response = user_interface
            .allocate_ui_with_layout(user_interface.available_size(), Layout::top_down(Align::Min), |mut user_interface| {
                let allocate_resize_bar = |user_interface: &mut Ui, resize_rectangle: Rect, id_suffix: &str| -> Response {
                    let id = user_interface.id().with(id_suffix);
                    let response = user_interface.interact(resize_rectangle, id, Sense::drag());

                    user_interface
                        .painter()
                        .rect_filled(resize_rectangle, 0.0, theme.background_control);

                    response
                };

                let (mut value_splitter_ratio, mut previous_value_splitter_ratio) = match self
                    .element_scanner_results_view_data
                    .read("Element scanner results view")
                {
                    Some(element_scanner_results_view_data) => (
                        element_scanner_results_view_data.value_splitter_ratio,
                        element_scanner_results_view_data.previous_value_splitter_ratio,
                    ),
                    None => return,
                };

                // Draw the header.
                let header_height = 32.0;
                let (header_rectangle, _header_response) =
                    user_interface.allocate_exact_size(vec2(user_interface.available_size().x, header_height), Sense::empty());
                let (separator_rect, _) = user_interface.allocate_exact_size(vec2(user_interface.available_size().x, FAUX_BAR_THICKNESS), Sense::empty());

                user_interface
                    .painter()
                    .rect_filled(separator_rect, 0.0, theme.background_control);

                let footer_height = ElementScannerResultsActionBarView::FOOTER_HEIGHT;
                let content_clip_rectangle = user_interface
                    .available_rect_before_wrap()
                    .with_max_y(user_interface.available_rect_before_wrap().max.y - footer_height);

                let content_width = content_clip_rectangle.width();
                let content_height = content_clip_rectangle.height();
                let content_min_x = content_clip_rectangle.min.x;

                // Clamp splitters to row height.
                let mut rows_min_y: Option<f32> = None;
                let mut rows_max_y: Option<f32> = None;

                if content_width <= 0.0 {
                    return;
                }

                if content_height > 0.0 {
                    let rows_fit = (content_height / AUTO_PAGE_SIZE_ROW_HEIGHT).floor().max(1.0) as u32;
                    let desired_page_size = rows_fit
                        .saturating_add(AUTO_PAGE_SIZE_ROW_BUFFER)
                        .min(AUTO_PAGE_SIZE_MAX)
                        .max(1);

                    ElementScannerResultsViewData::set_page_size_override(
                        self.element_scanner_results_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                        Some(desired_page_size),
                    );
                }

                if value_splitter_ratio <= 0.0 || previous_value_splitter_ratio <= 0.0 || previous_value_splitter_ratio <= value_splitter_ratio {
                    value_splitter_ratio = ElementScannerResultsViewData::DEFAULT_VALUE_SPLITTER_RATIO;
                    previous_value_splitter_ratio = ElementScannerResultsViewData::DEFAULT_PREVIOUS_VALUE_SPLITTER_RATIO;

                    new_value_splitter_ratio = Some(value_splitter_ratio);
                    new_previous_value_splitter_ratio = Some(previous_value_splitter_ratio);
                }

                let value_splitter_position_x = content_min_x + content_width * value_splitter_ratio;
                let previous_value_splitter_position_x = content_min_x + content_width * previous_value_splitter_ratio;
                let faux_address_splitter_position_x = content_min_x + 36.0;

                let splitter_min_y = header_rectangle.min.y;
                let splitter_max_y = content_clip_rectangle.max.y + footer_height;

                let faux_address_splitter_rectangle = Rect::from_min_max(
                    pos2(faux_address_splitter_position_x - FAUX_BAR_THICKNESS * 0.5, splitter_min_y),
                    pos2(faux_address_splitter_position_x + FAUX_BAR_THICKNESS * 0.5, splitter_max_y),
                );

                let value_splitter_rectangle = Rect::from_min_max(
                    pos2(value_splitter_position_x - BAR_THICKNESS * 0.5, splitter_min_y),
                    pos2(value_splitter_position_x + BAR_THICKNESS * 0.5, splitter_max_y),
                );

                let previous_value_splitter_rectangle = Rect::from_min_max(
                    pos2(previous_value_splitter_position_x - BAR_THICKNESS * 0.5, splitter_min_y),
                    pos2(previous_value_splitter_position_x + BAR_THICKNESS * 0.5, splitter_max_y),
                );

                // Freeze column header.
                let freeze_icon_size = vec2(16.0, 16.0);
                let freeze_icon_padding = 8.0;
                let freeze_icon_pos_y = header_rectangle.center().y - freeze_icon_size.y * 0.5;
                let freeze_icon_rectangle = Rect::from_min_size(pos2(header_rectangle.min.x + freeze_icon_padding, freeze_icon_pos_y), freeze_icon_size);

                IconDraw::draw_sized(
                    user_interface,
                    freeze_icon_rectangle.center(),
                    freeze_icon_size,
                    &self.app_context.theme.icon_library.icon_handle_results_freeze,
                );

                // Address column header.
                let text_left_padding = 8.0;
                let address_header_x = faux_address_splitter_position_x + text_left_padding;
                let address_header_position = pos2(address_header_x, header_rectangle.center().y);

                user_interface.painter().text(
                    address_header_position,
                    Align2::LEFT_CENTER,
                    "Address",
                    theme.font_library.font_noto_sans.font_header.clone(),
                    theme.foreground,
                );

                // Value column header.
                let value_label_position = pos2(value_splitter_position_x + text_left_padding, header_rectangle.center().y);

                user_interface.painter().text(
                    value_label_position,
                    Align2::LEFT_CENTER,
                    "Value",
                    theme.font_library.font_noto_sans.font_header.clone(),
                    theme.foreground,
                );

                // Previous value column header.
                let previous_value_label_position = pos2(previous_value_splitter_position_x + text_left_padding, header_rectangle.center().y);

                user_interface.painter().text(
                    previous_value_label_position,
                    Align2::LEFT_CENTER,
                    "Previous Value",
                    theme.font_library.font_noto_sans.font_header.clone(),
                    theme.foreground,
                );

                // Assume all false.
                let mut selection_freeze_checkstate = CheckState::False;

                // Result entries.
                ScrollArea::vertical()
                    .id_salt("element_scanner_result_entries")
                    .max_height(content_height)
                    .auto_shrink([false, false])
                    .show(&mut user_interface, |user_interface| {
                        let element_scanner_results_view_data = match self
                            .element_scanner_results_view_data
                            .read("Element scanner results view element scanner results view data")
                        {
                            Some(element_scanner_results_view_data) => element_scanner_results_view_data,
                            None => return,
                        };
                        let element_scanner_view_data = match self
                            .element_scanner_view_data
                            .read("Element scanner results view element scanner view data")
                        {
                            Some(element_scanner_view_data) => element_scanner_view_data,
                            None => return,
                        };

                        user_interface.spacing_mut().menu_margin = Margin::ZERO;
                        user_interface.spacing_mut().window_margin = Margin::ZERO;
                        user_interface.spacing_mut().menu_spacing = 0.0;
                        user_interface.spacing_mut().item_spacing = Vec2::ZERO;

                        if element_scanner_view_data.view_state == ElementScannerViewState::ScanInProgress
                            || element_scanner_results_view_data.is_querying_scan_results
                        {
                            user_interface.allocate_ui_with_layout(
                                vec2(user_interface.available_width(), 32.0),
                                Layout::centered_and_justified(Direction::LeftToRight),
                                |user_interface| {
                                    user_interface.add(Spinner::new().color(theme.foreground));
                                },
                            );

                            return;
                        }

                        let input = user_interface.input(|input| input.clone());

                        if input.modifiers.ctrl && input.key_pressed(eframe::egui::Key::A) {
                            ElementScannerResultsViewData::select_all(self.element_scanner_results_view_data.clone());
                            return;
                        }

                        if input.modifiers.ctrl && input.key_pressed(eframe::egui::Key::C) {
                            should_copy_selected_rows = true;
                        }

                        if !element_scanner_results_view_data.show_change_value_dialog {
                            if let Some(paste_text) = input.events.iter().find_map(|event| match event {
                                eframe::egui::Event::Paste(text) => Some(text.clone()),
                                _ => None,
                            }) {
                                let mut wanted_modules = HashSet::new();
                                let mut wanted_addresses = HashSet::new();

                                for line in paste_text.lines() {
                                    let token = line.trim();
                                    if token.is_empty() {
                                        continue;
                                    }

                                    if let Some((module, offset)) = token.split_once('+') {
                                        let module = module.trim().to_ascii_lowercase();
                                        let offset = offset.trim().trim_start_matches("0x").trim_start_matches("0X");
                                        if let Ok(value) = u64::from_str_radix(offset, 16) {
                                            wanted_modules.insert((module, value));
                                        }
                                        continue;
                                    }

                                    let cleaned = token.trim().trim_start_matches("0x").trim_start_matches("0X");
                                    let value = u64::from_str_radix(cleaned, 16).or_else(|_| cleaned.parse::<u64>());
                                    if let Ok(value) = value {
                                        wanted_addresses.insert(value);
                                    }
                                }

                                let mut matched_indices: Vec<i32> = Vec::new();
                                for (index, scan_result) in element_scanner_results_view_data.current_scan_results.iter().enumerate() {
                                    let local_index = index as i32;
                                    if scan_result.is_module() {
                                        let module = scan_result.get_module().to_ascii_lowercase();
                                        let offset = scan_result.get_module_offset();
                                        if wanted_modules.contains(&(module, offset)) {
                                            matched_indices.push(local_index);
                                        }
                                    } else if wanted_addresses.contains(&scan_result.get_address()) {
                                        matched_indices.push(local_index);
                                    }
                                }

                                if let (Some(min), Some(max)) = (matched_indices.iter().min().copied(), matched_indices.iter().max().copied()) {
                                    paste_selection_range = Some((min, max));
                                }
                                return;
                            }
                        }

                        user_interface.with_layout(Layout::top_down(Align::Min), |user_interface| {
                            // Draw rows, capture min/max Y.
                            for index in 0..element_scanner_results_view_data.current_scan_results.len() {
                                let is_selected = {
                                    match (
                                        element_scanner_results_view_data.selection_index_start,
                                        element_scanner_results_view_data.selection_index_end,
                                    ) {
                                        (Some(start), Some(end)) => {
                                            let (min_index, max_index) = if start <= end { (start, end) } else { (end, start) };
                                            index as i32 >= min_index && index as i32 <= max_index
                                        }
                                        (Some(start), None) => index as i32 == start,
                                        (None, Some(end)) => index as i32 == end,
                                        (None, None) => false,
                                    }
                                };

                                let scan_result = &element_scanner_results_view_data.current_scan_results[index];

                                // Update the cumulative check state based on whether this scan result is frozen.
                                if is_selected {
                                    match selection_freeze_checkstate {
                                        CheckState::False => {
                                            if scan_result.get_is_frozen() {
                                                selection_freeze_checkstate = CheckState::True;
                                            }
                                        }
                                        CheckState::True => {
                                            if !scan_result.get_is_frozen() {
                                                selection_freeze_checkstate = CheckState::Mixed;
                                            }
                                        }
                                        CheckState::Mixed => {}
                                    }
                                }

                                let entry_widget = ElementScannerResultEntryView::new(
                                    self.app_context.clone(),
                                    &scan_result,
                                    element_scanner_view_data.active_display_format,
                                    index,
                                    is_selected,
                                    &mut element_sanner_result_frame_action,
                                    faux_address_splitter_position_x,
                                    value_splitter_position_x,
                                    previous_value_splitter_position_x,
                                );
                                let row_response = user_interface.add(entry_widget);

                                if rows_min_y.is_none() {
                                    rows_min_y = Some(row_response.rect.min.y);
                                }

                                rows_max_y = Some(row_response.rect.max.y);

                                // Primary click should immediately select the row, matching CE-style behavior.
                                // (Selection mutation itself is deferred through frame action.)
                                if row_response.clicked() || row_response.clicked_by(eframe::egui::PointerButton::Primary) {
                                    element_sanner_result_frame_action = ElementScannerResultFrameAction::SetSelectionStart(Some(index as i32));
                                }

                                // NOTE: `Sense::click()` does not reliably surface `secondary_clicked()` across
                                // all widgets/versions of egui, but we still want CE-like behavior where a
                                // right-click selects the row before opening the context menu.
                                let secondary_clicked = row_response.secondary_clicked()
                                    || row_response.clicked_by(eframe::egui::PointerButton::Secondary)
                                    || (row_response.hovered()
                                        && user_interface.input(|input| {
                                            input.pointer.button_clicked(eframe::egui::PointerButton::Secondary)
                                                || input.pointer.button_pressed(eframe::egui::PointerButton::Secondary)
                                                || input.pointer.button_released(eframe::egui::PointerButton::Secondary)
                                        }));

                                if secondary_clicked {
                                    element_sanner_result_frame_action = ElementScannerResultFrameAction::SetSelectionStart(Some(index as i32));
                                }

                                if row_response.double_clicked() {
                                    element_sanner_result_frame_action = ElementScannerResultFrameAction::SetSelectionStart(Some(index as i32));
                                    should_open_change_value_dialog = true;
                                }

                                row_response.context_menu(|ui| {
                                    if ui.button("Select all").clicked() {
                                        should_select_all = true;
                                        ui.close();
                                    }

                                    if ui.button("Copy address").clicked() {
                                        let address = scan_result.get_address();
                                        let address_string = if scan_result.is_module() {
                                            format!("{}+{:X}", scan_result.get_module(), scan_result.get_module_offset())
                                        } else if address <= u32::MAX as u64 {
                                            format!("{:08X}", address)
                                        } else {
                                            format!("{:016X}", address)
                                        };
                                        copy_text = Some(address_string);
                                        ui.close();
                                    }

                                    if ui.button("Copy value").clicked() {
                                        let current_value_string = scan_result
                                            .get_recently_read_display_value(element_scanner_view_data.active_display_format)
                                            .or_else(|| scan_result.get_current_display_value(element_scanner_view_data.active_display_format))
                                            .map(|value| value.get_anonymous_value_string().to_string())
                                            .unwrap_or_else(|| "??".to_string());
                                        copy_text = Some(current_value_string);
                                        ui.close();
                                    }

                                    if ui.button("Copy previous value").clicked() {
                                        let previous_value_string = scan_result
                                            .get_previous_display_value(element_scanner_view_data.active_display_format)
                                            .map(|value| value.get_anonymous_value_string().to_string())
                                            .unwrap_or_else(|| "??".to_string());
                                        copy_text = Some(previous_value_string);
                                        ui.close();
                                    }

                                    if ui.button("Copy selected").clicked() {
                                        should_copy_selected_rows = true;
                                        ui.close();
                                    }

                                    if ui.button("Copy selected addresses").clicked() {
                                        should_copy_selected_addresses = true;
                                        ui.close();
                                    }

                                    ui.separator();

                                    if ui.button("Change value of selected addresses").clicked() {
                                        should_open_change_value_dialog = true;
                                        ui.close();
                                    }
                                    if ui.button("Freeze selected addresses").clicked() {
                                        element_sanner_result_frame_action = ElementScannerResultFrameAction::ToggleFreezeSelection(true);
                                        ui.close();
                                    }
                                    if ui.button("Unfreeze selected addresses").clicked() {
                                        element_sanner_result_frame_action = ElementScannerResultFrameAction::ToggleFreezeSelection(false);
                                        ui.close();
                                    }
                                    if ui.button("Add selected addresses to the addresslist").clicked() {
                                        element_sanner_result_frame_action = ElementScannerResultFrameAction::AddSelection;
                                        ui.close();
                                    }
                                    if ui.button("Delete selected addresses").clicked() {
                                        element_sanner_result_frame_action = ElementScannerResultFrameAction::DeleteSelection;
                                        ui.close();
                                    }
                                    if ui.button("Browse this memory region").clicked() {
                                        browse_memory_address = Some(scan_result.get_address());
                                        ui.close();
                                    }
                                    if ui.button("Disassemble this memory region").clicked() {
                                        disassemble_address = Some(scan_result.get_address());
                                        ui.close();
                                    }
                                    if ui.button("Pointer scan this address").clicked() {
                                        pointer_scan_address = Some(scan_result.get_address());
                                        ui.close();
                                    }
                                });
                            }
                        });
                    });

                // Draw the footer.
                user_interface.add(ElementScannerResultsActionBarView::new(
                    self.app_context.clone(),
                    selection_freeze_checkstate,
                    &mut element_sanner_result_frame_action,
                    faux_address_splitter_position_x,
                    value_splitter_position_x,
                    previous_value_splitter_position_x,
                ));

                // Faux address splitter.
                user_interface
                    .painter()
                    .rect_filled(faux_address_splitter_rectangle, 0.0, theme.background_control);

                // Value splitter.
                let value_splitter_response =
                    allocate_resize_bar(&mut user_interface, value_splitter_rectangle, "value_splitter").on_hover_cursor(CursorIcon::ResizeHorizontal);

                if value_splitter_response.dragged() {
                    let drag_delta = value_splitter_response.drag_delta();
                    let mut new_value_splitter_position_x = value_splitter_position_x + drag_delta.x;

                    let minimum_value_splitter_position_x = content_min_x + MINIMUM_COLUMN_PIXEL_WIDTH;
                    let maximum_value_splitter_position_x = previous_value_splitter_position_x - MINIMUM_SPLITTER_PIXEL_GAP;

                    new_value_splitter_position_x = new_value_splitter_position_x.clamp(minimum_value_splitter_position_x, maximum_value_splitter_position_x);

                    let bounded_value_splitter_ratio = (new_value_splitter_position_x - content_min_x) / content_width;

                    new_value_splitter_ratio = Some(bounded_value_splitter_ratio);
                }

                // Previous value splitter.
                let previous_value_splitter_response = allocate_resize_bar(&mut user_interface, previous_value_splitter_rectangle, "previous_value_splitter")
                    .on_hover_cursor(CursorIcon::ResizeHorizontal);

                if previous_value_splitter_response.dragged() {
                    let drag_delta = previous_value_splitter_response.drag_delta();
                    let mut new_previous_value_splitter_position_x = previous_value_splitter_position_x + drag_delta.x;

                    let minimum_previous_value_splitter_position_x = value_splitter_position_x + MINIMUM_SPLITTER_PIXEL_GAP;
                    let maximum_previous_value_splitter_position_x = content_min_x + content_width - MINIMUM_COLUMN_PIXEL_WIDTH;

                    new_previous_value_splitter_position_x =
                        new_previous_value_splitter_position_x.clamp(minimum_previous_value_splitter_position_x, maximum_previous_value_splitter_position_x);

                    let bounded_previous_value_splitter_ratio = (new_previous_value_splitter_position_x - content_min_x) / content_width;

                    new_previous_value_splitter_ratio = Some(bounded_previous_value_splitter_ratio);
                }
            })
            .response;

        if should_select_all {
            ElementScannerResultsViewData::select_all(self.element_scanner_results_view_data.clone());
        }

        if let Some(text) = copy_text.take() {
            if !text.is_empty() {
                user_interface.ctx().copy_text(text);
            }
        } else if should_copy_selected_rows {
            let active_display_format = self
                .element_scanner_view_data
                .read("Element scanner copy selected rows display format")
                .map(|view_data| view_data.active_display_format)
                .unwrap_or_default();

            let text = ElementScannerResultsViewData::copy_selected_rows_tsv(self.element_scanner_results_view_data.clone(), active_display_format);
            if !text.is_empty() {
                user_interface.ctx().copy_text(text);
            }
        } else if should_copy_selected_addresses {
            let text = ElementScannerResultsViewData::copy_selected_addresses(self.element_scanner_results_view_data.clone());
            if !text.is_empty() {
                user_interface.ctx().copy_text(text);
            }
        }

        if let Some((start, end)) = paste_selection_range.take() {
            let applied = ElementScannerResultsViewData::set_scan_result_selection_start(
                self.element_scanner_results_view_data.clone(),
                self.struct_viewer_view_data.clone(),
                Some(start),
            );
            if !applied {
                if let Some(mut view_data) = self.element_scanner_results_view_data.try_write("Element scanner stash pending paste selection") {
                    view_data.pending_frame_action = ElementScannerResultFrameAction::SetSelectionStart(Some(start));
                }
                user_interface.ctx().request_repaint();
                return response;
            }
            if end != start {
                let applied = ElementScannerResultsViewData::set_scan_result_selection_end(
                    self.element_scanner_results_view_data.clone(),
                    self.struct_viewer_view_data.clone(),
                    Some(end),
                );
                if !applied {
                    if let Some(mut view_data) = self.element_scanner_results_view_data.try_write("Element scanner stash pending paste selection end") {
                        view_data.pending_frame_action = ElementScannerResultFrameAction::SetSelectionEnd(Some(end));
                    }
                    user_interface.ctx().request_repaint();
                    return response;
                }
            }
        }

        if new_value_splitter_ratio.is_some() || new_previous_value_splitter_ratio.is_some() {
            if let Some(mut element_scanner_results_view_data) = self
                .element_scanner_results_view_data
                .write("Element scanner results view")
            {
                if let Some(new_value_splitter_ratio) = new_value_splitter_ratio {
                    element_scanner_results_view_data.value_splitter_ratio = new_value_splitter_ratio;
                }

                if let Some(new_previous_value_splitter_ratio) = new_previous_value_splitter_ratio {
                    element_scanner_results_view_data.previous_value_splitter_ratio = new_previous_value_splitter_ratio;
                }
            }
        }

        if element_sanner_result_frame_action != ElementScannerResultFrameAction::None {
            ui_trace::trace(format!("results_view.apply_action {:?}", element_sanner_result_frame_action));
            match element_sanner_result_frame_action {
                ElementScannerResultFrameAction::None => {}
                ElementScannerResultFrameAction::SetSelectionStart(index) => {
                    let applied = ElementScannerResultsViewData::set_scan_result_selection_start(
                        self.element_scanner_results_view_data.clone(),
                        self.struct_viewer_view_data.clone(),
                        index,
                    );
                    if !applied {
                        ui_trace::trace(format!("results_view.stash_pending SetSelectionStart({:?})", index));
                        if let Some(mut view_data) = self.element_scanner_results_view_data.try_write("Element scanner stash pending selection start") {
                            view_data.pending_frame_action = ElementScannerResultFrameAction::SetSelectionStart(index);
                        }
                        user_interface.ctx().request_repaint();
                        return response;
                    }
                }
                ElementScannerResultFrameAction::SetSelectionEnd(index) => {
                    let applied = ElementScannerResultsViewData::set_scan_result_selection_end(
                        self.element_scanner_results_view_data.clone(),
                        self.struct_viewer_view_data.clone(),
                        index,
                    );
                    if !applied {
                        ui_trace::trace(format!("results_view.stash_pending SetSelectionEnd({:?})", index));
                        if let Some(mut view_data) = self.element_scanner_results_view_data.try_write("Element scanner stash pending selection end") {
                            view_data.pending_frame_action = ElementScannerResultFrameAction::SetSelectionEnd(index);
                        }
                        user_interface.ctx().request_repaint();
                        return response;
                    }
                }
                ElementScannerResultFrameAction::FreezeIndex(index, is_frozen) => {
                    ElementScannerResultsViewData::set_scan_result_frozen(
                        self.element_scanner_results_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                        index,
                        is_frozen,
                    );
                }
                ElementScannerResultFrameAction::ToggleFreezeSelection(is_frozen) => {
                    ElementScannerResultsViewData::toggle_selected_scan_results_frozen(
                        self.element_scanner_results_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                        is_frozen,
                    );
                }
                ElementScannerResultFrameAction::AddSelection => {
                    ElementScannerResultsViewData::add_scan_results_to_project(
                        self.element_scanner_results_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                    );
                }
                ElementScannerResultFrameAction::DeleteSelection => {
                    ElementScannerResultsViewData::delete_selected_scan_results(
                        self.element_scanner_results_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                    );
                }
                ElementScannerResultFrameAction::CommitValueToSelection(edit_value) => {
                    ElementScannerResultsViewData::set_selected_scan_results_value(
                        self.element_scanner_results_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                        ScanResult::PROPERTY_NAME_VALUE,
                        edit_value,
                    );
                }
            }

            // Action applied; clear any pending retry.
            if let Some(mut view_data) = self.element_scanner_results_view_data.try_write("Element scanner clear pending frame action") {
                view_data.pending_frame_action = ElementScannerResultFrameAction::None;
            }
            ui_trace::trace("results_view.clear_pending_action");
        }

        if should_open_change_value_dialog {
            let seed_value = match self
                .element_scanner_results_view_data
                .read("Element scanner change value seed")
            {
                Some(view_data) => view_data.current_display_string.clone(),
                None => squalr_engine_api::structures::data_values::anonymous_value_string::AnonymousValueString::new(
                    String::new(),
                    squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat::Decimal,
                    squalr_engine_api::structures::data_values::container_type::ContainerType::None,
                ),
            };

            ElementScannerResultsViewData::show_change_value_dialog(self.element_scanner_results_view_data.clone(), seed_value);
        }

        if let Some(address) = browse_memory_address {
            MemoryViewerViewData::set_target_address(
                self.memory_viewer_view_data.clone(),
                self.app_context.engine_unprivileged_state.clone(),
                address,
            );
            MemoryViewerViewData::set_popout_open(self.memory_viewer_view_data.clone(), true);
        }

        if let Some(address) = disassemble_address {
            DisassemblerViewData::set_target_address(
                self.disassembler_view_data.clone(),
                self.app_context.engine_unprivileged_state.clone(),
                address,
            );

            if let Ok(mut docking_manager) = self.app_context.docking_manager.write() {
                docking_manager.set_window_visible(crate::views::disassembler::disassembler_view::DisassemblerView::WINDOW_ID, true);
            }
        }

        if let Some(address) = pointer_scan_address {
            if let Some(mut view_data) = self
                .pointer_scanner_view_data
                .write("Element scanner pointer scan address")
            {
                view_data.target_address = format!("{:X}", address);
            }

            if let Ok(mut docking_manager) = self.app_context.docking_manager.write() {
                docking_manager.set_window_visible(crate::views::pointer_scanner::pointer_scanner_view::PointerScannerView::WINDOW_ID, true);
            }
        }

        let mut should_commit_change_value = None;
        let mut should_close_change_dialog = false;
        let data_type_for_dialog = match self
            .element_scanner_view_data
            .read("Element scanner change value data type")
        {
            Some(view_data) => view_data.selected_data_type.clone(),
            None => return response,
        };

        let show_change_value_dialog = match self
            .element_scanner_results_view_data
            .read("Element scanner change value dialog read")
        {
            Some(view_data) => view_data.show_change_value_dialog,
            None => false,
        };

        if show_change_value_dialog {
            let app_context = self.app_context.clone();
            Window::new("Change value")
                .collapsible(false)
                .resizable(false)
                .show(user_interface.ctx(), |ui| {
                    if let Some(mut view_data) = self
                        .element_scanner_results_view_data
                        .write("Element scanner change value dialog write")
                    {
                        ui.add(crate::ui::widgets::controls::data_value_box::data_value_box_view::DataValueBoxView::new(
                            app_context.clone(),
                            &mut view_data.change_value_string,
                            &data_type_for_dialog,
                            false,
                            true,
                            "New value",
                            "element_scanner_change_value",
                        ));

                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                should_close_change_dialog = true;
                            }
                            if ui.button("OK").clicked() {
                                should_commit_change_value = Some(view_data.change_value_string.clone());
                                should_close_change_dialog = true;
                            }
                        });
                    }
                });
        }

        if should_close_change_dialog {
            ElementScannerResultsViewData::hide_change_value_dialog(self.element_scanner_results_view_data.clone());
        }

        if let Some(change_value) = should_commit_change_value {
            ElementScannerResultsViewData::set_selected_scan_results_value(
                self.element_scanner_results_view_data.clone(),
                self.app_context.engine_unprivileged_state.clone(),
                ScanResult::PROPERTY_NAME_VALUE,
                change_value,
            );
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::ElementScannerResultsView;
    use crate::app_context::AppContext;
    use crate::models::docking::docking_manager::DockingManager;
    use crate::models::docking::hierarchy::dock_node::DockNode;
    use crate::ui::theme::Theme;
    use crate::views::disassembler::view_data::disassembler_view_data::DisassemblerViewData;
    use crate::views::element_scanner::results::view_data::element_scanner_result_frame_action::ElementScannerResultFrameAction;
    use crate::views::element_scanner::results::view_data::element_scanner_results_view_data::ElementScannerResultsViewData;
    use crate::views::element_scanner::scanner::view_data::element_scanner_view_data::ElementScannerViewData;
    use crate::views::memory_viewer::view_data::memory_viewer_view_data::MemoryViewerViewData;
    use crate::views::pointer_scanner::view_data::pointer_scanner_view_data::PointerScannerViewData;
    use crate::views::struct_viewer::view_data::struct_viewer_view_data::StructViewerViewData;
    use crossbeam_channel::unbounded;
    use eframe::egui;
    use squalr_engine_api::commands::privileged_command::PrivilegedCommand;
    use squalr_engine_api::commands::privileged_command_response::PrivilegedCommandResponse;
    use squalr_engine_api::commands::unprivileged_command::UnprivilegedCommand;
    use squalr_engine_api::commands::unprivileged_command_response::UnprivilegedCommandResponse;
    use squalr_engine_api::engine::engine_api_unprivileged_bindings::EngineApiUnprivilegedBindings;
    use squalr_engine_api::engine::engine_unprivileged_state::EngineUnprivilegedState;
    use squalr_engine_api::events::engine_event::EngineEvent;
    use squalr_engine_api::structures::data_types::built_in_types::string::utf8::data_type_string_utf8::DataTypeStringUtf8;
    use squalr_engine_api::structures::data_types::data_type_ref::DataTypeRef;
    use squalr_engine_api::structures::data_values::anonymous_value_string::AnonymousValueString;
    use squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;
    use squalr_engine_api::structures::scan_results::scan_result::ScanResult;
    use squalr_engine_api::structures::scan_results::scan_result_ref::ScanResultRef;
    use squalr_engine_api::structures::scan_results::scan_result_valued::ScanResultValued;
    use std::sync::{Arc, Mutex, OnceLock, RwLock};
    use std::time::Duration;

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
            Err("Mock bindings: unprivileged commands not supported in this test".to_string())
        }

        fn subscribe_to_engine_events(&self) -> Result<crossbeam_channel::Receiver<EngineEvent>, String> {
            let (_sender, receiver) = unbounded();
            Ok(receiver)
        }
    }

    fn make_string_scan_result(
        address: u64,
        value: &str,
    ) -> ScanResult {
        let data_type_ref = DataTypeRef::new(DataTypeStringUtf8::get_data_type_id());
        let display_value = AnonymousValueString::new(value.to_string(), AnonymousValueStringFormat::String, Default::default());
        let current_value = Some(DataTypeStringUtf8::get_value_from_primitive_array(value.as_bytes().to_vec()));
        let valued = ScanResultValued::new(
            address,
            data_type_ref,
            String::new(),
            current_value,
            vec![display_value.clone()],
            None,
            vec![],
            ScanResultRef::new(0),
        );

        ScanResult::new(valued, String::new(), 0, None, vec![display_value], false)
    }

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("failed to lock element scanner results test mutex")
    }

    #[test]
    fn pending_selection_action_does_not_panic_or_hang() {
        let _guard = test_guard();
        let (done_tx, done_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let ctx = egui::Context::default();
            let theme = Arc::new(Theme::new(&ctx));
            let docking_root = DockNode::Window {
                window_identifier: "dummy".to_string(),
                is_visible: true,
            };
            let docking_manager = Arc::new(std::sync::RwLock::new(DockingManager::new(docking_root)));
            let engine_state = EngineUnprivilegedState::new(Arc::new(RwLock::new(MockUnprivilegedBindings)));
            let app_context = Arc::new(AppContext::new(ctx.clone(), theme, docking_manager, engine_state));

            // Register dependencies required by ElementScannerResultsView.
            app_context.dependency_container.register(ElementScannerViewData::new());
            app_context.dependency_container.register(StructViewerViewData::new());
            app_context.dependency_container.register(MemoryViewerViewData::new());
            app_context.dependency_container.register(DisassemblerViewData::new());
            app_context.dependency_container.register(PointerScannerViewData::new());

            let mut results = ElementScannerResultsViewData::new();
            results.current_scan_results = Arc::new(vec![make_string_scan_result(0x21BD0034, "note")]);
            results.result_count = 1;
            results.pending_frame_action = ElementScannerResultFrameAction::SetSelectionStart(Some(0));
            app_context.dependency_container.register(results);

            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0)));
            ctx.begin_frame(input);
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.add(ElementScannerResultsView::new(app_context.clone()));
            });
            let _ = ctx.end_frame();

            let _ = done_tx.send(());
        });

        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("UI frame with pending selection action hung or panicked");
    }

    fn run_frame_with_input(
        ctx: &egui::Context,
        app_context: Arc<AppContext>,
        mut input: egui::RawInput,
    ) -> egui::FullOutput {
        input.screen_rect = input.screen_rect.or(Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )));

        ctx.begin_frame(input);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(ElementScannerResultsView::new(app_context));
        });
        ctx.end_frame()
    }

    fn collect_texts(output: &egui::FullOutput) -> Vec<String> {
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text_shape) = &clipped.shape {
                let text = text_shape.galley.job.text.clone();
                if !texts.contains(&text) {
                    texts.push(text);
                }
            }
        }
        texts
    }

    fn find_text_center(output: &egui::FullOutput, needle: &str) -> Option<egui::Pos2> {
        for clipped in &output.shapes {
            if let egui::Shape::Text(text_shape) = &clipped.shape {
                if text_shape.galley.job.text == needle {
                    return Some(text_shape.visual_bounding_rect().center());
                }
            }
        }
        None
    }

    #[test]
    fn clicking_result_row_primary_does_not_crash_and_selects() {
        let _guard = test_guard();
        let ctx = egui::Context::default();
        let theme = Arc::new(Theme::new(&ctx));
        let docking_root = DockNode::Window {
            window_identifier: "dummy".to_string(),
            is_visible: true,
        };
        let docking_manager = Arc::new(std::sync::RwLock::new(DockingManager::new(docking_root)));
        let engine_state = EngineUnprivilegedState::new(Arc::new(RwLock::new(MockUnprivilegedBindings)));
        let app_context = Arc::new(AppContext::new(ctx.clone(), theme, docking_manager, engine_state));

        app_context.dependency_container.register(ElementScannerViewData::new());
        app_context.dependency_container.register(StructViewerViewData::new());
        app_context.dependency_container.register(MemoryViewerViewData::new());
        app_context.dependency_container.register(DisassemblerViewData::new());
        app_context.dependency_container.register(PointerScannerViewData::new());

        let mut results = ElementScannerResultsViewData::new();
        results.current_scan_results = Arc::new(vec![make_string_scan_result(0x21BD0034, "note")]);
        results.result_count = 1;
        app_context.dependency_container.register(results);

        // Frame 1: layout without any interaction.
        let out1 = run_frame_with_input(&ctx, app_context.clone(), egui::RawInput::default());

        // Frame 2: click somewhere that should land inside the first row.
        let click_pos = find_text_center(&out1, "21BD0034").unwrap_or_else(|| {
            panic!("failed to locate row text for click; texts={:?}", collect_texts(&out1));
        });
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::PointerMoved(click_pos));
        input.events.push(egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        input.events.push(egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = run_frame_with_input(&ctx, app_context.clone(), input);

        // Frame 3: apply pending frame action (selection updates happen next frame).
        let _ = run_frame_with_input(&ctx, app_context.clone(), egui::RawInput::default());

        let dep = app_context.dependency_container.get_dependency::<ElementScannerResultsViewData>();
        let data = dep
            .read("Assert selection after primary click")
            .expect("read results view data");
        assert_eq!(data.selection_index_start, Some(0), "primary click did not select the first row");
    }

    #[test]
    fn clicking_result_row_secondary_does_not_crash_and_selects() {
        let _guard = test_guard();
        let ctx = egui::Context::default();
        let theme = Arc::new(Theme::new(&ctx));
        let docking_root = DockNode::Window {
            window_identifier: "dummy".to_string(),
            is_visible: true,
        };
        let docking_manager = Arc::new(std::sync::RwLock::new(DockingManager::new(docking_root)));
        let engine_state = EngineUnprivilegedState::new(Arc::new(RwLock::new(MockUnprivilegedBindings)));
        let app_context = Arc::new(AppContext::new(ctx.clone(), theme, docking_manager, engine_state));

        app_context.dependency_container.register(ElementScannerViewData::new());
        app_context.dependency_container.register(StructViewerViewData::new());
        app_context.dependency_container.register(MemoryViewerViewData::new());
        app_context.dependency_container.register(DisassemblerViewData::new());
        app_context.dependency_container.register(PointerScannerViewData::new());

        let mut results = ElementScannerResultsViewData::new();
        results.current_scan_results = Arc::new(vec![make_string_scan_result(0x21BD0034, "note")]);
        results.result_count = 1;
        app_context.dependency_container.register(results);

        // Frame 1: layout without any interaction.
        let out1 = run_frame_with_input(&ctx, app_context.clone(), egui::RawInput::default());

        // Frame 2: secondary-click somewhere that should land inside the first row.
        let click_pos = find_text_center(&out1, "21BD0034").unwrap_or_else(|| {
            panic!("failed to locate row text for click; texts={:?}", collect_texts(&out1));
        });
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::PointerMoved(click_pos));
        input.events.push(egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Secondary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        input.events.push(egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = run_frame_with_input(&ctx, app_context.clone(), input);

        // Frame 3: apply pending frame action (selection updates happen next frame).
        let _ = run_frame_with_input(&ctx, app_context.clone(), egui::RawInput::default());

        let dep = app_context.dependency_container.get_dependency::<ElementScannerResultsViewData>();
        let data = dep
            .read("Assert selection after secondary click")
            .expect("read results view data");
        assert_eq!(data.selection_index_start, Some(0), "secondary click did not select the first row");
    }
}
