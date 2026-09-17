use std::process::{Command, Stdio};

use url::Url;

use crate::i18n::Localizer;

pub fn open(url: &str) -> std::io::Result<()> {
    open_localized(url, &Localizer::english())
}

pub fn open_localized(url: &str, localizer: &Localizer) -> std::io::Result<()> {
    let parsed = Url::parse(url).map_err(|err| {
        invalid(crate::tr!(
            localizer,
            "auth-browser-invalid-url",
            url = url,
            error = err.to_string()
        ))
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid(crate::tr!(
            localizer,
            "auth-browser-unsupported-url",
            url = url
        )));
    }
    let (program, args) = launcher();
    Command::new(program)
        .args(args)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn launcher() -> (String, Vec<String>) {
    for name in ["PORTONE_BROWSER", "BROWSER"] {
        if let Some(command) = std::env::var(name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            let mut words = command.split_whitespace().map(str::to_string);
            if let Some(program) = words.next() {
                return (program, words.collect());
            }
        }
    }
    platform_launcher()
}

#[cfg(target_os = "macos")]
fn platform_launcher() -> (String, Vec<String>) {
    ("open".to_string(), Vec::new())
}

#[cfg(target_os = "windows")]
fn platform_launcher() -> (String, Vec<String>) {
    (
        "rundll32".to_string(),
        vec!["url.dll,FileProtocolHandler".to_string()],
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_launcher() -> (String, Vec<String>) {
    ("xdg-open".to_string(), Vec::new())
}

fn invalid(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}
