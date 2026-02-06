use std::{fmt, mem};

pub struct Conversions {}

impl Conversions {
    pub fn parse_hex_address(src: &str) -> Result<u64, std::num::ParseIntError> {
        let trimmed = src.trim();
        if let Some(hex) = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
        {
            return u64::from_str_radix(hex, 16);
        }

        u64::from_str_radix(trimmed, 16)
    }

    pub fn parse_hex_or_int(src: &str) -> Result<u64, std::num::ParseIntError> {
        if src.starts_with("0x") || src.starts_with("0X") {
            u64::from_str_radix(&src[2..], 16)
        } else {
            src.parse::<u64>()
        }
    }

    pub fn primitive_to_binary<T>(value: &T) -> String
    where
        T: fmt::Binary + fmt::Display,
    {
        format!("{:b}", value)
    }

    pub fn primitive_to_binary_padded<T>(value: &T) -> String
    where
        T: fmt::Binary + fmt::Display,
    {
        let bit_width = mem::size_of::<T>() * 8;
        let bin = format!("{:0bit_width$b}", value, bit_width = bit_width);

        bin
    }

    pub fn primitive_to_hexadecimal<T>(value: &T) -> String
    where
        T: fmt::UpperHex + fmt::Display,
    {
        format!("{:X}", value)
    }

    pub fn primitive_to_hexadecimal_padded<T>(value: &T) -> String
    where
        T: fmt::UpperHex + fmt::Display,
    {
        let hex_width = mem::size_of::<T>() * 2;
        let hex_string = format!("{:0hex_width$X}", value, hex_width = hex_width);

        hex_string
    }
}

#[cfg(test)]
mod tests {
    use super::Conversions;

    #[test]
    fn parse_hex_address_accepts_hex_without_prefix() {
        let parsed = Conversions::parse_hex_address("1D59DFA77D1").expect("hex address should parse");
        assert_eq!(parsed, 0x1D59DFA77D1);
    }

    #[test]
    fn parse_hex_address_accepts_hex_with_prefix() {
        let parsed = Conversions::parse_hex_address("0x1D59DFA77D1").expect("hex address should parse");
        assert_eq!(parsed, 0x1D59DFA77D1);
    }

    #[test]
    fn parse_hex_or_int_keeps_decimal_semantics() {
        let parsed = Conversions::parse_hex_or_int("100").expect("decimal should parse");
        assert_eq!(parsed, 100);
    }
}
