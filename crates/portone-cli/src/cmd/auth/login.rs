use std::io::Write;
use std::time::Duration;

use anyhow::anyhow;
use clap::Args;

use crate::auth::callback::{Callback, CallbackServer};
use crate::auth::oauth::{self, OAuthConfig};
use crate::auth::store::{self, KEYRING_SERVICE, SecretStore, StoreError};
use crate::auth::store_discovery::{self, StoreSummary};
use crate::auth::{self, browser};
use crate::config::{Config, OAuthProfile, OAuthTokens, Storage};
use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::{LocalizedErrorContext, Localizer};

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to store")]
    pub profile: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Base URL for API requests (default: https://api.portone.io)"
    )]
    pub base_url: Option<String>,

    #[arg(
        long,
        value_name = "SCOPES",
        value_delimiter = ',',
        help = "Comma-separated console scopes to request (default: HOME_AND_REPORT,TX_READ,CHANNEL_READ,STORE_READ,MERCHANT_READ)"
    )]
    pub scopes: Option<Vec<String>>,

    #[arg(
        long,
        help = "Store tokens in the config file instead of the OS keyring"
    )]
    pub insecure_storage: bool,

    #[arg(long, help = "Print the login URL without opening a browser")]
    pub no_browser: bool,
}

pub fn run(f: &mut Factory, args: LoginArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    if let Some(name) = auth::active_env_credential() {
        let _ = writeln!(
            f.io.err,
            "{}",
            crate::tr!(localizer, "auth-login-env-active", name = name)
        );
        return Err(CliError::Silent);
    }
    let mut config = f.config()?.clone();
    let profile_name = args
        .profile
        .clone()
        .unwrap_or_else(|| "default".to_string());
    login_oauth(f, &mut config, &profile_name, &args)
}

fn login_oauth(
    f: &mut Factory,
    config: &mut Config,
    profile_name: &str,
    args: &LoginArgs,
) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    let cfg = OAuthConfig::from_env(args.scopes.clone())?;
    let server = CallbackServer::bind(&cfg.redirect_uri)?;
    let pkce = oauth::generate_pkce()?;
    let state = oauth::generate_state()?;
    let url = oauth::authorize_url(&cfg, &pkce, &state);

    let _ = writeln!(
        f.io.err,
        "{}\n  {url}",
        crate::tr!(localizer, "auth-login-browser-instructions")
    );
    if !args.no_browser
        && let Err(err) = browser::open_localized(url.as_str(), &localizer)
    {
        let _ = writeln!(
            f.io.err,
            "{}",
            crate::tr!(
                localizer,
                "auth-login-browser-failed",
                error = err.to_string()
            )
        );
    }

    let code = match server.wait_localized(&state, CALLBACK_TIMEOUT, &mut *f.io.err, &localizer)? {
        Callback::Code(code) => code,
        Callback::Denied { error, description } => {
            let detail = description.map(|d| format!(" ({d})")).unwrap_or_default();
            return Err(CliError::Other(anyhow!(crate::message!(
                "auth-login-denied",
                error = error,
                detail = detail
            ))));
        }
    };
    drop(server);

    let agent = f.agent();
    let tokens = oauth::exchange_code(&agent, &cfg, &code, &pkce.verifier).map_err(|err| {
        CliError::Other(
            err.into_anyhow()
                .lcontext(crate::message!("auth-login-token-failed")),
        )
    })?;
    let missing = oauth::missing_scopes(&cfg.scopes, &tokens.scope);
    if !missing.is_empty() {
        let _ = writeln!(
            f.io.err,
            "{}",
            crate::tr!(
                localizer,
                "auth-login-missing-scopes",
                scopes = missing.join(", ")
            )
        );
    }

    let base_url = auth::resolve_base_url(args.base_url.as_deref(), Some(profile_name), config);
    let plain_id =
        auth::verify_bearer(&agent, &base_url, &tokens.access_token)?.ok_or_else(|| {
            CliError::Other(anyhow!(crate::message!(
                "auth-login-environment-mismatch",
                base_url = base_url
            )))
        })?;

    let previous_store = config
        .profiles
        .get(profile_name)
        .and_then(|profile| profile.store_id.as_deref());
    let selected_store = select_login_store(f, &base_url, &tokens.access_token, previous_store);
    config
        .profiles
        .entry(profile_name.to_string())
        .or_default()
        .store_id = selected_store.as_ref().map(|store| store.plain_id.clone());

    let store = f.secret_store();
    let stored = store_web_login(
        store.as_ref(),
        config,
        profile_name,
        &cfg,
        &tokens,
        base_url,
        args.insecure_storage,
        &mut *f.io.err,
        &localizer,
    )?;

    let _ = writeln!(
        f.io.err,
        "{}",
        crate::tr!(localizer, "auth-login-complete", merchant = plain_id)
    );
    let _ = writeln!(
        f.io.err,
        "{}",
        crate::tr!(localizer, "auth-login-stored", profile = profile_name)
    );
    let store_message = match selected_store {
        Some(store) => crate::tr!(
            localizer,
            "auth-login-store-selected",
            store = store.label(&localizer)
        ),
        None => crate::tr!(localizer, "auth-login-store-unset"),
    };
    let _ = writeln!(f.io.err, "{store_message}");
    let location = match (stored.storage, stored.credential_id.as_deref()) {
        (Storage::Keyring, Some(id)) => crate::tr!(
            localizer,
            "auth-source-keyring",
            service = KEYRING_SERVICE,
            id = id
        ),
        _ => crate::tr!(localizer, "auth-storage-file"),
    };
    let _ = writeln!(
        f.io.err,
        "{}",
        crate::tr!(localizer, "auth-storage", location = location)
    );
    Ok(())
}

