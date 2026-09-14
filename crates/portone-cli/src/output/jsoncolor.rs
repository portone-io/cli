use std::io::Write;

use serde_json::Value;

const COLOR_DELIM: &str = "1;37";
const COLOR_KEY: &str = "1;34";
const COLOR_NULL: &str = "36";
const COLOR_STRING: &str = "32";
const COLOR_BOOL: &str = "33";

pub fn write_colored(
    w: &mut dyn Write,
    value: &Value,
    indent: &str,
    base_depth: usize,
) -> std::io::Result<()> {
    write_value(w, value, indent, base_depth)?;
    write!(w, "\n{}", indent.repeat(base_depth.saturating_sub(1)))
}

fn write_value(
    w: &mut dyn Write,
    value: &Value,
    indent: &str,
    depth: usize,
) -> std::io::Result<()> {
    match value {
        Value::Null => write_token(w, COLOR_NULL, "null"),
        Value::Bool(true) => write_token(w, COLOR_BOOL, "true"),
        Value::Bool(false) => write_token(w, COLOR_BOOL, "false"),
        Value::Number(number) => write!(w, "{number}"),
        Value::String(text) => write_token(w, COLOR_STRING, &escape_string(text)?),
        Value::Array(items) => {
            write_token(w, COLOR_DELIM, "[")?;
            if items.is_empty() {
                return write_token(w, COLOR_DELIM, "]");
            }
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    write_token(w, COLOR_DELIM, ",")?;
                }
                write!(w, "\n{}", indent.repeat(depth + 1))?;
                write_value(w, item, indent, depth + 1)?;
            }
            write!(w, "\n{}", indent.repeat(depth))?;
            write_token(w, COLOR_DELIM, "]")
        }
        Value::Object(entries) => {
            write_token(w, COLOR_DELIM, "{")?;
            if entries.is_empty() {
                return write_token(w, COLOR_DELIM, "}");
            }
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    write_token(w, COLOR_DELIM, ",")?;
                }
                write!(w, "\n{}", indent.repeat(depth + 1))?;
                write_token(w, COLOR_KEY, &escape_string(key)?)?;
                write_token(w, COLOR_DELIM, ":")?;
                write!(w, " ")?;
                write_value(w, item, indent, depth + 1)?;
            }
            write!(w, "\n{}", indent.repeat(depth))?;
            write_token(w, COLOR_DELIM, "}")
        }
    }
}

fn write_token(w: &mut dyn Write, color: &str, token: &str) -> std::io::Result<()> {
    write!(w, "\x1b[{color}m{token}\x1b[m")
}

fn escape_string(text: &str) -> std::io::Result<String> {
    serde_json::to_string(text).map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use anstream::adapter::strip_str;
    use serde_json::json;

    use super::*;

    fn render(value: &Value, base_depth: usize) -> String {
        let mut buf = Vec::new();
        write_colored(&mut buf, value, "  ", base_depth).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn c(color: &str, token: &str) -> String {
        format!("\x1b[{color}m{token}\x1b[m")
    }

    #[test]
    fn scalars_at_top_level() {
        assert_eq!(
            render(&json!(null), 0),
            format!("{}\n", c(COLOR_NULL, "null"))
        );
        assert_eq!(
            render(&json!(true), 0),
            format!("{}\n", c(COLOR_BOOL, "true"))
        );
        assert_eq!(render(&json!(42), 0), "42\n");
        assert_eq!(render(&json!(1.5), 0), "1.5\n");
        assert_eq!(
            render(&json!("hi"), 0),
            format!("{}\n", c(COLOR_STRING, "\"hi\""))
        );
    }

    #[test]
    fn empty_containers() {
        let delim = |t| c(COLOR_DELIM, t);
        assert_eq!(
            render(&json!({}), 0),
            format!("{}{}\n", delim("{"), delim("}"))
        );
        assert_eq!(
            render(&json!([]), 0),
            format!("{}{}\n", delim("["), delim("]"))
        );
    }

    #[test]
    fn nested_object_and_array() {
        let value = json!({
            "name": "PortOne",
            "ok": true,
            "none": null,
            "count": 42,
            "tags": ["a", "b"],
            "meta": {},
        });
        let output = render(&value, 0);
        assert_eq!(
            serde_json::from_str::<Value>(&strip_str(&output).to_string()).unwrap(),
            value
        );
        assert!(output.contains(&c(COLOR_KEY, "\"name\"")));
        assert!(output.contains(&c(COLOR_STRING, "\"PortOne\"")));
    }

    #[test]
    fn string_escaping_matches_serde_json() {
        let raw = "a\"b\\c\nd\te\u{001f}Unicode";
        let value = json!({ raw: raw });
        let output = strip_str(&render(&value, 0)).to_string();
        assert_eq!(serde_json::from_str::<Value>(&output).unwrap(), value);
    }

    #[test]
    fn base_depth_shifts_indentation() {
        let value = json!([1, { "k": "v" }]);
        assert_eq!(
            strip_str(&render(&value, 1)).to_string(),
            "[\n    1,\n    {\n      \"k\": \"v\"\n    }\n  ]\n"
        );
    }

    #[test]
    fn trailing_indent_for_deeper_base_depth() {
        assert_eq!(render(&json!(1), 2), "1\n  ");
    }

    #[test]
    fn number_tokens_are_preserved_verbatim() {
        let value: Value = serde_json::from_str(r#"[1.50, 1e2, 123456789012345678901]"#).unwrap();
        let out = render(&value, 0);
        assert!(out.contains("1.50"), "got: {out}");
        assert!(out.contains("1e+2"), "got: {out}");
        assert!(out.contains("123456789012345678901"), "got: {out}");
    }
}
