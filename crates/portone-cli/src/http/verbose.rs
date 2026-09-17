use crate::i18n::Localizer;
use std::io::Write;

use crate::http::response::{HttpResponse, is_json_content_type};

pub fn log_request(
    w: &mut dyn Write,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> std::io::Result<()> {
    log_request_localized(w, method, url, headers, body, crate::i18n::english())
}

pub fn log_request_localized(
    w: &mut dyn Write,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
    localizer: &Localizer,
) -> std::io::Result<()> {
    writeln!(
        w,
        "* {}",
        crate::tr!(localizer, "core-request-log", url = url)
    )?;
    writeln!(w, "> {method} {url}")?;
    for (name, value) in headers {
        if name.trim().eq_ignore_ascii_case("authorization") {
            writeln!(w, "> {name}: {}", mask_authorization(value))?;
        } else {
            writeln!(w, "> {name}: {value}")?;
        }
    }
    writeln!(w)?;
    if let Some(bytes) = body {
        write_body(w, bytes, true, localizer)?;
    }
    Ok(())
}

pub fn log_response(w: &mut dyn Write, resp: &HttpResponse) -> std::io::Result<()> {
    log_response_localized(w, resp, crate::i18n::english())
}

pub fn log_response_localized(
    w: &mut dyn Write,
    resp: &HttpResponse,
    localizer: &Localizer,
) -> std::io::Result<()> {
    writeln!(w, "< HTTP/1.1 {} {}", resp.status, resp.reason())?;
    for (name, value) in &resp.headers {
        writeln!(w, "< {name}: {value}")?;
    }
    writeln!(w)?;
    let texty = resp
        .header("content-type")
        .is_none_or(|ct| is_json_content_type(ct) || ct.starts_with("text/"));
    write_body(w, &resp.body, texty, localizer)
}

fn mask_authorization(value: &str) -> String {
    match value.split_whitespace().next() {
        Some(scheme) if value.trim().contains(char::is_whitespace) => format!("{scheme} ████"),
        _ => "████".to_string(),
    }
}

fn write_body(
    w: &mut dyn Write,
    bytes: &[u8],
    texty_hint: bool,
    localizer: &Localizer,
) -> std::io::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    let head = &bytes[..bytes.len().min(512)];
    if texty_hint && !head.contains(&0) {
        w.write_all(bytes)?;
        writeln!(w)?;
        writeln!(w)
    } else {
        writeln!(
            w,
            "* {}",
            crate::tr!(
                localizer,
                "core-body-omitted",
                bytes = bytes.len().to_string()
            )
        )?;
        writeln!(w)
    }
}