fn select_login_store(
    f: &mut Factory,
    base_url: &str,
    access_token: &str,
    previous: Option<&str>,
) -> Option<StoreSummary> {
    let localizer = f.localizer.clone();
    let result = store_discovery::discover(&f.agent(), base_url, access_token)
        .map_err(CliError::from)
        .and_then(|stores| {
            if let Some(store) = store_discovery::preferred_store(&stores, previous) {
                return Ok(Some(store.clone()));
            }
            if !stores.is_empty() && f.io.can_prompt() {
                return store_discovery::pick_store(&stores, previous, true, &localizer);
            }
            Ok(None)
        });
    match result {
        Ok(store) => store,
        Err(error) => {
            let error = match error {
                CliError::Other(error) => localizer.format_error(&error),
                CliError::Flag(error) => error,
                CliError::Silent => return None,
            };
            let _ = writeln!(
                f.io.err,
                "{}",
                crate::tr!(localizer, "auth-login-store-unavailable", error = error)
            );
            None
        }
    }
}

pub(crate) struct StoredLogin {
    pub storage: Storage,
    pub credential_id: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn store_web_login(
    store: &dyn SecretStore,
    config: &mut Config,
    profile_name: &str,
    cfg: &OAuthConfig,
    tokens: &OAuthTokens,
    base_url: String,
    insecure_storage: bool,
    err: &mut dyn Write,
    localizer: &Localizer,
) -> Result<StoredLogin, CliError> {
    let previous = config
        .profiles
        .get(profile_name)
        .and_then(|p| p.oauth.clone());
    let mut oauth = OAuthProfile {
        storage: Storage::Keyring,
        client_id: cfg.client_id.clone(),
        token_url: cfg.token_url(),
        console_url: cfg.console_url.clone(),
        credential_id: None,
        tokens: None,
    };
    if insecure_storage {
        oauth.storage = Storage::File;
        oauth.tokens = Some(tokens.clone());
    } else {
        let id = store::new_credential_id()?;
        match store.save(&id, tokens) {
            Ok(()) => oauth.credential_id = Some(id),
            Err(StoreError::Timeout) => {
                return Err(CliError::Other(anyhow!(crate::message!(
                    "auth-login-keyring-timeout",
                    service = KEYRING_SERVICE,
                    id = id
                ))));
            }
            Err(error) => {
                let _ = writeln!(
                    err,
                    "{}",
                    crate::tr!(
                        localizer,
                        "auth-login-keyring-fallback",
                        error = error.localized(localizer)
                    )
                );
                oauth.storage = Storage::File;
                oauth.tokens = Some(tokens.clone());
            }
        }
    }

    let entry = config.profiles.entry(profile_name.to_string()).or_default();
    entry.base_url = Some(base_url);
    entry.oauth = Some(oauth.clone());
    if config.default_profile.is_none() {
        config.default_profile = Some(profile_name.to_string());
    }
    if let Err(error) = config.save() {
        if let Some(id) = &oauth.credential_id {
            let _ = store.delete(id);
        }
        return Err(error.into());
    }
    if let Some(previous) = previous {
        delete_previous_entry(
            store,
            &previous,
            oauth.credential_id.as_deref(),
            err,
            localizer,
        );
    }
    Ok(StoredLogin {
        storage: oauth.storage,
        credential_id: oauth.credential_id,
    })
}

fn delete_previous_entry(
    store: &dyn SecretStore,
    previous: &OAuthProfile,
    keep: Option<&str>,
    err: &mut dyn Write,
    localizer: &Localizer,
) {
    let Some(id) = auth::normalize(previous.credential_id.as_deref()) else {
        return;
    };
    if keep == Some(id.as_str()) {
        return;
    }
    if let Err(error) = store.delete(&id) {
        let _ = writeln!(
            err,
            "{}",
            crate::tr!(
                localizer,
                "auth-login-cleanup-failed",
                service = KEYRING_SERVICE,
                id = id,
                error = error.localized(localizer)
            )
        );
    }
}
