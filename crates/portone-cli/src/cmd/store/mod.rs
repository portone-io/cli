use std::io::Write;

use anyhow::anyhow;
use clap::{Args, Subcommand};

use crate::auth::{self, store, store_discovery};
use crate::config::Config;
use crate::error::CliError;
use crate::factory::Factory;
use crate::output::resource::cell;

#[derive(Debug, Args)]
pub struct StoreArgs {
    #[command(subcommand)]
    pub command: StoreCommand,
}

#[derive(Debug, Subcommand)]
pub enum StoreCommand {
    #[command(about = "Set the default store for a configuration profile")]
    SetDefault(SetDefaultArgs),
}

#[derive(Debug, Args)]
pub struct SetDefaultArgs {
    #[arg(value_name = "ID", help = "Store ID to save as the default")]
    pub id: Option<String>,

    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,

    #[arg(long, conflicts_with_all = ["id", "unset"], help = "Print the saved default store ID")]
    pub view: bool,

    #[arg(long, conflicts_with_all = ["id", "view"], help = "Remove the saved default store")]
    pub unset: bool,
}

pub fn run(f: &mut Factory, args: StoreArgs) -> Result<(), CliError> {
    match args.command {
        StoreCommand::SetDefault(args) => set_default(f, args),
    }
}

fn set_default(f: &mut Factory, args: SetDefaultArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    let mut config = f.config()?.clone();
    let profile = auth::profile_name(args.profile.as_deref(), &config);
    if args.view {
        let id = config
            .profiles
            .get(&profile)
            .and_then(|profile| profile.store_id.as_deref())
            .ok_or_else(|| {
                anyhow!(crate::message!(
                    "store-default-missing",
                    profile = cell(&profile)
                ))
            })?;
        validate_id(id)?;
        writeln!(f.io.out, "{id}")?;
        return Ok(());
    }
    if args.unset {
        save_default(&profile, None)?;
        writeln!(
            f.io.err,
            "{}",
            crate::tr!(localizer, "store-default-unset", profile = cell(&profile))
        )?;
        return Ok(());
    }

    let id = if let Some(id) = args.id {
        validate_id(&id)?;
        let id = id.trim();
        id.to_string()
    } else {
        if !f.io.can_prompt() {
            return Err(CliError::Other(anyhow!(crate::message!(
                "store-selection-requires-tty"
            ))));
        }
        let agent = f.agent();
        let secret_store = f.secret_store();
        let resolved = auth::resolve_fresh_localized(
            &agent,
            secret_store.as_ref(),
            &mut config,
            Some(&profile),
            &mut *f.io.err,
            &localizer,
        )?
        .ok_or_else(|| anyhow!(crate::message!("auth-no-credentials")))?;
        let base_url = auth::resolve_base_url(None, Some(&profile), &config);
        let stores = store_discovery::discover(&agent, &base_url, &resolved.access_token)?;
        let previous = config
            .profiles
            .get(&profile)
            .and_then(|profile| profile.store_id.as_deref());
        let Some(selected) = store_discovery::pick_store(&stores, previous, false, &localizer)?
        else {
            return Ok(());
        };
        selected.plain_id
    };
    save_default(&profile, Some(&id))?;
    writeln!(
        f.io.err,
        "{}",
        crate::tr!(
            localizer,
            "store-default-set",
            store = cell(&id),
            profile = cell(&profile)
        )
    )?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), CliError> {
    if !store_discovery::valid_store_id(id) {
        return Err(CliError::Other(anyhow!(crate::message!(
            "store-invalid-id"
        ))));
    }
    Ok(())
}

fn save_default(profile: &str, id: Option<&str>) -> anyhow::Result<()> {
    let _lock = store::lock_config()?;
    let mut config = Config::load()?;
    if let Some(id) = id {
        config
            .profiles
            .entry(profile.to_string())
            .or_default()
            .store_id = Some(id.to_string());
    } else if let Some(profile) = config.profiles.get_mut(profile) {
        profile.store_id = None;
    }
    config.save()
}
