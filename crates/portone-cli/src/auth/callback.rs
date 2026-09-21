use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail};
use url::Url;

use crate::i18n::{LocalizedContext, LocalizedErrorContext, Localizer};

const REQUEST_LINE_LIMIT: usize = 8 * 1024;
const HEADER_LIMIT: usize = 64 * 1024;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub struct CallbackServer {
    listener: TcpListener,
    path: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callback {
    Code(String),
    Denied {
        error: String,
        description: Option<String>,
    },
}

impl CallbackServer {
    pub fn bind(redirect_uri: &Url) -> anyhow::Result<Self> {
        let host = redirect_uri.host_str().unwrap_or_default();
        if !matches!(host, "127.0.0.1" | "localhost") {
            bail!(crate::message!(
                "auth-callback-invalid-host",
                uri = redirect_uri
            ));
        }
        let port = redirect_uri.port().ok_or_else(|| {
            anyhow!(crate::message!(
                "auth-callback-missing-port",
                uri = redirect_uri
            ))
        })?;
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|err| {
            if err.kind() == std::io::ErrorKind::AddrInUse {
                anyhow!(crate::message!("auth-callback-port-in-use", port = port))
            } else {
                anyhow!(err).lcontext(crate::message!("auth-callback-start-failed", port = port))
            }
        })?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            path: redirect_uri.path().to_string(),
            port,
        })
    }

    pub fn wait(
        &self,
        expected_state: &str,
        timeout: Duration,
        err: &mut dyn Write,
    ) -> anyhow::Result<Callback> {
        self.wait_localized(expected_state, timeout, err, &Localizer::english())
    }

    pub fn wait_localized(
        &self,
        expected_state: &str,
        timeout: Duration,
        err: &mut dyn Write,
        localizer: &Localizer,
    ) -> anyhow::Result<Callback> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(callback) = self.handle(stream, expected_state, err, localizer) {
                        return Ok(callback);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        bail!(crate::message!(
                            "auth-callback-timeout",
                            minutes = timeout.as_secs().div_ceil(60)
                        ));
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(e) => return Err(e).lcontext(crate::message!("auth-callback-accept-failed")),
            }
        }
    }

    fn handle(
        &self,
        mut stream: TcpStream,
        expected_state: &str,
        err: &mut dyn Write,
        localizer: &Localizer,
    ) -> Option<Callback> {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(CONNECTION_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CONNECTION_TIMEOUT));

        let request_line = match read_request_line(&mut stream) {
            Ok(line) => line,
            Err(_) => {
                respond(
                    &mut stream,
                    400,
                    "Bad Request",
                    &page(
                        localizer,
                        &crate::tr!(localizer, "auth-callback-invalid-request"),
                        "",
                    ),
                );
                return None;
            }
        };
        let Some((method, target)) = parse_request_line(&request_line) else {
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page(
                    localizer,
                    &crate::tr!(localizer, "auth-callback-invalid-request"),
                    "",
                ),
            );
            return None;
        };
        if method != "GET" {
            respond(
                &mut stream,
                405,
                "Method Not Allowed",
                &page(
                    localizer,
                    &crate::tr!(localizer, "auth-callback-method-not-allowed"),
                    "",
                ),
            );
            return None;
        }
        let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page(
                    localizer,
                    &crate::tr!(localizer, "auth-callback-invalid-request"),
                    "",
                ),
            );
            return None;
        };
        if url.path() != self.path {
            respond(
                &mut stream,
                404,
                "Not Found",
                &page(
                    localizer,
                    &crate::tr!(localizer, "auth-callback-not-found"),
                    "",
                ),
            );
            return None;
        }
        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
        if params.get("state").map(String::as_str) != Some(expected_state) {
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page(
                    localizer,
                    &crate::tr!(localizer, "auth-callback-unverified-request"),
                    &crate::tr!(localizer, "auth-callback-state-mismatch-detail"),
                ),
            );
            let _ = writeln!(
                err,
                "{}",
                crate::tr!(localizer, "auth-callback-state-mismatch")
            );
            return None;
        }
        if let Some(error) = params.get("error") {
            let description = params
                .get("error_description")
                .filter(|d| !d.is_empty())
                .cloned();
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page(
                    localizer,
                    &crate::tr!(localizer, "auth-callback-denied-title"),
                    &crate::tr!(localizer, "auth-callback-denied-detail"),
                ),
            );
            return Some(Callback::Denied {
                error: error.clone(),
                description,
            });
        }
        match params.get("code").filter(|code| !code.is_empty()) {
            Some(code) => {
                respond(
                    &mut stream,
                    200,
                    "OK",
                    &page(
                        localizer,
                        &crate::tr!(localizer, "auth-callback-complete-title"),
                        &crate::tr!(localizer, "auth-callback-complete-detail"),
                    ),
                );
                Some(Callback::Code(code.clone()))
            }
            None => {
                respond(
                    &mut stream,
                    400,
                    "Bad Request",
                    &page(
                        localizer,
                        &crate::tr!(localizer, "auth-callback-unverified-request"),
                        &crate::tr!(localizer, "auth-callback-missing-code"),
                    ),
                );
                None
            }
        }
    }
}

fn read_request_line(stream: &mut TcpStream) -> anyhow::Result<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let line_end = loop {
        if let Some(pos) = find(&buf, b"\r\n") {
            break pos;
        }
        if buf.len() >= REQUEST_LINE_LIMIT {
            bail!(crate::message!("auth-callback-request-line-too-long"));
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            bail!(crate::message!("auth-callback-connection-closed"));
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let line = String::from_utf8_lossy(&buf[..line_end]).into_owned();
    while find(&buf, b"\r\n\r\n").is_none() && buf.len() < HEADER_LIMIT {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    Ok(line)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split(' ');
    let method = parts.next()?;
    let target = parts.next()?;
    if method.is_empty() || !target.starts_with('/') {
        return None;
    }
    Some((method.to_string(), target.to_string()))
}

fn respond(stream: &mut TcpStream, status: u16, reason: &str, html: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

fn page(localizer: &Localizer, title: &str, detail: &str) -> String {
    let lang = localizer.lang();
    format!(
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><title>{title} - PortOne CLI</title><style>body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:560px;margin:80px auto;padding:0 24px;color:#222}}h1{{font-size:20px}}p{{color:#555}}</style></head><body><h1>{title}</h1><p>{detail}</p></body></html>"
    )
}
