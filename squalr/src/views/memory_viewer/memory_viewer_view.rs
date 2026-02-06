use crate::app_context::AppContext;
use crate::ui::widgets::controls::data_type_selector::data_type_selector_view::DataTypeSelectorView;
use crate::views::disassembler::view_data::disassembler_view_data::DisassemblerViewData;
use crate::views::memory_viewer::view_data::memory_viewer_view_data::MemoryViewerViewData;
use eframe::egui::{
    Align, CentralPanel, Direction, Layout, Response, ScrollArea, Sense, Spinner, TextEdit, Ui, UiBuilder, ViewportBuilder, ViewportId,
    Widget,
};
use epaint::{CornerRadius, Rect, Stroke, StrokeKind, pos2, vec2};
use squalr_engine_api::dependency_injection::dependency::Dependency;
use squalr_engine_api::registries::symbols::symbol_registry::SymbolRegistry;
use squalr_engine_api::structures::data_values::data_value::DataValue;
use squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryViewerView {
    app_context: Arc<AppContext>,
    memory_viewer_view_data: Dependency<MemoryViewerViewData>,
    disassembler_view_data: Dependency<DisassemblerViewData>,
    is_popout: bool,
}

impl MemoryViewerView {
    pub const WINDOW_ID: &'static str = "window_memory_viewer";
    pub const VIEWPORT_ID: &'static str = "viewport_memory_viewer";

    pub fn new(app_context: Arc<AppContext>) -> Self {
        let memory_viewer_view_data = MemoryViewerViewData::register(&app_context);
        let disassembler_view_data = DisassemblerViewData::register(&app_context);

        Self {
            app_context,
            memory_viewer_view_data,
            disassembler_view_data,
            is_popout: false,
        }
    }

    fn from_dependencies(
        app_context: Arc<AppContext>,
        memory_viewer_view_data: Dependency<MemoryViewerViewData>,
        disassembler_view_data: Dependency<DisassemblerViewData>,
        is_popout: bool,
    ) -> Self {
        Self {
            app_context,
            memory_viewer_view_data,
            disassembler_view_data,
            is_popout,
        }
    }

    pub fn show_popout_window(app_context: Arc<AppContext>) {
        let memory_viewer_view_data = app_context
            .dependency_container
            .get_dependency::<MemoryViewerViewData>();
        let disassembler_view_data = app_context
            .dependency_container
            .get_dependency::<DisassemblerViewData>();

        let should_open = memory_viewer_view_data
            .read("Memory viewer popout open state")
            .map(|view_data| view_data.open_popout)
            .unwrap_or(false);

        if !should_open {
            return;
        }

        let viewport_id = ViewportId::from_hash_of(Self::VIEWPORT_ID);
        let builder = ViewportBuilder::default()
            .with_title("Memory View")
            .with_inner_size([1200.0, 760.0]);

        let app_context_clone = app_context.clone();
        let memory_viewer_view_data_clone = memory_viewer_view_data.clone();
        let disassembler_view_data_clone = disassembler_view_data.clone();

        app_context.context.show_viewport_deferred(viewport_id, builder, move |context, _class| {
            if context.input(|input| input.viewport().close_requested()) {
                MemoryViewerViewData::set_popout_open(memory_viewer_view_data_clone.clone(), false);
                return;
            }

            CentralPanel::default().show(context, |user_interface| {
                user_interface.add(Self::from_dependencies(
                    app_context_clone.clone(),
                    memory_viewer_view_data_clone.clone(),
                    disassembler_view_data_clone.clone(),
                    true,
                ));
            });
        });
    }
}

impl Widget for MemoryViewerView {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        let theme = &self.app_context.theme;

