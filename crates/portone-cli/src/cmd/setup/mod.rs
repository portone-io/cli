pub mod adapters;
pub mod install;
pub mod model;
pub mod source;
pub mod steps;

use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Args, Subcommand};
use indicatif::ProgressBar;

use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::{LocalizedErrorContext, Localizer};
use install::{InstallState, Receipt};
use model::{Agent, Bundle, Scope};

#[derive(Debug, Args)]
pub struct SetupArgs {
    #[command(subcommand)]
    pub command: Option<SetupCommand>,

    #[arg(
        long,
        global = true,
        value_enum,
        value_delimiter = ',',
        value_name = "AGENT",
        help = "Agents to configure (comma-separated or repeated)"
    )]
    pub agent: Vec<Agent>,

    #[arg(
        long,
        global = true,
        value_enum,
        value_name = "SCOPE",
        help = "Installation scope (project | user)"
    )]
    pub scope: Option<Scope>,

    #[arg(
        long,
        value_delimiter = ',',
        conflicts_with = "agent",
        value_name = "ASSISTANT",
        help = "Legacy selection (claude | codex | both); defaults to user scope"
    )]
    pub assistant: Vec<String>,

    #[arg(
        long,
        hide = true,
        help = "Deprecated compatibility flag; has no effect"
    )]
    pub allow_dirty: bool,
}

#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    #[command(about = "Update recorded PortOne skills and MCP settings")]
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(long, help = "Preview updates without writing files")]
    pub dry_run: bool,
}

#[derive(Debug)]
struct Request {
    agents: Vec<Agent>,
    scope: Option<Scope>,
    update: bool,
    dry_run: bool,
}

struct SetupContext {
    project: PathBuf,
    home: PathBuf,
    user_receipt: PathBuf,
    env: BTreeMap<String, String>,
}

impl SetupContext {
    fn detect() -> anyhow::Result<Self> {
        let project = std::env::current_dir()?;
        let home = etcetera::home_dir()?;
        let user_receipt = crate::config::paths::config_dir().join("setup.json");
        let user_receipt = if user_receipt.is_absolute() {
            user_receipt
        } else {
            project.join(user_receipt)
        };
        Ok(Self {
            project,
            home,
            user_receipt,
            env: std::env::vars_os()
                .filter_map(|(key, value)| {
                    Some((key.into_string().ok()?, value.into_string().ok()?))
                })
                .collect(),
        })
    }

    fn location(&self, scope: Scope) -> (PathBuf, &Path) {
        match scope {
            Scope::Project => (self.project.join(".portone/setup.json"), &self.project),
            Scope::User => (self.user_receipt.clone(), &self.home),
        }
    }
}

pub fn run(f: &mut Factory, args: SetupArgs) -> Result<(), CliError> {
    let request = resolve_request(&f.localizer, args, std::io::stdin().is_terminal())?;
    let context = SetupContext::detect()?;
    execute(
        &f.localizer,
        &request,
        &context,
        || steps::check_runtime(&steps::ShellRunner, &context.project),
        || source::SourceClient::new()?.fetch_bundle(),
    )
}

fn resolve_request(
    localizer: &Localizer,
    args: SetupArgs,
    interactive: bool,
) -> Result<Request, CliError> {
    let update = args.command.is_some();
    let dry_run = matches!(
        args.command,
        Some(SetupCommand::Update(UpdateArgs { dry_run: true }))
    );
    let legacy = !args.assistant.is_empty();
    if update && legacy {
        return Err(CliError::Flag(crate::tr!(localizer, "setup-update-legacy")));
    }
    let mut agents = args.agent;
    for value in &args.assistant {
        match value.as_str() {
            "claude" => agents.push(Agent::ClaudeCode),
            "codex" => agents.push(Agent::Codex),
            "both" => agents.extend([Agent::ClaudeCode, Agent::Codex]),
            _ => {
                return Err(CliError::Flag(crate::tr!(
                    localizer,
                    "setup-unsupported-assistant",
                    assistant = value.as_str()
                )));
            }
        }
    }
    let mut unique = Vec::new();
    for agent in agents {
        if !unique.contains(&agent) {
            unique.push(agent);
        }
    }
    let mut scope = args.scope.or(if legacy { Some(Scope::User) } else { None });
    if !update {
        if !interactive && (unique.is_empty() || scope.is_none()) {
            return Err(CliError::Flag(crate::tr!(
                localizer,
                "setup-options-required"
            )));
        }
        if unique.is_empty() {
            let question = crate::tr!(localizer, "setup-agent-question");
            let hint = crate::tr!(localizer, "setup-multiselect-hint");
            let canceled = crate::tr!(localizer, "setup-prompt-canceled-indicator");
            let required = crate::tr!(localizer, "setup-agent-required");
            let mut prompt = inquire::MultiSelect::new(&question, Agent::ALL.to_vec())
                .with_help_message(&hint)
                .with_validator(move |values: &[inquire::list_option::ListOption<&Agent>]| {
                    Ok(if values.is_empty() {
                        inquire::validator::Validation::Invalid(required.clone().into())
                    } else {
                        inquire::validator::Validation::Valid
                    })
                });
            prompt.render_config.canceled_prompt_indicator.content = &canceled;
            unique = prompt.prompt().map_err(prompt_error)?;
        }
        if scope.is_none() {
            let question = crate::tr!(localizer, "setup-scope-question");
            let project = crate::tr!(localizer, "setup-scope-project");
            let user = crate::tr!(localizer, "setup-scope-user");
            let canceled = crate::tr!(localizer, "setup-prompt-canceled-indicator");
            let mut prompt = inquire::Select::new(&question, vec![project.clone(), user]);
            prompt.render_config.canceled_prompt_indicator.content = &canceled;
            scope = Some(if prompt.prompt().map_err(prompt_error)? == project {
                Scope::Project
            } else {
                Scope::User
            });
        }
    }
    Ok(Request {
        agents: unique,
        scope,
        update,
        dry_run,
    })
}

