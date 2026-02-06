use crate::app_context::AppContext;
use crate::ui::draw::icon_draw::IconDraw;
use crate::ui::widgets::controls::button::Button;
use crate::ui::widgets::controls::checkbox::Checkbox;
use crate::ui::widgets::controls::data_type_selector::data_type_selector_view::DataTypeSelectorView;
use crate::views::pointer_scanner::view_data::pointer_scanner_view_data::PointerScannerViewData;
use eframe::egui::{Align, Direction, Layout, Response, ScrollArea, Sense, Spinner, TextEdit, Ui, UiBuilder, Widget};
use epaint::{Color32, CornerRadius, Rect, Stroke, StrokeKind, pos2, vec2};
use squalr_engine_api::dependency_injection::dependency::Dependency;
use std::sync::Arc;

#[derive(Clone)]
pub struct PointerScannerView {
    app_context: Arc<AppContext>,
    pointer_scanner_view_data: Dependency<PointerScannerViewData>,
}

impl PointerScannerView {
    pub const WINDOW_ID: &'static str = "window_pointer_scanner";

    pub fn new(app_context: Arc<AppContext>) -> Self {
        let pointer_scanner_view_data = app_context
            .dependency_container
            .register(PointerScannerViewData::new());

        PointerScannerViewData::poll_results(pointer_scanner_view_data.clone(), app_context.engine_unprivileged_state.clone());

        Self {
            app_context,
            pointer_scanner_view_data,
        }
    }
}

impl Widget for PointerScannerView {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        let theme = &self.app_context.theme;

        let mut should_start_scan = false;
        let mut should_cancel_scan = false;
        let mut should_navigate_first_page = false;
        let mut should_navigate_previous_page = false;
        let mut should_navigate_next_page = false;
        let mut should_navigate_last_page = false;
        let mut pending_page_index_text: Option<String> = None;

        let response = user_interface
            .allocate_ui_with_layout(user_interface.available_size(), Layout::top_down(Align::Min), |user_interface| {
                let toolbar_height = 72.0;
                let (toolbar_rect, _) =
                    user_interface.allocate_exact_size(vec2(user_interface.available_width(), toolbar_height), Sense::hover());

                user_interface
                    .painter()
                    .rect_filled(toolbar_rect, CornerRadius::ZERO, theme.background_primary);

                let builder = UiBuilder::new().max_rect(toolbar_rect).layout(Layout::top_down(Align::Min));
                let mut toolbar_ui = user_interface.new_child(builder);

                let mut pointer_scanner_view_data = match self.pointer_scanner_view_data.write("Pointer scanner toolbar") {
                    Some(data) => data,
                    None => return,
                };

                toolbar_ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_sized(
                        vec2(160.0, 28.0),
                        TextEdit::singleline(&mut pointer_scanner_view_data.target_address)
                            .hint_text("Target address")
                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                            .text_color(theme.hexadecimal_green)
                            .background_color(theme.background_primary),
                    );

                    ui.add(DataTypeSelectorView::new(
                        self.app_context.clone(),
                        &mut pointer_scanner_view_data.pointer_data_type,
                        "pointer_scanner_data_type_selector",
                    ));

                    ui.add_sized(
                        vec2(72.0, 28.0),
                        TextEdit::singleline(&mut pointer_scanner_view_data.max_depth_text)
                            .hint_text("Depth")
                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                            .text_color(theme.foreground)
                            .background_color(theme.background_primary),
                    );

                    ui.add_sized(
                        vec2(88.0, 28.0),
                        TextEdit::singleline(&mut pointer_scanner_view_data.offset_size_text)
                            .hint_text("Max offset")
                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                            .text_color(theme.foreground)
                            .background_color(theme.background_primary),
                    );

                    if ui
                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(pointer_scanner_view_data.scan_statics))
                        .clicked()
                    {
                        pointer_scanner_view_data.scan_statics = !pointer_scanner_view_data.scan_statics;
                    }

                    ui.label("Statics");

                    if ui
                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(pointer_scanner_view_data.scan_heaps))
                        .clicked()
                    {
                        pointer_scanner_view_data.scan_heaps = !pointer_scanner_view_data.scan_heaps;
                    }

                    ui.label("Heaps");

