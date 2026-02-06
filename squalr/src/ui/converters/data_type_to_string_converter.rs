use squalr_engine_api::structures::data_types::built_in_types::{
    aob::data_type_aob::DataTypeAob, bool8::data_type_bool8::DataTypeBool8, bool32::data_type_bool32::DataTypeBool32, f32::data_type_f32::DataTypeF32,
    f32be::data_type_f32be::DataTypeF32be, f64::data_type_f64::DataTypeF64, f64be::data_type_f64be::DataTypeF64be, i8::data_type_i8::DataTypeI8,
    i16::data_type_i16::DataTypeI16, i16be::data_type_i16be::DataTypeI16be, i32::data_type_i32::DataTypeI32, i32be::data_type_i32be::DataTypeI32be,
    i64::data_type_i64::DataTypeI64, i64be::data_type_i64be::DataTypeI64be, string::utf8::data_type_string_utf8::DataTypeStringUtf8,
    u8::data_type_u8::DataTypeU8, u16::data_type_u16::DataTypeU16, u16be::data_type_u16be::DataTypeU16be, u32::data_type_u32::DataTypeU32,
    u32be::data_type_u32be::DataTypeU32be, u64::data_type_u64::DataTypeU64, u64be::data_type_u64be::DataTypeU64be,
};

pub struct DataTypeToStringConverter {}

impl DataTypeToStringConverter {
    pub fn convert_data_type_to_string(data_type_id: &str) -> &'static str {
        match data_type_id {
            DataTypeBool8::DATA_TYPE_ID => "Byte (Boolean)",
            DataTypeBool32::DATA_TYPE_ID => "4 Bytes (Boolean)",
            DataTypeU8::DATA_TYPE_ID => "Byte",
            DataTypeU16::DATA_TYPE_ID => "2 Bytes",
            DataTypeU16be::DATA_TYPE_ID => "2 Bytes (BE)",
            DataTypeU32::DATA_TYPE_ID => "4 Bytes",
            DataTypeU32be::DATA_TYPE_ID => "4 Bytes (BE)",
            DataTypeU64::DATA_TYPE_ID => "8 Bytes",
            DataTypeU64be::DATA_TYPE_ID => "8 Bytes (BE)",
            DataTypeI8::DATA_TYPE_ID => "Byte (Signed)",
            DataTypeI16::DATA_TYPE_ID => "2 Bytes (Signed)",
            DataTypeI16be::DATA_TYPE_ID => "2 Bytes (Signed, BE)",
            DataTypeI32::DATA_TYPE_ID => "4 Bytes (Signed)",
            DataTypeI32be::DATA_TYPE_ID => "4 Bytes (Signed, BE)",
            DataTypeI64::DATA_TYPE_ID => "8 Bytes (Signed)",
            DataTypeI64be::DATA_TYPE_ID => "8 Bytes (Signed, BE)",
            DataTypeF32::DATA_TYPE_ID => "Float",
            DataTypeF32be::DATA_TYPE_ID => "Float (BE)",
            DataTypeF64::DATA_TYPE_ID => "Double",
            DataTypeF64be::DATA_TYPE_ID => "Double (BE)",
            DataTypeStringUtf8::DATA_TYPE_ID => "String",
            DataTypeAob::DATA_TYPE_ID => "Array of Bytes",
            _ => "Unknown",
        }
    }
}
