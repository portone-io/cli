use std::io::Read;

use crate::i18n::{LocalizedContext, Localizer};
use anyhow::anyhow;

use crate::error::CliError;
use crate::http::response::HttpResponse;

pub fn resolve_method(
    explicit: Option<&str>,
    has_body: bool,
    paginate: bool,
    graphql: bool,
) -> String {
    match explicit {
        Some(method) => method.to_ascii_uppercase(),
        None if has_body && (graphql || !paginate) => "POST".to_string(),
        None => "GET".to_string(),
    }
}

pub fn build_url(base_url: &str, endpoint: &str) -> String {
    if endpoint.contains("://") {
        endpoint.to_string()
    } else {
        format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }
}

pub fn parse_headers(raw: &[String]) -> Result<Vec<(String, String)>, CliError> {
    parse_headers_localized(raw, crate::i18n::english())
}

pub fn parse_headers_localized(
    raw: &[String],
    localizer: &Localizer,
) -> Result<Vec<(String, String)>, CliError> {
    let mut headers = Vec::with_capacity(raw.len());
    for h in raw {
        let Some(idx) = h.find(':') else {
            return Err(CliError::Flag(crate::tr!(
                localizer,
                "core-header-value",
                header = format!("{h:?}")
            )));
        };
        let name = h[..idx].trim().to_string();
        let value = h[idx + 1..].trim().to_string();
        if ureq::http::HeaderName::from_bytes(name.as_bytes()).is_err() {
            return Err(CliError::Flag(crate::tr!(
                localizer,
                "core-header-name",
                name = format!("{name:?}")
            )));
        }
        if name.eq_ignore_ascii_case("content-length") {
            if value.parse::<u64>().is_err() {
                return Err(CliError::Flag(crate::tr!(
                    localizer,
                    "core-content-length",
                    value = format!("{value:?}")
                )));
            }
            continue;
        }
        headers.push((name, value));
    }
    Ok(headers)
}

pub fn same_origin(a: &str, b: &str) -> bool {
    fn origin(url: &str) -> Option<(String, String, u16)> {
        let uri: ureq::http::Uri = url.parse().ok()?;
        let scheme = uri.scheme_str()?.to_ascii_lowercase();
        let host = uri.host()?.to_ascii_lowercase();
        let port = uri.port_u16().unwrap_or(match scheme.as_str() {
            "https" => 443,
            "http" => 80,
            _ => return None,
        });
        Some((scheme, host, port))
    }
    match (origin(a), origin(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

pub fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
}

pub fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub fn apply_default_headers(
    headers: &mut Vec<(String, String)>,
    json_body: bool,
    authorization: Option<String>,
) {
    if json_body && !has_header(headers, "content-type") {
        headers.push((
            "Content-Type".to_string(),
            "application/json; charset=utf-8".to_string(),
        ));
    }
    if !has_header(headers, "accept") {
        headers.push(("Accept".to_string(), "*/*".to_string()));
    }
    if !has_header(headers, "user-agent") {
        headers.push((
            "User-Agent".to_string(),
            concat!("portone-cli/", env!("CARGO_PKG_VERSION")).to_string(),
        ));
    }
    if let Some(value) = authorization {
        headers.push(("Authorization".to_string(), value));
    }
}

pub fn read_input(path: &str) -> anyhow::Result<Vec<u8>> {
    if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .lcontext(crate::message!("core-input-stdin"))?;
        Ok(buf)
    } else {
        std::fs::read(path).with_lcontext(|| crate::message!("core-input-file", path = path))
    }
}

pub fn build_agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(None)
        .redirect_auth_headers(ureq::config::RedirectAuthHeaders::SameHost)
        .tls_config(tls)
        .build();
    ureq::Agent::new_with_config(config)
}

pub fn send(
    agent: &ureq::Agent,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> anyhow::Result<HttpResponse> {
    use ureq::http;

    let http_method = http::Method::from_bytes(method.as_bytes())
        .map_err(|_| anyhow!(crate::message!("core-http-method", method = method)))?;
    let mut builder = http::Request::builder().method(http_method).uri(url);
    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    let response = match body {
        Some(bytes) => agent.run(
            builder
                .body(bytes.to_vec())
                .lcontext(crate::message!("core-request-build"))?,
        ),
        None => agent.run(
            builder
                .body(())
                .lcontext(crate::message!("core-request-build"))?,
        ),
    }?;

    let status = response.status().as_u16();
    let resp_headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let body = response
        .into_body()
        .into_with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .lcontext(crate::message!("core-response-read"))?;

    Ok(HttpResponse {
        status,
        headers: resp_headers,
        body,
    })
}
