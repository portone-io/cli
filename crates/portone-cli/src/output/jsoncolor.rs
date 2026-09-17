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