        if !self.is_popout {
            let mut should_open_popout = false;
            let mut should_refresh = false;
            let (target_address, region_count) = self
                .memory_viewer_view_data
                .read("Memory viewer dock launcher state")
                .map(|view_data| (view_data.target_address, view_data.regions.len()))
                .unwrap_or((0, 0));

            let response = user_interface
                .allocate_ui_with_layout(user_interface.available_size(), Layout::top_down(Align::Min), |user_interface| {
                    user_interface.add_space(8.0);
                    user_interface.label(
                        eframe::egui::RichText::new("Memory Viewer is now a pop-out window (CE style).")
                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                            .color(theme.foreground),
                    );
                    user_interface.add_space(8.0);
                    user_interface.label(
                        eframe::egui::RichText::new(format!("Target: {:X} | Regions: {}", target_address, region_count))
                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                            .color(theme.hexadecimal_green),
                    );
                    user_interface.add_space(12.0);

                    if user_interface.button("Open Memory View").clicked() {
                        should_open_popout = true;
                    }

                    user_interface.add_space(8.0);
                    if user_interface.button("Refresh").clicked() {
                        should_refresh = true;
                    }
                })
                .response;

            if should_open_popout {
                MemoryViewerViewData::set_popout_open(self.memory_viewer_view_data.clone(), true);
            }

            if should_refresh {
                MemoryViewerViewData::refresh(self.memory_viewer_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
            }

            return response;
        }

        let mut should_refresh = false;
        let mut jump_to_region_base: Option<u64> = None;
        let mut disassemble_region_base: Option<u64> = None;

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

                let mut memory_viewer_view_data = match self.memory_viewer_view_data.write("Memory viewer toolbar") {
                    Some(data) => data,
                    None => return,
                };

                let address_box = Rect::from_min_size(pos2(toolbar_rect.min.x + 8.0, toolbar_rect.min.y + 4.0), vec2(180.0, 28.0));
                toolbar_ui.put(
                    address_box,
                    TextEdit::singleline(&mut memory_viewer_view_data.address_input)
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
                        eframe::egui::Button::new("Go"),
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

                let popout_button_text = if self.is_popout { "Dock" } else { "Pop-out" };
                let popout_button_rect = Rect::from_min_size(pos2(go_button_rect.max.x + 8.0, toolbar_rect.min.y + 4.0), vec2(84.0, 28.0));
                if toolbar_ui
                    .put(
                        popout_button_rect,
                        eframe::egui::Button::new(popout_button_text),
                    )
                    .clicked()
                {
                    MemoryViewerViewData::set_popout_open(self.memory_viewer_view_data.clone(), !self.is_popout);
                }

                toolbar_ui.painter().rect_stroke(
                    popout_button_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, theme.submenu_border),
                    StrokeKind::Inside,
                );

                toolbar_ui.add_space(8.0);
                toolbar_ui.add(DataTypeSelectorView::new(
                    self.app_context.clone(),
                    &mut memory_viewer_view_data.display_data_type,
                    "memory_viewer_display_type_selector",
                )
                .width(180.0));

                let region_label = format!(
                    "Region: {:016X} (+{} bytes)",
                    memory_viewer_view_data.region_base,
                    memory_viewer_view_data.region_size
                );
                toolbar_ui.label(region_label);

                if memory_viewer_view_data.regions.is_empty()
                    && !memory_viewer_view_data.is_loading
                    && memory_viewer_view_data.address_input.trim().is_empty()
                {
                    should_refresh = true;
                }

                drop(memory_viewer_view_data);

                user_interface.add_space(4.0);

                if self.is_popout {
                    let disassembler_height = user_interface.available_height().clamp(220.0, 360.0);
                    user_interface.allocate_ui_with_layout(
                        vec2(user_interface.available_width(), disassembler_height),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.label("Disassembler");
                            ui.add_space(4.0);

                            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                                let disassembler_view_data = match self.disassembler_view_data.read("Memory viewer popout disassembler list") {
                                    Some(data) => data,
                                    None => return,
                                };

                                if disassembler_view_data.is_loading {
                                    ui.allocate_ui_with_layout(
                                        vec2(ui.available_width(), 24.0),
                                        Layout::centered_and_justified(Direction::LeftToRight),
                                        |ui| {
                                            ui.add(Spinner::new().color(theme.foreground));
                                        },
                                    );
                                    return;
                                }

                                if let Some(error_message) = &disassembler_view_data.error_message {
                                    ui.label(error_message);
                                    return;
                                }

                                if disassembler_view_data.lines.is_empty() {
                                    ui.label("No disassembly yet.");
                                    return;
                                }

                                for line in &disassembler_view_data.lines {
                                    let is_highlighted = disassembler_view_data.highlight_address == Some(line.address);
                                    let row_response = ui
                                        .allocate_ui_with_layout(
                                            vec2(ui.available_width(), 20.0),
                                            Layout::left_to_right(Align::Min),
                                            |ui| {
                                                if is_highlighted {
                                                    let row_rect = ui.available_rect_before_wrap();
                                                    ui.painter().rect_filled(row_rect, 0.0, theme.selected_background);
                                                }

                                                ui.add_sized(
                                                    vec2(210.0, 20.0),
                                                    eframe::egui::Label::new(
                                                        eframe::egui::RichText::new(&line.display_address)
                                                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                            .color(theme.hexadecimal_green),
                                                    ),
                                                );

                                                ui.add_sized(
                                                    vec2(190.0, 20.0),
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

                                    if is_highlighted && disassembler_view_data.highlight_pending {
                                        row_response.scroll_to_me(Some(Align::Center));
                                    }
                                }
                            });
                        },
                    );

                    user_interface.add_space(8.0);
                }

                // Regions list + hex view, similar to CE's "Memory Region" workflow.
                user_interface.horizontal(|ui| {
                    let memory_viewer_view_data = match self.memory_viewer_view_data.read("Memory viewer list") {
                        Some(data) => data,
                        None => return,
                    };

                    // Left: regions list (columnar layout to avoid compressed/overlapping rows).
                    ui.allocate_ui_with_layout(
                        vec2(460.0, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.label("Memory Regions");
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    vec2(120.0, 18.0),
                                    eframe::egui::Label::new(
                                        eframe::egui::RichText::new("Base")
                                            .font(theme.font_library.font_noto_sans.font_small.clone())
                                            .color(theme.foreground),
                                    ),
                                );
                                ui.add_sized(
                                    vec2(120.0, 18.0),
                                    eframe::egui::Label::new(
                                        eframe::egui::RichText::new("End")
                                            .font(theme.font_library.font_noto_sans.font_small.clone())
                                            .color(theme.foreground),
                                    ),
                                );
                                ui.add_sized(
                                    vec2(70.0, 18.0),
                                    eframe::egui::Label::new(
                                        eframe::egui::RichText::new("Size")
                                            .font(theme.font_library.font_noto_sans.font_small.clone())
                                            .color(theme.foreground),
                                    ),
                                );
                                ui.label(
                                    eframe::egui::RichText::new("Module+Offset")
                                        .font(theme.font_library.font_noto_sans.font_small.clone())
                                        .color(theme.foreground),
                                );
                            });
                            ui.separator();

                            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                                let highlight_base = memory_viewer_view_data.region_base;

                                for region in memory_viewer_view_data.regions.iter() {
                                    let base = region.base_address;
                                    let end = region.base_address.saturating_add(region.region_size);
                                    let is_highlight = base == highlight_base && highlight_base != 0;

                                    let module_label = if region.module_name.is_empty() {
                                        format!("{:016X}", base)
                                    } else {
                                        format!("{}+{:X}", region.module_name, region.module_offset)
                                    };

                                    let (row_rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
                                    if is_highlight {
                                        ui.painter().rect_filled(row_rect, 0.0, theme.selected_background);
                                    }
                                    let builder = UiBuilder::new().max_rect(row_rect).layout(Layout::left_to_right(Align::Center));
                                    let mut row_ui = ui.new_child(builder);
                                    row_ui.add_sized(
                                        vec2(120.0, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(format!("{:016X}", base))
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.hexadecimal_green),
                                        ),
                                    );
                                    row_ui.add_sized(
                                        vec2(120.0, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(format!("{:016X}", end))
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.foreground),
                                        ),
                                    );
                                    row_ui.add_sized(
                                        vec2(70.0, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(region.region_size.to_string())
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.foreground),
                                        ),
                                    );
                                    row_ui.label(
                                        eframe::egui::RichText::new(module_label)
                                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                            .color(theme.foreground),
                                    );

                                    if is_highlight {
                                        response.scroll_to_me(Some(Align::Center));
                                    }

                                    response.context_menu(|ui| {
                                        if ui.button("Copy base address").clicked() {
                                            ui.ctx().copy_text(format!("{:X}", base));
                                            ui.close();
                                        }
                                        if ui.button("Copy end address").clicked() {
                                            ui.ctx().copy_text(format!("{:X}", end));
                                            ui.close();
                                        }
                                        if ui.button("Copy size").clicked() {
                                            ui.ctx().copy_text(region.region_size.to_string());
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button("Browse this memory region").clicked() {
                                            jump_to_region_base = Some(base);
                                            ui.close();
                                        }
                                        if ui.button("Disassemble this memory region").clicked() {
                                            disassemble_region_base = Some(base);
                                            ui.close();
                                        }
                                    });

                                    if response.clicked() {
                                        jump_to_region_base = Some(base);
                                    }
                                }

                                if memory_viewer_view_data.regions.is_empty() {
                                    ui.label(eframe::egui::RichText::new("No regions available (select a process).").color(theme.foreground));
                                }
                            });
                        },
                    );

                    ui.add_space(8.0);

                    // Right: hex view
                    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::Min), |ui| {
                        if memory_viewer_view_data.is_loading {
                            ui.allocate_ui_with_layout(
                                vec2(ui.available_width(), 32.0),
                                Layout::centered_and_justified(Direction::LeftToRight),
                                |ui| {
                                    ui.add(Spinner::new().color(theme.foreground));
                                },
                            );
                            return;
                        }

                        let bytes_per_row = memory_viewer_view_data.bytes_per_row;
                        let total_rows = memory_viewer_view_data.row_count;
                        let base = memory_viewer_view_data.base_address;
                        let target_address = memory_viewer_view_data.target_address;
                        let bytes = &memory_viewer_view_data.bytes;
                        let display_data_type = memory_viewer_view_data.display_data_type.clone();

                        if let Some(error_message) = &memory_viewer_view_data.error_message {
                            ui.label(error_message);
                        }

                        let symbol_registry = SymbolRegistry::get_instance();
                        let display_value = {
                            let unit_size = symbol_registry.get_unit_size_in_bytes(&display_data_type) as usize;
                            let offset = target_address.saturating_sub(base) as usize;
                            let max_len = bytes.len().saturating_sub(offset);

                            if unit_size == 0 || max_len == 0 {
                                "??".to_string()
                            } else {
                                let read_len = if display_data_type.get_data_type_id() == "string_utf8"
                                    || display_data_type.get_data_type_id() == "aob"
                                {
                                    max_len.min(64)
                                } else {
                                    unit_size.min(max_len)
                                };

                                let slice_end = offset.saturating_add(read_len);
                                let slice = &bytes[offset..slice_end];
                                let data_value = DataValue::new(display_data_type.clone(), slice.to_vec());
                                let format = if display_data_type.get_data_type_id() == "aob" {
                                    AnonymousValueStringFormat::Hexadecimal
                                } else {
                                    symbol_registry.get_default_anonymous_value_string_format(&display_data_type)
                                };

                                symbol_registry
                                    .anonymize_value(&data_value, format)
                                    .map(|value| value.get_anonymous_value_string().to_string())
                                    .unwrap_or_else(|_| "??".to_string())
                            }
                        };

                        ui.label("Hex View");
                        ui.label(format!("Value: {}", display_value));
                        ui.separator();

                        ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                            for row in 0..total_rows {
                                let start = row.saturating_mul(bytes_per_row);
                                let address = base.saturating_add(start as u64);

                                let mut hex_parts = Vec::with_capacity(bytes_per_row);
                                let mut ascii = String::with_capacity(bytes_per_row);

                                for col in 0..bytes_per_row {
                                    let index = start.saturating_add(col);
                                    if let Some(byte) = bytes.get(index).copied() {
                                        hex_parts.push(format!("{:02X}", byte));
                                        let ch = byte as char;
                                        ascii.push(if ch.is_ascii_graphic() { ch } else { '.' });
                                    } else {
                                        hex_parts.push("??".to_string());
                                        ascii.push('.');
                                    }
                                }

                                let hex = hex_parts.join(" ");

                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        vec2(110.0, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(format!("{:016X}", address))
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.hexadecimal_green),
                                        ),
                                    );

                                    ui.add_sized(
                                        vec2(360.0, 20.0),
                                        eframe::egui::Label::new(
                                            eframe::egui::RichText::new(hex)
                                                .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                                .color(theme.foreground),
                                        ),
                                    );

                                    ui.label(
                                        eframe::egui::RichText::new(ascii)
                                            .font(theme.font_library.font_ubuntu_mono_bold.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                            }
                        });
                    });
                });
            })
            .response;

        if let Some(base) = jump_to_region_base {
            MemoryViewerViewData::set_target_address(self.memory_viewer_view_data.clone(), self.app_context.engine_unprivileged_state.clone(), base);
            DisassemblerViewData::set_target_address(self.disassembler_view_data.clone(), self.app_context.engine_unprivileged_state.clone(), base);
        }

        if let Some(base) = disassemble_region_base {
            DisassemblerViewData::set_target_address(self.disassembler_view_data.clone(), self.app_context.engine_unprivileged_state.clone(), base);
            if let Ok(mut docking_manager) = self.app_context.docking_manager.write() {
                docking_manager.set_window_visible(crate::views::disassembler::disassembler_view::DisassemblerView::WINDOW_ID, true);
            }
        }

        if should_refresh {
            MemoryViewerViewData::refresh(self.memory_viewer_view_data.clone(), self.app_context.engine_unprivileged_state.clone());
            if let Some(address) = self
                .memory_viewer_view_data
                .read("Memory viewer refresh disassembler target")
                .map(|view_data| view_data.target_address)
            {
                if address != 0 {
                    DisassemblerViewData::set_target_address(
                        self.disassembler_view_data.clone(),
                        self.app_context.engine_unprivileged_state.clone(),
                        address,
                    );
                }
            }
        }

        response
    }
}
