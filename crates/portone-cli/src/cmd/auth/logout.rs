use std::io::Write;

use anyhow::anyhow;
use clap::Args;

use crate::auth::{self, store::KEYRING_SERVICE};
use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::LocalizedErrorContext;

#[derive(Debug, Args)]
pub struct LogoutArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to remove")]
    pub profile: Option<String>,
}

pub fn run(f: &mut Factory, args: LogoutArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    if let Some(name) = auth::active_env_credential() {
        let _ = writeln!(
            f.io.err,
            "{}",
            crate::tr!(localizer, "auth-logout-env-active", name = name)
        );
        return Err(CliError::Silent);
    }
    let mut config = f.config()?.clone();
    let name = args
        .profile
        .or_else(|| config.default_profile.clone())
        .unwrap_or_else(|| "default".to_string());
    let Some(profile) = config.profiles.get(&name) else {
        return Err(CliError::Other(anyhow!(crate::message!(
            "auth-profile-not-found",
            profile = name
        ))));
    };
    if let Some(oauth) = &profile.oauth
        && let Some(id) = auth::normalize(oauth.credential_id.as_deref())
    {
        f.secret_store().delete(&id).map_err(|err| {
            CliError::Other(err.into_anyhow().lcontext(crate::message!(
                "auth-logout-keyring-delete-failed",
                service = KEYRING_SERVICE,
                id = id
            )))
        })?;
    }
    config.profiles.remove(&name);
    if config.default_profile.as_deref() == Some(name.as_str()) {
        config.default_profile = None;
    }
    config.save()?;
    let _ = writeln!(
        f.io.err,
        "{}",
        crate::tr!(localizer, "auth-logout-removed", profile = name)
    );
    Ok(())
}
