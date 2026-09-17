pub mod browser;
pub mod callback;
pub mod oauth;
pub mod store;
pub mod store_discovery;

use std::io::Write;

use anyhow::anyhow;
use serde_json::Value;

use crate::config::{Config, DEFAULT_BASE_URL, OAuthProfile, OAuthTokens, Storage};
use crate::error::CliError;
use crate::i18n::{LocalizedContext, LocalizedErrorContext, Localizer};
use oauth::{OAuthIssuer, TokenError};
use store::{KEYRING_SERVICE, SecretStore};

pub const ACCESS_TOKEN_ENV: &str = "PORTONE_ACCESS_TOKEN";
pub const SESSION_EXPIRED: &str =
    "console login session has expired; run `portone auth login` to authenticate again";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthSource {
    Env(&'static str),
    ConfigProfile(String),
    Keyring(String),
}

#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub profile: String,
    pub oauth: OAuthProfile,
    pub tokens: OAuthTokens,
}

impl OAuthSession {
    pub fn issuer(&self) -> OAuthIssuer {
        OAuthIssuer {
            client_id: self.oauth.client_id.clone(),
            token_url: self.oauth.token_url.clone(),
            console_url: self.oauth.console_url.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub access_token: String,
    pub source: AuthSource,
    pub oauth: Option<OAuthSession>,
}

impl ResolvedAuth {
    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

pub fn active_env_credential() -> Option<&'static str> {
    env_value(ACCESS_TOKEN_ENV).map(|_| ACCESS_TOKEN_ENV)
}

pub fn resolve_static() -> Option<ResolvedAuth> {
    if let Some(token) = env_value(ACCESS_TOKEN_ENV) {
        return Some(ResolvedAuth {
            access_token: token,
            source: AuthSource::Env(ACCESS_TOKEN_ENV),
            oauth: None,
        });
    }
    None
}

pub fn resolve(
    store: &dyn SecretStore,
    profile: Option<&str>,
    config: &Config,
) -> anyhow::Result<Option<ResolvedAuth>> {
    if let Some(resolved) = resolve_static() {
        return Ok(Some(resolved));
    }
    let name = profile_name(profile, config);
    let Some(oauth) = config.profiles.get(&name).and_then(|p| p.oauth.as_ref()) else {
        return Ok(None);
    };
    let Some(tokens) = load_tokens(store, &name, oauth)? else {
        return Ok(None);
    };
    let source = match oauth.storage {
        Storage::Keyring => AuthSource::Keyring(oauth.credential_id.clone().unwrap_or_default()),
        Storage::File => AuthSource::ConfigProfile(name.clone()),
    };
    Ok(Some(ResolvedAuth {
        access_token: tokens.access_token.clone(),
        source,
        oauth: Some(OAuthSession {
            profile: name,
            oauth: oauth.clone(),
            tokens,
        }),
    }))
}

pub fn load_tokens(
    store: &dyn SecretStore,
    profile: &str,
    oauth: &OAuthProfile,
) -> anyhow::Result<Option<OAuthTokens>> {
    match oauth.storage {
        Storage::File => Ok(oauth.tokens.clone()),
        Storage::Keyring => {
            let Some(id) = normalize(oauth.credential_id.as_deref()) else {
                return Ok(None);
            };
            store.load(&id).map_err(|err| {
                err.into_anyhow().lcontext(crate::message!(
                    "auth-keyring-load-failed",
                    profile = profile,
                    service = KEYRING_SERVICE,
                    id = id
                ))
            })
        }
    }
}

pub fn resolve_fresh(
    agent: &ureq::Agent,
    store: &dyn SecretStore,
    config: &mut Config,
    profile: Option<&str>,
    err: &mut dyn Write,
) -> Result<Option<ResolvedAuth>, CliError> {
    resolve_fresh_localized(agent, store, config, profile, err, &Localizer::english())
}

pub fn resolve_fresh_localized(
    agent: &ureq::Agent,
    store: &dyn SecretStore,
    config: &mut Config,
    profile: Option<&str>,
    err: &mut dyn Write,
    localizer: &Localizer,
) -> Result<Option<ResolvedAuth>, CliError> {
    let Some(mut resolved) = resolve(store, profile, config)? else {
        return Ok(None);
    };
    let Some(mut session) = resolved.oauth.take() else {
        return Ok(Some(resolved));
    };
    let now = oauth::now();
    if !oauth::needs_refresh(&session.tokens, now) {
        return Ok(Some(finish(resolved, session)));
    }

    let lock_key = normalize(session.oauth.credential_id.as_deref())
        .unwrap_or_else(|| format!("profile-{}", session.profile));
    let _lock = store::lock_refresh(&lock_key)?;

    // A previous lock holder may have switched from keyring to file storage.
    *config = Config::load()?;
    let Some(oauth) = config
        .profiles
        .get(&session.profile)
        .and_then(|profile| profile.oauth.as_ref())
    else {
        return Ok(None);
    };
    session.oauth = oauth.clone();
    let Some(latest) = load_tokens(store, &session.profile, &session.oauth)? else {
        return Ok(None);
    };
    let changed = latest != session.tokens;
    session.tokens = latest;
    let now = oauth::now();
    if changed && oauth::is_valid(&session.tokens, now) {
        return Ok(Some(finish(resolved, session)));
    }

    let Some(refresh_token) = normalize(session.tokens.refresh_token.as_deref()) else {
        return Err(CliError::Flag(crate::tr!(
            localizer,
            "auth-session-expired"
        )));
    };
    match oauth::refresh(agent, &session.issuer(), &refresh_token) {
        Ok(tokens) => {
            persist_refreshed(store, config, &mut session, &tokens, err, localizer)?;
            session.tokens = tokens;
            Ok(Some(finish(resolved, session)))
        }
        Err(TokenError::InvalidGrant(_)) => Err(CliError::Flag(crate::tr!(
            localizer,
            "auth-session-expired"
        ))),
        Err(TokenError::Rejected { error, detail }) => Err(CliError::Other(anyhow!(
            crate::message!("auth-refresh-rejected", error = error, detail = detail)
        ))),
        Err(error) => {
            if oauth::is_valid(&session.tokens, now) {
                let _ = writeln!(
                    err,
                    "{}",
                    crate::tr!(
                        localizer,
                        "auth-refresh-failed-continuing",
                        error = error.localized(localizer)
                    )
                );
                Ok(Some(finish(resolved, session)))
            } else {
                Err(CliError::Other(
                    error
                        .into_anyhow()
                        .lcontext(crate::message!("auth-refresh-failed")),
                ))
            }
        }
    }
}

fn finish(mut resolved: ResolvedAuth, session: OAuthSession) -> ResolvedAuth {
    resolved.access_token = session.tokens.access_token.clone();
    resolved.source = match session.oauth.storage {
        Storage::Keyring => {
            AuthSource::Keyring(session.oauth.credential_id.clone().unwrap_or_default())
        }
        Storage::File => AuthSource::ConfigProfile(session.profile.clone()),
    };
    resolved.oauth = Some(session);
    resolved
}

fn persist_refreshed(
    store: &dyn SecretStore,
    config: &mut Config,
    session: &mut OAuthSession,
    tokens: &OAuthTokens,
    err: &mut dyn Write,
    localizer: &Localizer,
) -> Result<(), CliError> {
    if session.oauth.storage == Storage::Keyring
        && let Some(id) = normalize(session.oauth.credential_id.as_deref())
    {
        match store.save(&id, tokens) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let _ = writeln!(
                    err,
                    "{}",
                    crate::tr!(
                        localizer,
                        "auth-refreshed-keyring-fallback",
                        error = error.localized(localizer)
                    )
                );
            }
        }
    }
    // Credential locks do not serialize updates to different profiles in this file.
    let _config_lock = store::lock_config().map_err(save_error)?;
    let mut fresh = Config::load().map_err(save_error)?;
    let profile = fresh.profiles.entry(session.profile.clone()).or_default();
    let mut oauth = profile
        .oauth
        .clone()
        .unwrap_or_else(|| session.oauth.clone());
    oauth.storage = Storage::File;
    oauth.tokens = Some(tokens.clone());
    profile.oauth = Some(oauth.clone());
    fresh.save().map_err(save_error)?;
    session.oauth = oauth;
    *config = fresh;
    Ok(())
}

