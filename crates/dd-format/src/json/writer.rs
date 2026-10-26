use serde_json::{Number, Value};

use crate::godot::trim_fraction_zeros;

const INDENT: char = '\t';
const REAL_DECIMALS: usize = 6;

/// Writes `value` like Godot 3 `JSON.print(value, "\t")`.
#[must_use]
pub fn to_godot_json(value: &Value) -> String {
    let mut writer = Writer::default();
    writer.write_value(value, 0);
    writer.output
}

#[derive(Default)]
struct Writer {
    output: String,
}

impl Writer {
    fn write_value(&mut self, value: &Value, depth: usize) {
        match value {
            Value::Null => self.output.push_str("null"),
            Value::Bool(flag) => self.output.push_str(if *flag { "true" } else { "false" }),
            Value::Number(number) => self.output.push_str(&format_number(number)),
            Value::String(text) => self.write_string(text),
            Value::Array(items) => {
                let entries = items.iter().map(|item| (None, item));
                self.write_container(['[', ']'], entries, depth);
            }
            Value::Object(fields) => {
                let entries = fields
                    .iter()
                    .map(|(key, field)| (Some(key.as_str()), field));
                self.write_container(['{', '}'], entries, depth);
            }
        }
    }

    /// Godot puts every entry on its own line, so an empty container
    /// still gets an opening and a closing line break.
    fn write_container<'a>(
        &mut self,
        [open, close]: [char; 2],
        entries: impl Iterator<Item = (Option<&'a str>, &'a Value)>,
        depth: usize,
    ) {
        self.output.push(open);
        self.output.push('\n');
        for (index, (key, value)) in entries.enumerate() {
            if index > 0 {
                self.output.push_str(",\n");
            }
            self.write_indent(depth + 1);
            if let Some(key) = key {
                self.write_string(key);
                self.output.push_str(": ");
            }
            self.write_value(value, depth + 1);
        }
        self.output.push('\n');
        self.write_indent(depth);
        self.output.push(close);
    }

    fn write_indent(&mut self, depth: usize) {
        self.output.extend(std::iter::repeat_n(INDENT, depth));
    }

    /// Mirrors Godot's `String::json_escape`, including its non-standard `\v`.
    fn write_string(&mut self, text: &str) {
        self.output.push('"');
        for character in text.chars() {
            match escape(character) {
                Some(escaped) => self.output.push_str(escaped),
                None => self.output.push(character),
            }
        }
        self.output.push('"');
    }
}

fn escape(character: char) -> Option<&'static str> {
    let escaped = match character {
        '\\' => r"\\",
        '"' => r#"\""#,
        '\n' => r"\n",
        '\t' => r"\t",
        '\r' => r"\r",
        '\u{8}' => r"\b",
        '\u{c}' => r"\f",
        '\u{b}' => r"\v",
        _ => return None,
    };
    Some(escaped)
}

/// Godot parses every JSON number as a real and prints it with `%f`,
/// trailing zeros removed: `4.0` gives `4` and `0.2` gives `0.2`.
fn format_number(number: &Number) -> String {
    if number.is_i64() || number.is_u64() {
        return number.to_string();
    }
    let real = number
        .as_f64()
        .expect("serde_json numbers are i64, u64 or f64");
    trim_fraction_zeros(&format!("{real:.REAL_DECIMALS$}")).to_owned()
}

#[cfg(test)]
mod tests {
    use super::to_godot_json;
    use serde_json::{Value, json};

    fn written(value: &Value) -> String {
        to_godot_json(value)
    }

    #[test]
    fn writes_scalars() {
        assert_eq!(written(&json!(null)), "null");
        assert_eq!(written(&json!(true)), "true");
        assert_eq!(written(&json!(false)), "false");
        assert_eq!(written(&json!(-400)), "-400");
    }

    #[test]
    fn writes_reals_with_at_most_six_decimals() {
        assert_eq!(written(&json!(1.772_727)), "1.772727");
        assert_eq!(written(&json!(0.2)), "0.2");
        assert_eq!(written(&json!(4.0)), "4");
        assert_eq!(written(&json!(-std::f64::consts::PI)), "-3.141593");
    }

    #[test]
    fn escapes_strings_like_godot() {
        let text = "a\"b\\c\nd\te\rf\u{8}g\u{c}h\u{b}i/é";
        assert_eq!(written(&json!(text)), r#""a\"b\\c\nd\te\rf\bg\fh\vi/é""#);
    }

    #[test]
    fn writes_empty_containers_with_a_blank_line() {
        assert_eq!(written(&json!([])), "[\n\n]");
        assert_eq!(written(&json!({})), "{\n\n}");
    }

    #[test]
    fn indents_nested_containers_with_tabs() {
        let value = json!({ "a": [1, { "b": [] }], "c": "d" });
        let expected = "{\n\t\"a\": [\n\t\t1,\n\t\t{\n\t\t\t\"b\": [\n\n\t\t\t]\n\t\t}\n\t],\n\t\"c\": \"d\"\n}";
        assert_eq!(written(&value), expected);
    }

    #[test]
    fn keeps_object_key_order() {
        let value: Value = serde_json::from_str(r#"{"z": 1, "-400": 2, "a": 3}"#).unwrap();
        assert_eq!(
            written(&value),
            "{\n\t\"z\": 1,\n\t\"-400\": 2,\n\t\"a\": 3\n}"
        );
    }
}