fn execute(
    localizer: &Localizer,
    request: &Request,
    context: &SetupContext,
    check_runtime: impl FnOnce() -> anyhow::Result<()>,
    fetch_bundle: impl FnOnce() -> anyhow::Result<Bundle>,
) -> Result<(), CliError> {
    let scopes = request
        .scope
        .map_or_else(|| vec![Scope::Project, Scope::User], |scope| vec![scope]);
    let mut receipts = Vec::new();
    for scope in scopes {
        let (path, root) = context.location(scope);
        let mut receipt = match Receipt::load(&path, scope, root)? {
            Some(receipt) => receipt,
            None if request.update => continue,
            None => Receipt::new(scope, root),
        };
        if request.update {
            if !request.agents.is_empty()
                && !receipt
                    .agents()
                    .iter()
                    .any(|agent| request.agents.contains(agent))
            {
                continue;
            }
            if receipt.agents().is_empty() {
                continue;
            }
        } else {
            let destinations = request
                .agents
                .iter()
                .map(|agent| {
                    adapters::resolve_destination(
                        *agent,
                        scope,
                        &context.project,
                        &context.home,
                        &context.env,
                    )
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            receipt.add_targets(&destinations)?;
        }
        receipts.push((scope, path, receipt));
    }
    if receipts.is_empty() {
        anstream::println!("{}", crate::tr!(localizer, "setup-not-configured"));
        return Ok(());
    }
    if !request.dry_run {
        check_runtime().map_err(CliError::Other)?;
        anstream::println!("{}", crate::tr!(localizer, "setup-runtime-ready"));
    }
    let spinner =
        ProgressBar::new_spinner().with_message(crate::tr!(localizer, "setup-downloading"));
    spinner.enable_steady_tick(Duration::from_millis(80));
    let result = fetch_bundle();
    spinner.finish_and_clear();
    let bundle = result.map_err(|error| {
        CliError::Other(error.lcontext(crate::message!("setup-download-failed")))
    })?;
    anstream::println!(
        "{}",
        crate::tr!(
            localizer,
            "setup-source",
            reference = bundle.revision.reference.as_str(),
            commit = bundle.revision.commit.as_str()
        )
    );

    // Parse and validate every selected configuration before any destination is written.
    let prepared = receipts
        .into_iter()
        .map(|(scope, path, receipt)| {
            receipt
                .prepare(&bundle, &request.agents)
                .map(|prepared| (scope, path, prepared))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut failed = false;
    for (scope, path, prepared) in prepared {
        let scope_label = match scope {
            Scope::Project => crate::tr!(localizer, "setup-scope-project"),
            Scope::User => crate::tr!(localizer, "setup-scope-user"),
        };
        anstream::println!("\n[{scope_label}]");
        let report = prepared.apply(&path, request.dry_run);
        for item in report.items {
            let state = match item.state {
                InstallState::Unchanged => crate::tr!(localizer, "setup-state-unchanged"),
                InstallState::WouldUpdate => crate::tr!(localizer, "setup-state-would-update"),
                InstallState::Updated => crate::tr!(localizer, "setup-state-updated"),
                InstallState::Failed => crate::tr!(localizer, "setup-state-failed"),
            };
            anstream::println!("  {state}: {}", item.path.display());
        }
        for error in &report.errors {
            anstream::eprintln!(
                "{}",
                crate::tr!(localizer, "setup-write-failed", detail = error.as_str())
            );
        }
        failed |= !report.errors.is_empty();
        for agent in &report.successful_agents {
            anstream::println!("  [{agent}]");
        }
    }
    if failed {
        anstream::eprintln!("{}", crate::tr!(localizer, "setup-incomplete"));
        return Err(CliError::Silent);
    }
    if request.dry_run {
        anstream::println!("\n{}", crate::tr!(localizer, "setup-dry-run-complete"));
    } else {
        anstream::println!("\n{}", crate::tr!(localizer, "setup-complete"));
        anstream::println!("{}", crate::tr!(localizer, "setup-mcp-next-steps"));
        anstream::println!("{}", crate::tr!(localizer, "setup-update-hint"));
    }
    Ok(())
}

fn prompt_error(error: inquire::InquireError) -> CliError {
    use inquire::InquireError;

    let message = match error {
        InquireError::NotTTY => crate::message!("setup-prompt-not-tty"),
        InquireError::OperationCanceled => crate::message!("setup-prompt-canceled"),
        InquireError::OperationInterrupted => crate::message!("setup-prompt-interrupted"),
        // Preserve external error details, localizing only the surrounding prompt message.
        InquireError::InvalidConfiguration(detail) => {
            crate::message!("setup-prompt-invalid-config", detail = detail)
        }
        InquireError::IO(detail) => {
            return CliError::Other(
                anyhow::Error::new(detail).lcontext(crate::message!("setup-prompt-io-error")),
            );
        }
        InquireError::Custom(detail) => {
            return CliError::Other(
                anyhow::Error::from_boxed(detail)
                    .lcontext(crate::message!("setup-prompt-custom-error")),
            );
        }
    };
    CliError::Other(anyhow::anyhow!(message))
}

#[cfg(test)]
mod cli_tests;
