use crate::i18n::Localizer;
use serde_json::{Map, Value, json};

#[derive(Debug, PartialEq, Eq)]
pub enum Advance {
    Next,
    Done,
    Stop(String),
}

pub struct Paginator {
    number: u64,
    size: u64,
    cursor_size: Option<u64>,
    injected_page: bool,
}

impl Paginator {
    pub fn new(params: &mut Map<String, Value>) -> Paginator {
        if let Some(page) = params.get("page") {
            let number = page.get("number").and_then(Value::as_u64).unwrap_or(0);
            let size = page.get("size").and_then(Value::as_u64).unwrap_or(100);
            Paginator {
                number,
                size,
                cursor_size: None,
                injected_page: false,
            }
        } else if params.contains_key("size") || params.contains_key("cursor") {
            if !params.contains_key("size") {
                params.insert("size".to_string(), json!(1000));
            }
            let cursor_size = params.get("size").and_then(Value::as_u64);
            Paginator {
                number: 0,
                size: 100,
                cursor_size,
                injected_page: false,
            }
        } else {
            params.insert("page".to_string(), json!({"number": 0, "size": 100}));
            Paginator {
                number: 0,
                size: 100,
                cursor_size: None,
                injected_page: true,
            }
        }
    }

    pub fn advance(&mut self, params: &mut Map<String, Value>, body: &Value) -> Advance {
        self.advance_localized(params, body, crate::i18n::english())
    }

    pub fn advance_localized(
        &mut self,
        params: &mut Map<String, Value>,
        body: &Value,
        localizer: &Localizer,
    ) -> Advance {
        let total_count = body
            .get("page")
            .and_then(|p| p.get("totalCount"))
            .and_then(Value::as_u64);
        let items = body.get("items").and_then(Value::as_array);

        if let Some(total) = total_count {
            self.advance_offset(params, body, total, items, localizer)
        } else if let Some(items) = items {
            self.advance_cursor(params, items)
        } else {
            Advance::Stop(crate::tr!(localizer, "core-pagination-unknown"))
        }
    }

    fn advance_offset(
        &mut self,
        params: &mut Map<String, Value>,
        body: &Value,
        total: u64,
        items: Option<&Vec<Value>>,
        localizer: &Localizer,
    ) -> Advance {
        let size = body
            .get("page")
            .and_then(|p| p.get("size"))
            .and_then(Value::as_u64)
            .unwrap_or(self.size);
        if size == 0 {
            return Advance::Done;
        }
        if items.is_none_or(|items| items.is_empty()) {
            return Advance::Done;
        }
        let fetched = (self.number + 1) * size;
        if fetched >= total {
            return Advance::Done;
        }
        if fetched + size > 60000 {
            return Advance::Stop(crate::tr!(localizer, "core-pagination-limit"));
        }

        self.number += 1;
        self.size = size;
        let page = params
            .entry("page".to_string())
            .or_insert_with(|| json!({}));
        if !page.is_object() {
            *page = json!({});
        }
        let obj = page
            .as_object_mut()
            .expect("page was normalized to an object");
        obj.insert("number".to_string(), json!(self.number));
        obj.insert("size".to_string(), json!(self.size));
        Advance::Next
    }

    fn advance_cursor(&mut self, params: &mut Map<String, Value>, items: &[Value]) -> Advance {
        if self.injected_page {
            params.remove("page");
            self.injected_page = false;
        }
        if items.is_empty() {
            return Advance::Done;
        }
        if let Some(requested) = self.cursor_size
            && (items.len() as u64) < requested
        {
            return Advance::Done;
        }
        let Some(cursor) = items
            .last()
            .and_then(|item| item.get("cursor"))
            .and_then(Value::as_str)
        else {
            return Advance::Done;
        };
        params.insert("cursor".to_string(), json!(cursor));
        Advance::Next
    }
}
