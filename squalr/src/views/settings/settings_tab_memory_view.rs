use crate::{
    app_context::AppContext,
    ui::widgets::controls::{checkbox::Checkbox, groupbox::GroupBox},
};
use eframe::egui::{Align, Layout, Response, RichText, Ui, Widget};
use squalr_engine_api::{
    commands::{
        memory::regions::memory_regions_request::MemoryRegionsRequest,
        privileged_command_request::PrivilegedCommandRequest,
        settings::memory::{list::memory_settings_list_request::MemorySettingsListRequest, set::memory_settings_set_request::MemorySettingsSetRequest},
    },
    conversions::storage_size_conversions::StorageSizeConversions,
    structures::settings::memory_settings::MemorySettings,
};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct SettingsTabMemoryView {
    app_context: Arc<AppContext>,
    cached_memory_settings: Arc<RwLock<MemorySettings>>,
    cached_region_preview: Arc<RwLock<Option<(usize, u64)>>>,
}

impl SettingsTabMemoryView {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        let settings_view = Self {
            app_context,
            cached_memory_settings: Arc::new(RwLock::new(MemorySettings::default())),
            cached_region_preview: Arc::new(RwLock::new(None)),
        };

        settings_view.sync_ui_with_memory_settings();
        settings_view.sync_region_preview();
        settings_view.listen_for_process_change();

        settings_view
    }

    fn sync_ui_with_memory_settings(&self) {
        let memory_settings_list_request = MemorySettingsListRequest {};
        let cached_memory_settings = self.cached_memory_settings.clone();

        memory_settings_list_request.send(&self.app_context.engine_unprivileged_state, move |scan_results_query_response| {
            if let Ok(memory_settings) = scan_results_query_response.memory_settings {
                if let Ok(mut cached_memory_settings) = cached_memory_settings.write() {
                    *cached_memory_settings = memory_settings;
                }
            }
        });
    }

    fn sync_region_preview(&self) {
        let memory_regions_request = MemoryRegionsRequest {};
        let cached_region_preview = self.cached_region_preview.clone();

        memory_regions_request.send(&self.app_context.engine_unprivileged_state, move |response| {
            let region_count = response.regions.len();
            let total_bytes = response.regions.iter().map(|region| region.region_size).sum::<u64>();

            if let Ok(mut cached_region_preview) = cached_region_preview.write() {
                *cached_region_preview = Some((region_count, total_bytes));
            }
        });
    }

    fn listen_for_process_change(&self) {
        let engine_unprivileged_state = self.app_context.engine_unprivileged_state.clone();
        let engine_unprivileged_state_for_listener = engine_unprivileged_state.clone();
        let cached_region_preview = self.cached_region_preview.clone();

        engine_unprivileged_state.listen_for_engine_event::<squalr_engine_api::events::process::changed::process_changed_event::ProcessChangedEvent>(
            move |_| {
                let memory_regions_request = MemoryRegionsRequest {};
                let cached_region_preview = cached_region_preview.clone();
                let engine_unprivileged_state = engine_unprivileged_state_for_listener.clone();

                memory_regions_request.send(&engine_unprivileged_state, move |response| {
                    let region_count = response.regions.len();
                    let total_bytes = response.regions.iter().map(|region| region.region_size).sum::<u64>();

                    if let Ok(mut cached_region_preview) = cached_region_preview.write() {
                        *cached_region_preview = Some((region_count, total_bytes));
                    }
                });
            },
        );
    }
}

impl Widget for SettingsTabMemoryView {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        let theme = &self.app_context.theme;
        let cached_memory_settings = match self.cached_memory_settings.read() {
            Ok(cached_memory_settings) => *cached_memory_settings,
            Err(_error) => MemorySettings::default(),
        };
        let cached_region_preview = match self.cached_region_preview.read() {
            Ok(cached_region_preview) => *cached_region_preview,
            Err(_error) => None,
        };
        let mut preview_dirty = false;

