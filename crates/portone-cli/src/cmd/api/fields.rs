use std::io::Read;

use crate::i18n::LocalizedContext;
use anyhow::{Result, bail};
use serde_json::{Map, Value};

pub fn parse_fields(
    raw_fields: &[String],
    magic_fields: &[String],
    stdin: &mut dyn Read,
) -> Result<Map<String, Value>> {
    let mut params = Map::new();
    for f in raw_fields {
        parse_field(&mut params, f, false, stdin)?;
    }
    for f in magic_fields {
        parse_field(&mut params, f, true, stdin)?;
    }
    Ok(params)
}

fn parse_field(
    params: &mut Map<String, Value>,
    f: &str,
    is_magic: bool,
    stdin: &mut dyn Read,
) -> Result<()> {
    let mut value_index = 0usize;
    let mut keystack: Vec<&str> = Vec::new();
    let mut key_start_at = 0usize;
    for (i, r) in f.char_indices() {
        match r {
            '[' => {
                if key_start_at == 0 {
                    keystack.push(&f[..i]);
                }
                key_start_at = i + 1;
            }
            ']' => keystack.push(&f[key_start_at..i]),
            '=' => {
                if key_start_at == 0 {
                    keystack.push(&f[..i]);
                }
                value_index = i + 1;
                break;
            }
            _ => {}
        }
    }

    if keystack.is_empty() {
        bail!(crate::message!(
            "core-field-invalid",
            key = format!("{f:?}")
        ));
    }

    let key;
    let raw_value: Option<&str>;
    if value_index == 0 {
        if !keystack.last().unwrap().is_empty() {
            bail!(crate::message!("core-field-value", key = format!("{f:?}")));
        }
        key = f;
        raw_value = None;
    } else {
        key = &f[..value_index - 1];
        raw_value = Some(&f[value_index..]);
    }

    let value: Option<Value> = match raw_value {
        None => None,
        Some(s) if is_magic => match magic_field_value(s, stdin)
            .with_lcontext(|| crate::message!("core-field-parse", key = format!("{key:?}")))?
        {
            Value::Null => None,
            v => Some(v),
        },
        Some(s) => Some(Value::String(s.to_string())),
    };

    let mut dest_map: &mut Map<String, Value> = params;
    let mut is_array = false;
    let mut subkey = "";
    for &k in &keystack {
        if k.is_empty() {
            is_array = true;
            continue;
        }
        if !subkey.is_empty() {
            if is_array {
                dest_map = add_params_slice(dest_map, subkey, k)?;
                is_array = false;
            } else {
                dest_map = add_params_map(dest_map, subkey)?;
            }
        }
        subkey = k;
    }

    if is_array {
        match value {
            None => {
                dest_map.insert(subkey.to_string(), Value::Array(Vec::new()));
            }
            Some(v) => match dest_map.get_mut(subkey) {
                Some(Value::Array(arr)) => arr.push(v),
                Some(existing) => {
                    bail!(crate::message!(
                        "core-field-array",
                        key = format!("{subkey:?}"),
                        actual = go_type_name(existing)
                    ));
                }
                None => {
                    dest_map.insert(subkey.to_string(), Value::Array(vec![v]));
                }
            },
        }
    } else {
        if dest_map.contains_key(subkey) {
            bail!(crate::message!(
                "core-field-override",
                key = format!("{subkey:?}")
            ));
        }
        dest_map.insert(subkey.to_string(), value.unwrap_or(Value::Null));
    }
    Ok(())
}

fn add_params_map<'a>(
    m: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    if !m.contains_key(key) {
        m.insert(key.to_string(), Value::Object(Map::new()));
    }
    match m.get_mut(key).unwrap() {
        Value::Object(map) => Ok(map),
        other => bail!(crate::message!(
            "core-field-map",
            key = format!("{key:?}"),
            actual = go_type_name(other)
        )),
    }
}

fn add_params_slice<'a>(
    m: &'a mut Map<String, Value>,
    prevkey: &str,
    newkey: &str,
) -> Result<&'a mut Map<String, Value>> {
    if !m.contains_key(prevkey) {
        m.insert(prevkey.to_string(), Value::Array(Vec::new()));
    }
    match m.get_mut(prevkey).unwrap() {
        Value::Array(arr) => {
            let reuse_last = match arr.last() {
                Some(Value::Object(last)) => match last.get(newkey) {
                    None | Some(Value::Array(_)) => true,
                    Some(_) => false,
                },
                _ => false,
            };
            if !reuse_last {
                arr.push(Value::Object(Map::new()));
            }
            match arr.last_mut().unwrap() {
                Value::Object(map) => Ok(map),
                _ => unreachable!(),
            }
        }
        other => bail!(crate::message!(
            "core-field-array",
            key = format!("{prevkey:?}"),
            actual = go_type_name(other)
        )),
    }
}

fn magic_field_value(v: &str, stdin: &mut dyn Read) -> Result<Value> {
    if let Some(path) = v.strip_prefix('@') {
        return Ok(Value::String(read_user_file(path, stdin)?));
    }

    if let Ok(n) = v.parse::<i64>() {
        return Ok(Value::Number(n.into()));
    }

    Ok(match v {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => Value::String(v.to_string()),
    })
}

fn read_user_file(path: &str, stdin: &mut dyn Read) -> Result<String> {
    let bytes = if path == "-" {
        let mut buf = Vec::new();
        stdin
            .read_to_end(&mut buf)
            .lcontext(crate::message!("core-field-open", path = "-"))?;
        buf
    } else {
        std::fs::read(path).with_lcontext(|| crate::message!("core-field-open", path = path))?
    };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn go_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "<nil>",
        Value::Bool(_) => "bool",
        Value::Number(_) => "int",
        Value::String(_) => "string",
        Value::Array(_) => "[]interface {}",
        Value::Object(_) => "map[string]interface {}",
    }
}
