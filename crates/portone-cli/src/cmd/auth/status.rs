use std::io::Write;
use std::time::{Duration, UNIX_EPOCH};

use clap::Args;

use crate::auth::store::KEYRING_SERVICE;
use crate::auth::{self, AuthSource, oauth};
use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::Localizer;

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,

    #[arg(long, help = "Display the access token without masking it")]
    pub show_secret: bool,
}

pub fn run(f: &mut Factory, args: StatusArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    let mut config = f.config()?.clone();
    let agent = f.agent();
    let store = f.secret_store();
    let resolved = auth::resolve_fresh_localized(
        &agent,
        store.as_ref(),
        &mut config,
        args.profile.as_deref(),
        &mut *f.io.err,
        &localizer,
    )?;
    let Some(resolved) = resolved else {
        let _ = writeln!(f.io.err, "{}", crate::tr!(localizer, "auth-no-credentials"));
        return Err(CliError::Silent);
    };
    let base = auth::resolve_base_url(None, args.profile.as_deref(), &config);

    let source = match &resolved.source {
        AuthSource::Env(name) => crate::tr!(localizer, "auth-source-environment", name = *name),
        AuthSource::ConfigProfile(name) => {
            crate::tr!(localizer, "auth-source-config", profile = name)
        }
        AuthSource::Keyring(id) => crate::tr!(
            localizer,
            "auth-source-keyring",
            service = KEYRING_SERVICE,
            id = id
        ),
    };

    let token = &resolved.access_token;
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(localizer, "auth-status-authentication")
    )?;
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(localizer, "auth-status-source", source = source)
    )?;
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(
            localizer,
            "auth-status-access-token",
            token = mask_secret(token, args.show_secret)
        )
    )?;
    if let Some(session) = &resolved.oauth {
        let now = oauth::now();
        writeln!(
            f.io.out,
            "{}",
            crate::tr!(
                localizer,
                "auth-status-expires",
                timestamp = rfc3339(session.tokens.expires_at),
                remaining = remaining(session.tokens.expires_at, now, &localizer)
            )
        )?;
        if let Some(exp) = session
            .tokens
            .refresh_token
            .as_deref()
            .and_then(oauth::jwt_exp)
        {
            writeln!(
                f.io.out,
                "{}",
                crate::tr!(
                    localizer,
                    "auth-status-session-expires",
                    timestamp = rfc3339(exp)
                )
            )?;
        }
        if !session.tokens.scope.is_empty() {
            writeln!(
                f.io.out,
                "{}",
                crate::tr!(
                    localizer,
                    "auth-status-scopes",
                    scopes = session.tokens.scope.join(", ")
                )
            )?;
        }
        writeln!(
            f.io.out,
            "{}",
            crate::tr!(
                localizer,
                "auth-status-issued-by",
                client_id = &session.oauth.client_id,
                url = &session.oauth.console_url
            )
        )?;
    }
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(localizer, "auth-status-api-base-url", url = &base)
    )?;

    match auth::verify_bearer(&agent, &base, token)? {
        Some(plain_id) => {
            writeln!(
                f.io.out,
                "{}",
                crate::tr!(localizer, "auth-status-valid", merchant = plain_id)
            )?;
            Ok(())
        }
        None => {
            writeln!(f.io.out, "{}", crate::tr!(localizer, "auth-status-invalid"))?;
            let _ = writeln!(
                f.io.err,
                "{}",
                crate::tr!(localizer, "auth-status-invalid-token")
            );
            Err(CliError::Silent)
        }
    }
}

fn mask_secret(secret: &str, show: bool) -> String {
    if show {
        return secret.to_string();
    }
    let prefix: String = secret.chars().take(4).collect();
    format!("{prefix}****")
}

fn rfc3339(secs: u64) -> String {
    humantime::format_rfc3339_seconds(UNIX_EPOCH + Duration::from_secs(secs)).to_string()
}

fn remaining(expires_at: u64, now: u64, localizer: &Localizer) -> String {
    if expires_at <= now {
        return crate::tr!(localizer, "auth-remaining-expired");
    }
    let secs = expires_at - now;
    if secs >= 3600 {
        crate::tr!(
            localizer,
            "auth-remaining-hours",
            hours = secs / 3600,
            minutes = (secs % 3600) / 60
        )
    } else if secs >= 60 {
        crate::tr!(localizer, "auth-remaining-minutes", minutes = secs / 60)
    } else {
        crate::tr!(localizer, "auth-remaining-seconds", seconds = secs)
    }
}
