use crate::structures::data_types::data_type_error::DataTypeError;
use crate::structures::data_types::data_type_ref::DataTypeRef;
use crate::structures::data_values::anonymous_value_string::AnonymousValueString;
use crate::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;
use crate::structures::memory::endian::Endian;
use crate::structures::{data_types::data_type::DataType, data_values::data_value::DataValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataTypeAob {}

impl DataTypeAob {
    pub const DATA_TYPE_ID: &str = "aob";

    pub fn get_data_type_id() -> &'static str {
        Self::DATA_TYPE_ID
    }

    pub fn get_icon_id() -> &'static str {
        Self::DATA_TYPE_ID
    }

    fn parse_hex_bytes(value_string: &str) -> Result<Vec<u8>, DataTypeError> {
        let trimmed = value_string.trim();
        if trimmed.is_empty() {
            return Err(DataTypeError::ParseError("AOB pattern cannot be empty.".to_string()));
        }

        let mut bytes = Vec::new();
        let separators = |ch: char| ch.is_whitespace() || ch == ',';
        let tokens: Vec<&str> = trimmed.split(separators).filter(|token| !token.is_empty()).collect();

        if tokens.len() <= 1 {
            let cleaned = trimmed
                .replace(',', " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("");
            let mut cleaned = cleaned.as_str();
            if cleaned.starts_with("0x") || cleaned.starts_with("0X") {
                cleaned = &cleaned[2..];
            }

            if cleaned.len() % 2 != 0 {
                return Err(DataTypeError::ParseError("AOB hex string length must be even.".to_string()));
            }

            for chunk in cleaned.as_bytes().chunks(2) {
                let hex_pair = std::str::from_utf8(chunk).map_err(|_| DataTypeError::ParseError("Invalid UTF-8 in hex string.".to_string()))?;
                let value = u8::from_str_radix(hex_pair, 16)
                    .map_err(|error| DataTypeError::ParseError(format!("Failed to parse hex byte '{}': {}", hex_pair, error)))?;
                bytes.push(value);
            }

            return Ok(bytes);
        }

        for token in tokens {
            let mut token = token.trim();
            if token.starts_with("0x") || token.starts_with("0X") {
                token = &token[2..];
            }

            if token.is_empty() {
                continue;
            }

            let token = if token.len() == 1 {
                format!("0{}", token)
            } else {
                token.to_string()
            };

            if token.len() != 2 {
                return Err(DataTypeError::ParseError(format!(
                    "Invalid AOB token '{}'. Expected 1-2 hex digits.",
                    token
                )));
            }

            let value = u8::from_str_radix(&token, 16)
                .map_err(|error| DataTypeError::ParseError(format!("Failed to parse hex byte '{}': {}", token, error)))?;
            bytes.push(value);
        }

        Ok(bytes)
    }

    fn format_hex_bytes(value_bytes: &[u8]) -> String {
        value_bytes
            .iter()
            .map(|value| format!("{:02X}", value))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl DataType for DataTypeAob {
    fn get_data_type_id(&self) -> &str {
        Self::get_data_type_id()
    }

    fn get_icon_id(&self) -> &str {
        Self::get_icon_id()
    }

    fn get_unit_size_in_bytes(&self) -> u64 {
        1
    }

    fn validate_value_string(
        &self,
        anonymous_value_string: &AnonymousValueString,
    ) -> bool {
        self.deanonymize_value_string(anonymous_value_string).is_ok()
    }

    fn deanonymize_value_string(
        &self,
        anonymous_value_string: &AnonymousValueString,
    ) -> Result<DataValue, DataTypeError> {
        let bytes = match anonymous_value_string.get_anonymous_value_string_format() {
            AnonymousValueStringFormat::Hexadecimal
            | AnonymousValueStringFormat::String
            | AnonymousValueStringFormat::Decimal
            | AnonymousValueStringFormat::Address => {
                Self::parse_hex_bytes(anonymous_value_string.get_anonymous_value_string())?
            }
            AnonymousValueStringFormat::Binary => {
                return Err(DataTypeError::ParseError("Binary format is not supported for AOB.".to_string()));
            }
            _ => {
                return Err(DataTypeError::ParseError("Unsupported data value format".to_string()));
            }
        };

        Ok(DataValue::new(DataTypeRef::new(Self::get_data_type_id()), bytes))
    }

    fn anonymize_value_bytes(
        &self,
        value_bytes: &[u8],
        anonymous_value_string_format: AnonymousValueStringFormat,
    ) -> Result<AnonymousValueString, DataTypeError> {
        match anonymous_value_string_format {
            AnonymousValueStringFormat::Hexadecimal | AnonymousValueStringFormat::Address => Ok(AnonymousValueString::new(
                Self::format_hex_bytes(value_bytes),
                anonymous_value_string_format,
                crate::structures::data_values::container_type::ContainerType::ArrayFixed(value_bytes.len() as u64),
            )),
            _ => Err(DataTypeError::ParseError("Unsupported data value format".to_string())),
        }
    }

    fn get_supported_anonymous_value_string_formats(&self) -> Vec<AnonymousValueStringFormat> {
        vec![AnonymousValueStringFormat::Hexadecimal]
    }

    fn get_default_anonymous_value_string_format(&self) -> AnonymousValueStringFormat {
        AnonymousValueStringFormat::Hexadecimal
    }

    fn get_endian(&self) -> Endian {
        Endian::Little
    }

    fn is_floating_point(&self) -> bool {
        false
    }

    fn is_signed(&self) -> bool {
        false
    }

    fn get_default_value(
        &self,
        data_type_ref: DataTypeRef,
    ) -> DataValue {
        DataValue::new(data_type_ref, vec![])
    }
}