fn save_error(err: anyhow::Error) -> CliError {
    CliError::Other(err.lcontext(crate::message!("auth-refreshed-save-failed")))
}

pub fn resolve_base_url(flag: Option<&str>, profile: Option<&str>, config: &Config) -> String {
    if let Some(url) = normalize(flag) {
        return url;
    }
    if let Some(url) = env_value("PORTONE_API_BASE") {
        return url;
    }
    let name = profile_name(profile, config);
    if let Some(url) = config
        .profiles
        .get(&name)
        .and_then(|profile| normalize(profile.base_url.as_deref()))
    {
        return url;
    }
    DEFAULT_BASE_URL.to_string()
}

pub fn profile_name(profile: Option<&str>, config: &Config) -> String {
    profile
        .or(config.default_profile.as_deref())
        .unwrap_or("default")
        .to_string()
}

pub(crate) fn normalize(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn env_value(name: &str) -> Option<String> {
    normalize(std::env::var(name).ok().as_deref())
}

pub fn verify_bearer(
    agent: &ureq::Agent,
    base_url: &str,
    access_token: &str,
) -> anyhow::Result<Option<String>> {
    let url = format!("{}/graphql", base_url.trim_end_matches('/'));
    let mut response = agent
        .post(&url)
        .header("Authorization", &format!("Bearer {access_token}"))
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .send_json(serde_json::json!({
            "query": "query { merchant { __typename ... on Merchant { plainId } } }"
        }))
        .lcontext(crate::message!("auth-validation-request-failed"))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Ok(None);
    }
    let value: Value = response
        .body_mut()
        .read_json()
        .lcontext(crate::message!("auth-validation-parse-failed"))?;
    let merchant = value.pointer("/data/merchant");
    let is_merchant = merchant
        .and_then(|m| m.get("__typename"))
        .and_then(Value::as_str)
        == Some("Merchant");
    if !is_merchant {
        return Ok(None);
    }
    Ok(Some(
        merchant
            .and_then(|m| m.get("plainId"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    ))
}
