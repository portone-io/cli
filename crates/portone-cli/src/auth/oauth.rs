use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use base64::Engine;
use base64::engine::general_purpose::{URL_SAFE_NO_PAD, URL_SAFE_NO_PAD_INDIFFERENT};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::OAuthTokens;
use crate::i18n::{LocalizedContext, LocalizedErrorContext, Localizer};

pub const CONSOLE_URL: &str = "https://admin.portone.io";
pub const MERCHANT_SERVICE_URL: &str = "https://merchant-service.prod.iamport.co";
pub const CLIENT_ID: &str = "CLI";
pub const REDIRECT_URI: &str = "http://127.0.0.1:1271/oauth/cli";
pub const DEFAULT_SCOPES: &[&str] = &[
    "HOME_AND_REPORT",
    "TX_READ",
    "CHANNEL_READ",
    "STORE_READ",
    "MERCHANT_READ",
];
pub const REFRESH_MARGIN_SECS: u64 = 60;
const TOKEN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthConfig {
    pub console_url: String,
    pub merchant_service_url: String,
    pub client_id: String,
    pub redirect_uri: Url,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthIssuer {
    pub client_id: String,
    pub token_url: String,
    pub console_url: String,
}

impl OAuthConfig {
    pub fn from_env(scopes: Option<Vec<String>>) -> anyhow::Result<Self> {
        let console_url = base_url_from_env("PORTONE_CONSOLE_URL", CONSOLE_URL)?;
        let merchant_service_url =
            base_url_from_env("PORTONE_MERCHANT_SERVICE_URL", MERCHANT_SERVICE_URL)?;
        let client_id = env_or("PORTONE_OAUTH_CLIENT_ID", CLIENT_ID);
        let redirect = env_or("PORTONE_OAUTH_REDIRECT_URI", REDIRECT_URI);
        let redirect_uri = Url::parse(&redirect)
            .with_lcontext(|| crate::message!("auth-invalid-redirect-uri", uri = redirect))?;
        let scopes = match scopes {
            Some(scopes) if !scopes.is_empty() => scopes,
            _ => DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        };
        Ok(Self {
            console_url,
            merchant_service_url,
            client_id,
            redirect_uri,
            scopes,
        })
    }

    pub fn token_url(&self) -> String {
        format!("{}/oauth/token", self.merchant_service_url)
    }

    pub fn issuer(&self) -> OAuthIssuer {
        OAuthIssuer {
            client_id: self.client_id.clone(),
            token_url: self.token_url(),
            console_url: self.console_url.clone(),
        }
    }
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn base_url_from_env(name: &str, default: &str) -> anyhow::Result<String> {
    let value = env_or(name, default);
    let parsed = Url::parse(&value)
        .with_lcontext(|| crate::message!("auth-invalid-env-url", name = name, value = value))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!(crate::message!(
            "auth-invalid-env-url-scheme",
            name = name,
            value = value
        ));
    }
    Ok(value.trim_end_matches('/').to_string())
}

#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

pub fn generate_pkce() -> anyhow::Result<Pkce> {
    let verifier = random_base64url(32)?;
    let challenge = code_challenge(&verifier);
    Ok(Pkce {
        verifier,
        challenge,
    })
}

pub fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn generate_state() -> anyhow::Result<String> {
    random_base64url(16)
}

pub fn random_base64url(len: usize) -> anyhow::Result<String> {
    let mut buf = vec![0u8; len];
    getrandom::getrandom(&mut buf)
        .map_err(|err| anyhow!(crate::message!("auth-random-failed", error = err)))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

pub fn authorize_url(cfg: &OAuthConfig, pkce: &Pkce, state: &str) -> Url {
    let mut url = Url::parse(&format!("{}/oauth/authorize", cfg.console_url))
        .expect("console_url was validated by from_env");
    url.query_pairs_mut()
        .append_pair("client_id", &cfg.client_id)
        .append_pair("redirect_uri", cfg.redirect_uri.as_str())
        .append_pair("response_type", "code")
        .append_pair("scope", &cfg.scopes.join(" "))
        .append_pair("state", state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256");
    url
}

#[derive(Debug)]
pub enum TokenError {
    InvalidGrant(String),
    Rejected { error: String, detail: String },
    Transient(anyhow::Error),
    Malformed(anyhow::Error),
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::InvalidGrant(detail) if detail.is_empty() => write!(f, "invalid_grant"),
            TokenError::InvalidGrant(detail) => write!(f, "invalid_grant: {detail}"),
            TokenError::Rejected { error, detail } if detail.is_empty() => write!(f, "{error}"),
            TokenError::Rejected { error, detail } => write!(f, "{error}: {detail}"),
            TokenError::Transient(err) | TokenError::Malformed(err) => write!(f, "{err:#}"),
        }
    }
}

impl std::error::Error for TokenError {}

impl TokenError {
    pub fn localized(&self, localizer: &Localizer) -> String {
        match self {
            Self::Transient(error) | Self::Malformed(error) => localizer.format_error(error),
            _ => self.to_string(),
        }
    }

    pub fn into_anyhow(self) -> anyhow::Error {
        match self {
            Self::Transient(error) | Self::Malformed(error) => error,
            other => anyhow!(other),
        }
    }
}

pub fn exchange_code(
    agent: &ureq::Agent,
    cfg: &OAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<OAuthTokens, TokenError> {
    token_request(
        agent,
        &cfg.token_url(),
        json!({
            "client_id": cfg.client_id,
            "grant_type": "authorization_code",
            "code": code,
            "code_verifier": verifier,
        }),
        None,
    )
}

pub fn refresh(
    agent: &ureq::Agent,
    issuer: &OAuthIssuer,
    refresh_token: &str,
) -> Result<OAuthTokens, TokenError> {
    token_request(
        agent,
        &issuer.token_url,
        json!({
            "client_id": issuer.client_id,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }),
        Some(refresh_token),
    )
}

fn token_request(
    agent: &ureq::Agent,
    url: &str,
    body: Value,
    previous_refresh: Option<&str>,
) -> Result<OAuthTokens, TokenError> {
    let mut response = agent
        .post(url)
        .config()
        .timeout_global(Some(TOKEN_TIMEOUT))
        .build()
        .send_json(body)
        .map_err(|err| {
            TokenError::Transient(
                anyhow!(err).lcontext(crate::message!("auth-token-request-failed")),
            )
        })?;
    let status = response.status().as_u16();
    let bytes = response.body_mut().read_to_vec().map_err(|err| {
        TokenError::Transient(anyhow!(err).lcontext(crate::message!("auth-token-read-failed")))
    })?;
    classify_response(status, &bytes, now(), previous_refresh)
}

pub(crate) fn classify_response(
    status: u16,
    body: &[u8],
    now: u64,
    previous_refresh: Option<&str>,
) -> Result<OAuthTokens, TokenError> {
    if (200..300).contains(&status) {
        return parse_token_response(body, now, previous_refresh);
    }
    if status >= 500 {
        return Err(TokenError::Transient(anyhow!("HTTP {status}")));
    }
    let (error, detail) =
        parse_error_body(body).unwrap_or_else(|| (format!("HTTP {status}"), String::new()));
    if error == "invalid_grant" {
        Err(TokenError::InvalidGrant(detail))
    } else {
        Err(TokenError::Rejected { error, detail })
    }
}

fn parse_error_body(body: &[u8]) -> Option<(String, String)> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let error = value.get("error")?.as_str()?.to_string();
    let detail = value
        .get("detail")
        .or_else(|| value.get("error_description"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((error, detail))
}

fn parse_token_response(
    body: &[u8],
    now: u64,
    previous_refresh: Option<&str>,
) -> Result<OAuthTokens, TokenError> {
    let value: Value = serde_json::from_slice(body).map_err(|err| {
        TokenError::Malformed(anyhow!(err).lcontext(crate::message!("auth-token-parse-failed")))
    })?;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            TokenError::Malformed(anyhow!(crate::message!("auth-token-missing-access-token")))
        })?
        .to_string();
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            TokenError::Malformed(anyhow!(crate::message!("auth-token-missing-expires-in")))
        })?;
    let scope = match value.get("scope") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(text)) => text.split_whitespace().map(str::to_string).collect(),
        _ => Vec::new(),
    };
    let token_type = value
        .get("token_type")
        .and_then(Value::as_str)
        .unwrap_or("Bearer")
        .to_string();
    let refresh_token = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| previous_refresh.map(str::to_string));
    Ok(OAuthTokens {
        access_token,
        refresh_token,
        expires_at: now.saturating_add(expires_in),
        scope,
        token_type,
    })
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn needs_refresh(tokens: &OAuthTokens, now: u64) -> bool {
    tokens.expires_at <= now.saturating_add(REFRESH_MARGIN_SECS)
}

pub fn is_valid(tokens: &OAuthTokens, now: u64) -> bool {
    tokens.expires_at > now
}

pub fn jwt_exp(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD_INDIFFERENT.decode(payload).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value.get("exp")?.as_u64()
}

pub fn missing_scopes(requested: &[String], granted: &[String]) -> Vec<String> {
    requested
        .iter()
        .filter(|scope| !granted.contains(scope))
        .cloned()
        .collect()
}
