use crate::app_context::AppContext;
use crate::ui::widgets::controls::button::Button;
use crate::views::disassembler::view_data::disassembler_view_data::DisassemblerViewData;
use eframe::egui::{Align, Color32, Direction, Layout, Response, ScrollArea, Sense, Spinner, TextEdit, Ui, UiBuilder, Widget, vec2};
use epaint::{CornerRadius, Rect, Stroke, StrokeKind, pos2};
use squalr_engine_api::dependency_injection::dependency::Dependency;
use std::sync::Arc;

#[derive(Clone)]
pub struct DisassemblerView {
    app_context: Arc<AppContext>,
    disassembler_view_data: Dependency<DisassemblerViewData>,
}

impl DisassemblerView {
    pub const WINDOW_ID: &'static str = "window_disassembler";

    pub fn new(app_context: Arc<AppContext>) -> Self {
        let disassembler_view_data = DisassemblerViewData::register(&app_context);

        Self {
            app_context,
            disassembler_view_data,
        }
    }
}

impl Widget for DisassemblerView {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        let theme = &self.app_context.theme;
        let mut should_refresh = false;

        let response = user_interface
            .allocate_ui_with_layout(user_interface.available_size(), Layout::top_down(Align::Min), |user_interface| {
                let toolbar_height = 36.0;
                let (toolbar_rect, _) =
                    user_interface.allocate_exact_size(vec2(user_interface.available_width(), toolbar_height), Sense::hover());

                user_interface
                    .painter()
                    .rect_filled(toolbar_rect, CornerRadius::ZERO, theme.background_primary);

                let builder = UiBuilder::new().max_rect(toolbar_rect).layout(Layout::left_to_right(Align::Center));
                let mut toolbar_ui = user_interface.new_child(builder);

                let mut disassembler_view_data = match self.disassembler_view_data.write("Disassembler view toolbar") {
                    Some(data) => data,
                    None => return,
                };

                let address_box = Rect::from_min_size(pos2(toolbar_rect.min.x + 8.0, toolbar_rect.min.y + 4.0), vec2(180.0, 28.0));
                let _address_response = toolbar_ui.put(
                    address_box,
                    TextEdit::singleline(&mut disassembler_view_data.address_input)
                        .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                        .text_color(theme.hexadecimal_green)
                        .background_color(theme.background_primary),
                );

                toolbar_ui.painter().rect_stroke(
                    address_box,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, theme.submenu_border),
                    StrokeKind::Inside,
                );

                let go_button_rect = Rect::from_min_size(pos2(address_box.max.x + 8.0, toolbar_rect.min.y + 4.0), vec2(64.0, 28.0));
                if toolbar_ui
                    .put(
                        go_button_rect,
                        Button::new_from_theme(theme)
                            .background_color(Color32::TRANSPARENT)
                            .with_tooltip_text("Go"),
                    )
                    .clicked()
                {
                    should_refresh = true;
                }

                toolbar_ui.painter().rect_stroke(
                    go_button_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, theme.submenu_border),
                    StrokeKind::Inside,
                );

                toolbar_ui.label("Disassemble");

                drop(disassembler_view_data);

                user_interface.add_space(4.0);

                ScrollArea::vertical().auto_shrink([false, false]).show(user_interface, |user_interface| {
                    let disassembler_view_data = match self.disassembler_view_data.read("Disassembler view data list") {
                        Some(data) => data,
                        None => return,
                    };

                    // Clone required data and drop the lock quickly. Holding the read lock while rendering
                    // rows can starve async callbacks that want to write updated disassembly results.
                    let is_loading = disassembler_view_data.is_loading;
                    let error_message = disassembler_view_data.error_message.clone();
                    let lines = disassembler_view_data.lines.clone();
                    let module_name_present = disassembler_view_data.module_name.is_some();
                    let highlight_address = disassembler_view_data.highlight_address;
                    let highlight_pending = disassembler_view_data.highlight_pending;

                    drop(disassembler_view_data);

                    let mut highlight_consumed = false;

                    if is_loading {
                        user_interface.allocate_ui_with_layout(
                            vec2(user_interface.available_width(), 32.0),
                            Layout::centered_and_justified(Direction::LeftToRight),
                            |ui| {
                            ui.add(Spinner::new().color(theme.foreground));
                        },
                        );
                        return;
                    }

                    if let Some(error_message) = &error_message {
                        user_interface.allocate_ui_with_layout(
                            vec2(user_interface.available_width(), 32.0),
                            Layout::centered_and_justified(Direction::LeftToRight),
                            |ui| {
                                ui.label(error_message);
                            },
                        );
                        return;
                    }

                    let address_width = 180.0;
                    let bytes_width = 200.0;

                    if lines.is_empty() {
                        user_interface.allocate_ui_with_layout(
                            vec2(user_interface.available_width(), 32.0),
                            Layout::centered_and_justified(Direction::LeftToRight),
                            |ui| {
                                ui.label("No disassembly yet.");
                            },
                        );
                        return;
                    }

                    for line in &lines {
                        let is_highlighted = highlight_address == Some(line.address);
                        let row_response = user_interface
                            .allocate_ui_with_layout(
                                vec2(user_interface.available_width(), 20.0),
                                Layout::left_to_right(Align::Min),
                                |ui| {
                                    let row_rect = ui.available_rect_before_wrap();
                                    if is_highlighted {
                                        ui.painter().rect_filled(row_rect, 0.0, theme.selected_background);
                                    }

                                    let address_resp = ui.add_sized(
                                        vec2(address_width, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(&line.display_address)
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.hexadecimal_green),
                                        ),
                                    );
                                    if module_name_present {
                                        let _address_resp = address_resp.on_hover_text(format!("0x{:016X}", line.address));
                                    }

                                    ui.add_sized(
                                        vec2(bytes_width, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(&line.bytes)
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.foreground),
                                        ),
                                    );

                                    ui.label(
                                        eframe::egui::RichText::new(&line.instruction)
                                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                },
                            )
                            .response;

                        if highlight_pending && is_highlighted {
                            row_response.scroll_to_me(Some(Align::Center));
                            highlight_consumed = true;
                        }

                        row_response.context_menu(|ui| {
                            if ui.button("Copy address").clicked() {
                                ui.ctx().copy_text(line.display_address.clone());
                                ui.close();
                            }
                            if ui.button("Copy absolute address").clicked() {
                                ui.ctx().copy_text(format!("0x{:016X}", line.address));
                                ui.close();
                            }
                            if ui.button("Copy bytes").clicked() {
                                ui.ctx().copy_text(line.bytes.trim().to_string());
                                ui.close();
                            }
                            if ui.button("Copy instruction").clicked() {
                                ui.ctx().copy_text(line.instruction.clone());
                                ui.close();
                            }
                        });
                    }

                    if highlight_consumed {
                        if let Some(mut data) = self.disassembler_view_data.write("Disassembler consume highlight") {
                            data.highlight_pending = false;
                        }
                    }
                });
            })
            .response;

        if should_refresh {
            DisassemblerViewData::refresh(self.disassembler_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
        }

        response
    }
}
