use crate::ui::converters::data_type_to_string_converter::DataTypeToStringConverter;
use crate::ui::widgets::controls::combo_box::combo_box_view::ComboBoxView;
use crate::ui::widgets::controls::data_type_selector::data_type_item_view::DataTypeItemView;
use crate::{app_context::AppContext, ui::converters::data_type_to_icon_converter::DataTypeToIconConverter};
use eframe::egui::{Id, Response, Ui, Widget};
use squalr_engine_api::structures::data_types::{
    built_in_types::{
        aob::data_type_aob::DataTypeAob, f32::data_type_f32::DataTypeF32, f32be::data_type_f32be::DataTypeF32be, f64::data_type_f64::DataTypeF64,
        f64be::data_type_f64be::DataTypeF64be, i8::data_type_i8::DataTypeI8, i16::data_type_i16::DataTypeI16, i16be::data_type_i16be::DataTypeI16be,
        i32::data_type_i32::DataTypeI32, i32be::data_type_i32be::DataTypeI32be, i64::data_type_i64::DataTypeI64, i64be::data_type_i64be::DataTypeI64be,
        u8::data_type_u8::DataTypeU8, u16::data_type_u16::DataTypeU16, u16be::data_type_u16be::DataTypeU16be, u32::data_type_u32::DataTypeU32,
        u32be::data_type_u32be::DataTypeU32be, u64::data_type_u64::DataTypeU64, u64be::data_type_u64be::DataTypeU64be,
        string::utf8::data_type_string_utf8::DataTypeStringUtf8,
    },
    data_type_ref::DataTypeRef,
};
use std::sync::Arc;

/// A widget that allows selecting from a set of data types.
pub struct DataTypeSelectorView<'lifetime> {
    app_context: Arc<AppContext>,
    active_data_type: &'lifetime mut DataTypeRef,
    menu_id: &'lifetime str,
    width: f32,
    height: f32,
}

impl<'lifetime> DataTypeSelectorView<'lifetime> {
    const MIN_COMBO_WIDTH: f32 = 220.0;

    pub fn new(
        app_context: Arc<AppContext>,
        active_data_type: &'lifetime mut DataTypeRef,
        menu_id: &'lifetime str,
    ) -> Self {
        Self {
            app_context,
            active_data_type,
            menu_id,
            width: 200.0,
            height: 28.0,
        }
    }

    pub fn width(
        mut self,
        width: f32,
    ) -> Self {
        self.width = width;
        self
    }

    pub fn height(
        mut self,
        height: f32,
    ) -> Self {
        self.height = height;
        self
    }

    pub fn close(
        &self,
        user_interface: &mut Ui,
    ) {
        let popup_id = Id::new(("data_type_selector_popup", user_interface.id().value()));

        user_interface.memory_mut(|memory| {
            memory.data.insert_temp(popup_id, false);
        });
    }
}

impl<'lifetime> Widget for DataTypeSelectorView<'lifetime> {
    fn ui(
        self,
        user_interface: &mut Ui,
    ) -> Response {
        let theme = &self.app_context.theme;
        let icon_library = &theme.icon_library;
        let width = self.width.max(Self::MIN_COMBO_WIDTH);
        let height = self.height;
        let element_width = width;
        let data_type_id = self.active_data_type.get_data_type_id();
        let icon = DataTypeToIconConverter::convert_data_type_to_icon(data_type_id, icon_library);

        let combo_box = ComboBoxView::new(
            self.app_context.clone(),
            DataTypeToStringConverter::convert_data_type_to_string(data_type_id),
            self.menu_id,
            Some(icon),
            |popup_user_interface: &mut Ui, should_close: &mut bool| {
                popup_user_interface.vertical(|user_interface| {
                    let mut add_item = |user_interface: &mut Ui, data_type_id: &str| {
                        if user_interface
                            .add(DataTypeItemView::new(
                                self.app_context.clone(),
                                DataTypeToStringConverter::convert_data_type_to_string(data_type_id),
                                Some(DataTypeToIconConverter::convert_data_type_to_icon(data_type_id, icon_library)),
                                element_width,
                            ))
                            .clicked()
                        {
                            *self.active_data_type = DataTypeRef::new(data_type_id);
                            *should_close = true;
                        }
                    };

                    // CE-style primary types.
                    add_item(user_interface, DataTypeU8::get_data_type_id());
                    add_item(user_interface, DataTypeI8::get_data_type_id());
                    add_item(user_interface, DataTypeU16::get_data_type_id());
                    add_item(user_interface, DataTypeI16::get_data_type_id());
                    add_item(user_interface, DataTypeU32::get_data_type_id());
                    add_item(user_interface, DataTypeI32::get_data_type_id());
                    add_item(user_interface, DataTypeU64::get_data_type_id());
                    add_item(user_interface, DataTypeI64::get_data_type_id());
                    add_item(user_interface, DataTypeF32::get_data_type_id());
                    add_item(user_interface, DataTypeF64::get_data_type_id());
                    user_interface.separator();
                    add_item(user_interface, DataTypeStringUtf8::get_data_type_id());
                    add_item(user_interface, DataTypeAob::get_data_type_id());
                    user_interface.separator();
                    // Big-endian variants (advanced).
                    add_item(user_interface, DataTypeU16be::get_data_type_id());
                    add_item(user_interface, DataTypeI16be::get_data_type_id());
                    add_item(user_interface, DataTypeU32be::get_data_type_id());
                    add_item(user_interface, DataTypeI32be::get_data_type_id());
                    add_item(user_interface, DataTypeU64be::get_data_type_id());
                    add_item(user_interface, DataTypeI64be::get_data_type_id());
                    add_item(user_interface, DataTypeF32be::get_data_type_id());
                    add_item(user_interface, DataTypeF64be::get_data_type_id());
                });
            },
        )
        .width(width)
        .height(height);

        // Add the combo box to the layout
        user_interface.add(combo_box)
    }
}
