use serde_json::{Map, Value};

pub fn group_variables(params: &Map<String, Value>) -> Map<String, Value> {
    let mut top = Map::new();
    let mut variables = Map::new();
    for (key, value) in params {
        match key.as_str() {
            "query" | "operationName" => {
                top.insert(key.clone(), value.clone());
            }
            _ => {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
    if !variables.is_empty() {
        top.insert("variables".to_string(), Value::Object(variables));
    }
    top
}

pub fn find_end_cursor(body: &Value) -> Option<String> {
    let page_info = find_page_info(body)?;
    if page_info.get("hasNextPage").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    page_info
        .get("endCursor")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn find_page_info(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "pageInfo"
                    && let Some(obj) = child.as_object()
                {
                    return Some(obj);
                }
                if let Some(found) = find_page_info(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(find_page_info),
        _ => None,
    }
}

pub fn error_message(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let errors = value.as_object()?.get("errors")?.as_array()?;
    let messages: Vec<&str> = errors
        .iter()
        .filter_map(|err| match err {
            Value::String(text) => Some(text.as_str()),
            Value::Object(obj) => obj.get("message").and_then(Value::as_str),
            _ => None,
        })
        .collect();
    if messages.is_empty() {
        return None;
    }
    Some(messages.join("\n"))
}