        let response = user_interface
            .allocate_ui_with_layout(user_interface.available_size(), Layout::top_down(Align::Min), |user_interface| {
                user_interface.add_space(4.0);
                user_interface.horizontal(|user_interface| {
                    user_interface.add(
                        GroupBox::new_from_theme(theme, "Required Protection Flags", |user_interface| {
                            user_interface.vertical(|user_interface| {
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.required_write))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.required_write;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.required_write = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            required_write: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Write")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.required_execute))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.required_execute;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.required_execute = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            required_execute: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Execute")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.required_copy_on_write))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.required_copy_on_write;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.required_copy_on_write = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            required_copy_on_write: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Copy on Write")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                            });
                        })
                        .desired_width(224.0),
                    );
                    user_interface.add_space(8.0);
                    user_interface.add(
                        GroupBox::new_from_theme(theme, "Excluded Protection Flags", |user_interface| {
                            user_interface.vertical(|user_interface| {
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.excluded_write))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.excluded_write;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.excluded_write = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            excluded_write: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Write")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.excluded_execute))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.excluded_execute;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.excluded_execute = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            excluded_execute: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Execute")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.excluded_copy_on_write))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.excluded_copy_on_write;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.excluded_copy_on_write = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            excluded_copy_on_write: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Copy on Write")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.excluded_no_cache))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.excluded_no_cache;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.excluded_no_cache = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            excluded_no_cache: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("No Cache (skip)")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.excluded_write_combine))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.excluded_write_combine;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.excluded_write_combine = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            excluded_write_combine: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Write Combine (skip)")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                            });
                        })
                        .desired_width(256.0),
                    );
                });

                user_interface.horizontal(|user_interface| {
                    user_interface.add(
                        GroupBox::new_from_theme(theme, "Memory Types", |user_interface| {
                            user_interface.add_space(4.0);
                            user_interface.vertical(|user_interface| {
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.memory_type_none))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.memory_type_none;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.memory_type_none = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            memory_type_none: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("None")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.memory_type_image))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.memory_type_image;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.memory_type_image = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            memory_type_image: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Image")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.memory_type_private))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.memory_type_private;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.memory_type_private = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            memory_type_private: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Private")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.memory_type_mapped))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.memory_type_mapped;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.memory_type_mapped = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            memory_type_mapped: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Mapped (slow)")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                                user_interface.add_space(4.0);
                                user_interface.horizontal(|user_interface| {
                                    if user_interface
                                        .add(Checkbox::new_from_theme(theme).with_check_state_bool(cached_memory_settings.only_main_module_image))
                                        .clicked()
                                    {
                                        let new_value = !cached_memory_settings.only_main_module_image;
                                        if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                            cached_memory_settings.only_main_module_image = new_value;
                                        }

                                        let memory_settings_set_request = MemorySettingsSetRequest {
                                            only_main_module_image: Some(new_value),
                                            ..MemorySettingsSetRequest::default()
                                        };

                                        memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                        preview_dirty = true;
                                    }

                                    user_interface.add_space(8.0);
                                    user_interface.label(
                                        RichText::new("Main module image only")
                                            .font(theme.font_library.font_noto_sans.font_normal.clone())
                                            .color(theme.foreground),
                                    );
                                });
                            });
                        })
                        .desired_width(224.0)
                        // JIRA: Bugged. I believe these rows are not allocating sufficient available height, and then groupbox treats desired as a suggestion.
                        .desired_height(320.0),
                    );
                    user_interface.add_space(8.0);
                user_interface.add(
                    GroupBox::new_from_theme(theme, "Virtual Memory Querying", |user_interface| {
                        user_interface.vertical(|user_interface| {
                            let query_usermode = cached_memory_settings.only_query_usermode;

                            user_interface.horizontal(|user_interface| {
                                if user_interface
                                    .add(Checkbox::new_from_theme(theme).with_check_state_bool(query_usermode))
                                    .clicked()
                                {
                                    if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                        cached_memory_settings.only_query_usermode = true;
                                    }

                                    let memory_settings_set_request = MemorySettingsSetRequest {
                                        only_query_usermode: Some(true),
                                        ..MemorySettingsSetRequest::default()
                                    };

                                    memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                    preview_dirty = true;
                                }

                                user_interface.add_space(8.0);
                                user_interface.label(
                                    RichText::new("Query All Usermode Memory")
                                        .font(theme.font_library.font_noto_sans.font_normal.clone())
                                        .color(theme.foreground),
                                );
                            });

                            user_interface.add_space(4.0);

                            user_interface.horizontal(|user_interface| {
                                if user_interface
                                    .add(Checkbox::new_from_theme(theme).with_check_state_bool(!query_usermode))
                                    .clicked()
                                {
                                    if let Ok(mut cached_memory_settings) = self.cached_memory_settings.write() {
                                        cached_memory_settings.only_query_usermode = false;
                                    }

                                    let memory_settings_set_request = MemorySettingsSetRequest {
                                        only_query_usermode: Some(false),
                                        ..MemorySettingsSetRequest::default()
                                    };

                                    memory_settings_set_request.send(&self.app_context.engine_unprivileged_state, move |_memory_settings_set_response| {});
                                    preview_dirty = true;
                                }

                                user_interface.add_space(8.0);
                                user_interface.label(
                                    RichText::new("Query Custom Range")
                                        .font(theme.font_library.font_noto_sans.font_normal.clone())
                                        .color(theme.foreground),
                                );
                            });
                        });
                    })
                        .desired_width(256.0)
                        // JIRA: Bugged. I believe these rows are not allocating sufficient available height, and then groupbox treats desired as a suggestion.
                        .desired_height(320.0),
                    );
                });

                user_interface.add_space(8.0);
                user_interface.add(
                    GroupBox::new_from_theme(theme, "Scan Coverage (estimate)", |user_interface| {
                        let preview_text = if let Some((region_count, total_bytes)) = cached_region_preview {
                            let size_text = StorageSizeConversions::value_to_metric_size(total_bytes as u128);
                            format!("Scannable regions: {} | Total: {}", region_count, size_text)
                        } else {
                            "Scannable regions: (open a process to preview)".to_string()
                        };

                        user_interface.label(
                            RichText::new(preview_text)
                                .font(theme.font_library.font_noto_sans.font_normal.clone())
                                .color(theme.foreground),
                        );
                    })
                    .desired_width(520.0),
                );

                if preview_dirty {
                    self.sync_region_preview();
                }
            })
            .response;

        response
    }
}
