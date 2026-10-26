/// Parses unsigned hexadecimal digits only: no sign, no prefix, not empty.
pub fn parse_hex_digits(text: &str) -> Option<u32> {
    let digits_only = !text.is_empty() && text.chars().all(|digit| digit.is_ascii_hexdigit());
    digits_only
        .then(|| u32::from_str_radix(text, 16).ok())
        .flatten()
}