                    if pointer_scanner_view_data.is_scanning {
                        let stop_button = ui.add_sized(
                            vec2(88.0, 28.0),
                            Button::new_from_theme(theme)
                                .background_color(Color32::TRANSPARENT)
                                .with_tooltip_text("Cancel pointer scan"),
                        );

                        if stop_button.clicked() {
                            should_cancel_scan = true;
                        }
                    } else {
                        let start_button = ui.add_sized(
                            vec2(88.0, 28.0),
                            Button::new_from_theme(theme)
                                .background_color(Color32::TRANSPARENT)
                                .with_tooltip_text("Start pointer scan"),
                        );

                        if start_button.clicked() {
                            should_start_scan = true;
                        }
                    }
                });

                toolbar_ui.add_space(4.0);

                toolbar_ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if pointer_scanner_view_data.is_scanning {
                        ui.add(Spinner::new().color(theme.foreground));
                        ui.label(format!("Progress: {:.0}%", pointer_scanner_view_data.progress * 100.0));
                    } else {
                        ui.label(&pointer_scanner_view_data.stats_string);
                    }
                });

                drop(pointer_scanner_view_data);

                user_interface.add_space(4.0);

                let footer_height = 48.0;
                let list_height = (user_interface.available_height() - footer_height).max(32.0);

                let mut selection_start: Option<i32> = None;
                let mut selection_end: Option<i32> = None;

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(list_height)
                    .show(user_interface, |user_interface| {
                        let pointer_scanner_view_data = match self.pointer_scanner_view_data.read("Pointer scanner list") {
                            Some(data) => data,
                            None => return,
                        };

                        selection_start = pointer_scanner_view_data.selection_index_start;
                        selection_end = pointer_scanner_view_data.selection_index_end;

                        let input = user_interface.input(|input| input.clone());
                        if input.modifiers.ctrl && input.key_pressed(eframe::egui::Key::A) {
                            drop(pointer_scanner_view_data);
                            PointerScannerViewData::select_all(self.pointer_scanner_view_data.clone());
                            return;
                        }

                        if input.modifiers.ctrl && input.key_pressed(eframe::egui::Key::C) {
                            let text = PointerScannerViewData::copy_selected_results(self.pointer_scanner_view_data.clone());
                            if !text.is_empty() {
                                user_interface.ctx().copy_text(text);
                            }
                        }

                        if !pointer_scanner_view_data.is_scanning && pointer_scanner_view_data.result_count == 0 {
                            user_interface.allocate_ui_with_layout(
                                vec2(user_interface.available_width(), 32.0),
                                Layout::centered_and_justified(Direction::LeftToRight),
                                |ui| {
                                    ui.label("No pointer scan results yet.");
                                },
                            );
                            return;
                        }

                        for (index, result) in pointer_scanner_view_data.current_results.iter().enumerate() {
                            let is_selected = match (selection_start, selection_end) {
                                (Some(start), Some(end)) => {
                                    let (min_index, max_index) = if start <= end { (start, end) } else { (end, start) };
                                    index as i32 >= min_index && index as i32 <= max_index
                                }
                                (Some(start), None) => index as i32 == start,
                                (None, Some(end)) => index as i32 == end,
                                (None, None) => false,
                            };

                            let row_height = 28.0;
                            let (row_rect, row_response) =
                                user_interface.allocate_exact_size(vec2(user_interface.available_width(), row_height), Sense::click());

                            if is_selected {
                                user_interface
                                    .painter()
                                    .rect_filled(row_rect, CornerRadius::ZERO, theme.selected_background);
                            }

                            if row_response.clicked() {
                                if input.modifiers.shift {
                                    selection_end = Some(index as i32);
                                } else {
                                    selection_start = Some(index as i32);
                                    selection_end = None;
                                }
                            }

                            let base = if result.is_module() {
                                format!("{}+{:X}", result.get_module_name(), result.get_module_offset())
                            } else {
                                format!("{:016X}", result.get_base_address())
                            };

                            let offsets = result
                                .get_offsets()
                                .iter()
                                .map(|offset| format!("{:X}", offset))
                                .collect::<Vec<_>>()
                                .join(", ");

                            let base_pos = pos2(row_rect.min.x + 8.0, row_rect.center().y);
                            let offsets_pos = pos2(row_rect.min.x + 220.0, row_rect.center().y);

                            user_interface.painter().text(
                                base_pos,
                                eframe::egui::Align2::LEFT_CENTER,
                                base,
                                theme.font_library.font_ubuntu_mono_bold.font_normal.clone(),
                                theme.hexadecimal_green,
                            );

                            user_interface.painter().text(
                                offsets_pos,
                                eframe::egui::Align2::LEFT_CENTER,
                                format!("[{}]", offsets),
                                theme.font_library.font_ubuntu_mono_bold.font_normal.clone(),
                                theme.foreground,
                            );
                        }
                    });

                if let Some(mut view_data) = self.pointer_scanner_view_data.write("Pointer scanner list selection update") {
                    view_data.selection_index_start = selection_start;
                    view_data.selection_index_end = selection_end;
                }

                user_interface.add_space(4.0);

                let (footer_rect, _) =
                    user_interface.allocate_exact_size(vec2(user_interface.available_width(), footer_height), Sense::hover());

                user_interface
                    .painter()
                    .rect_filled(footer_rect, CornerRadius::ZERO, theme.background_primary);

                let mut footer_ui = user_interface.new_child(
                    UiBuilder::new()
                        .max_rect(footer_rect)
                        .layout(Layout::left_to_right(Align::Center)),
                );

                let (current_page_index, last_page_index, stats_string) = match self.pointer_scanner_view_data.read("Pointer scanner footer read") {
                    Some(view_data) => (
                        view_data.current_page_index,
                        view_data.last_page_index,
                        view_data.stats_string.clone(),
                    ),
                    None => (0, 0, String::new()),
                };

                let button_size = vec2(36.0, 28.0);
                let y_center = footer_rect.center().y - button_size.y * 0.5;

                let first_rect = Rect::from_min_size(pos2(footer_rect.min.x + 8.0, y_center), button_size);
                let first_button = footer_ui.put(
                    first_rect,
                    Button::new_from_theme(theme)
                        .background_color(Color32::TRANSPARENT)
                        .with_tooltip_text("First page"),
                );
                IconDraw::draw(&footer_ui, first_button.rect, &theme.icon_library.icon_handle_navigation_left_arrows);
                if first_button.clicked() {
                    should_navigate_first_page = true;
                }

                let prev_rect = Rect::from_min_size(pos2(first_rect.max.x, y_center), button_size);
                let prev_button = footer_ui.put(
                    prev_rect,
                    Button::new_from_theme(theme)
                        .background_color(Color32::TRANSPARENT)
                        .with_tooltip_text("Previous page"),
                );
                IconDraw::draw(&footer_ui, prev_button.rect, &theme.icon_library.icon_handle_navigation_left_arrow);
                if prev_button.clicked() {
                    should_navigate_previous_page = true;
                }

                let mut page_text = (current_page_index + 1).to_string();
                let page_rect = Rect::from_min_size(pos2(prev_rect.max.x + 8.0, y_center + 2.0), vec2(96.0, 24.0));
                let page_edit = footer_ui.put(
                    page_rect,
                    TextEdit::singleline(&mut page_text)
                        .horizontal_align(Align::Center)
                        .font(theme.font_library.font_noto_sans.font_normal.clone())
                        .background_color(theme.background_primary)
                        .text_color(theme.foreground),
                );
                footer_ui.painter().rect_stroke(
                    page_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, theme.submenu_border),
                    StrokeKind::Inside,
                );
                if page_edit.changed() {
                    pending_page_index_text = Some(page_text);
                }

                let next_rect = Rect::from_min_size(pos2(page_rect.max.x + 8.0, y_center), button_size);
                let next_button = footer_ui.put(
                    next_rect,
                    Button::new_from_theme(theme)
                        .background_color(Color32::TRANSPARENT)
                        .with_tooltip_text("Next page"),
                );
                IconDraw::draw(&footer_ui, next_button.rect, &theme.icon_library.icon_handle_navigation_right_arrow);
                if next_button.clicked() {
                    should_navigate_next_page = true;
                }

                let last_rect = Rect::from_min_size(pos2(next_rect.max.x, y_center), button_size);
                let last_button = footer_ui.put(
                    last_rect,
                    Button::new_from_theme(theme)
                        .background_color(Color32::TRANSPARENT)
                        .with_tooltip_text("Last page"),
                );
                IconDraw::draw(&footer_ui, last_button.rect, &theme.icon_library.icon_handle_navigation_right_arrows);
                if last_button.clicked() {
                    should_navigate_last_page = true;
                }

                footer_ui.add_space(12.0);
                footer_ui.label(format!(
                    "{} (Page {}/{})",
                    stats_string,
                    current_page_index + 1,
                    last_page_index + 1
                ));
            })
            .response;

        if should_start_scan {
            PointerScannerViewData::start_scan(self.pointer_scanner_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        }

        if should_cancel_scan {
            PointerScannerViewData::cancel_scan(self.pointer_scanner_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        }

        if should_navigate_first_page {
            PointerScannerViewData::navigate_first_page(self.pointer_scanner_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        } else if should_navigate_previous_page {
            PointerScannerViewData::navigate_previous_page(self.pointer_scanner_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        } else if should_navigate_next_page {
            PointerScannerViewData::navigate_next_page(self.pointer_scanner_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        } else if should_navigate_last_page {
            PointerScannerViewData::navigate_last_page(self.pointer_scanner_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        } else if let Some(page_text) = pending_page_index_text {
            let page_index = page_text
                .chars()
                .take_while(|char| char.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .ok()
                .and_then(|v| v.checked_sub(1))
                .unwrap_or(0);

            PointerScannerViewData::set_page_index(
                self.pointer_scanner_view_data.clone(),
                self.app_context.engine_unprivileged_state.clone(),
                page_index,
            );
        }

        response
    }
}
