use super::*;
use crate::cmd::{Cli, Command};
use clap::Parser;

fn args(input: &[&str]) -> SetupArgs {
    let Command::Setup(args) = Cli::try_parse_from([&["portone", "setup"][..], input].concat())
        .unwrap()
        .command
    else {
        panic!("setup")
    };
    args
}

#[test]
fn selected_agents_deduplicate_in_first_selected_order() {
    let request = resolve_request(
        &Localizer::english(),
        args(&[
            "--agent",
            "opencode,codex,opencode",
            "--agent",
            "cursor,codex",
            "--scope",
            "project",
        ]),
        false,
    )
    .unwrap();
    assert_eq!(
        request.agents,
        [Agent::Opencode, Agent::Codex, Agent::Cursor]
    );
    assert_eq!(request.scope, Some(Scope::Project));
}

#[test]
fn legacy_comma_and_repeated_selection_defaults_to_user_scope() {
    for input in [
        vec!["--assistant", "codex,claude,codex", "--assistant", "both"],
        vec!["--assistant", "codex", "--assistant", "claude"],
    ] {
        let request = resolve_request(&Localizer::english(), args(&input), false).unwrap();
        assert_eq!(request.agents, [Agent::Codex, Agent::ClaudeCode]);
        assert_eq!(request.scope, Some(Scope::User));
    }
}

#[test]
fn new_installation_requires_both_options_outside_a_terminal() {
    for input in [vec![], vec!["--agent", "codex"], vec!["--scope", "project"]] {
        assert!(resolve_request(&Localizer::english(), args(&input), false).is_err());
    }
}

#[test]
fn update_has_optional_filters_and_accepts_them_after_the_subcommand() {
    let request =
        resolve_request(&Localizer::english(), args(&["update", "--dry-run"]), false).unwrap();
    assert!(request.update && request.dry_run);
    assert!(request.agents.is_empty());
    assert_eq!(request.scope, None);
    let request = resolve_request(
        &Localizer::english(),
        args(&["update", "--agent", "opencode", "--scope", "user"]),
        false,
    )
    .unwrap();
    assert_eq!(request.agents, [Agent::Opencode]);
    assert_eq!(request.scope, Some(Scope::User));
}

#[test]
fn legacy_and_new_flags_cannot_be_mixed_and_invalid_later_legacy_values_fail() {
    assert!(
        Cli::try_parse_from([
            "portone",
            "setup",
            "--agent",
            "codex",
            "--assistant",
            "both"
        ])
        .is_err()
    );
    for input in [
        vec!["--assistant", "claude,gemini"],
        vec!["--assistant", "claude", "--assistant", "gemini"],
    ] {
        assert!(resolve_request(&Localizer::english(), args(&input), false).is_err());
    }
}

fn bundle(revision: &str) -> Bundle {
    use model::{McpServer, REPOSITORY, Revision, SKILL_NAMES, SkillBundle};
    Bundle {
        revision: Revision {
            repository: REPOSITORY.into(),
            reference: "latest".into(),
            commit: revision.repeat(40),
        },
        skills: SKILL_NAMES
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    SkillBundle {
                        tree_sha: revision.repeat(40),
                        files: BTreeMap::from([(
                            PathBuf::from("SKILL.md"),
                            format!("---\nname: {name}\n---\n{revision}\n").into_bytes(),
                        )]),
                    },
                )
            })
            .collect(),
        mcp: McpServer {
            command: "npx".into(),
            args: vec!["-y".into(), "@portone/mcp-server@latest".into()],
            env: BTreeMap::new(),
        },
    }
}

fn context(root: &Path) -> SetupContext {
    let project = root.join("project");
    let home = root.join("home");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    SetupContext {
        project,
        home,
        user_receipt: root.join("config/setup.json"),
        env: BTreeMap::new(),
    }
}

fn install(context: &SetupContext, agents: Vec<Agent>, scope: Scope) {
    let request = Request {
        agents,
        scope: Some(scope),
        update: false,
        dry_run: false,
    };
    execute(
        &Localizer::english(),
        &request,
        context,
        || Ok(()),
        || Ok(bundle("a")),
    )
    .unwrap();
}

#[test]
fn setup_then_update_all_recorded_scopes_downloads_once_and_repairs_local_drift() {
    use std::cell::Cell;
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    install(
        &context,
        vec![Agent::Codex, Agent::Opencode],
        Scope::Project,
    );
    install(&context, vec![Agent::Cursor], Scope::User);
    let project_skill = context.project.join(".agents/skills/portone-cli/SKILL.md");
    std::fs::write(&project_skill, "local drift").unwrap();
    let user_skill = context.home.join(".cursor/skills/portone-cli/SKILL.md");
    std::fs::remove_file(&user_skill).unwrap();
    let calls = Cell::new(0);
    let request = Request {
        agents: vec![],
        scope: None,
        update: true,
        dry_run: false,
    };
    execute(
        &Localizer::english(),
        &request,
        &context,
        || Ok(()),
        || {
            calls.set(calls.get() + 1);
            Ok(bundle("a"))
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        std::fs::read(&project_skill).unwrap(),
        std::fs::read(&user_skill).unwrap()
    );
    assert!(
        std::fs::read_to_string(project_skill)
            .unwrap()
            .ends_with("a\n")
    );
}

#[test]
fn dry_run_and_download_failure_preserve_skills_and_receipts() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    install(&context, vec![Agent::Codex], Scope::Project);
    let receipt = context.project.join(".portone/setup.json");
    let skill = context.project.join(".agents/skills/portone-cli/SKILL.md");
    let before = (
        std::fs::read(&receipt).unwrap(),
        std::fs::read(&skill).unwrap(),
    );
    let mut request = Request {
        agents: vec![],
        scope: None,
        update: true,
        dry_run: true,
    };
    execute(
        &Localizer::english(),
        &request,
        &context,
        || panic!("dry-run needs no runtime"),
        || Ok(bundle("b")),
    )
    .unwrap();
    assert_eq!(
        before,
        (
            std::fs::read(&receipt).unwrap(),
            std::fs::read(&skill).unwrap()
        )
    );
    request.dry_run = false;
    assert!(
        execute(
            &Localizer::english(),
            &request,
            &context,
            || Ok(()),
            || anyhow::bail!("network error")
        )
        .is_err()
    );
    assert_eq!(
        before,
        (
            std::fs::read(&receipt).unwrap(),
            std::fs::read(&skill).unwrap()
        )
    );
}

#[test]
fn late_scope_preflight_error_prevents_earlier_scope_changes() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    install(&context, vec![Agent::Cursor], Scope::Project);
    install(&context, vec![Agent::Cursor], Scope::User);
    std::fs::write(context.home.join(".cursor/mcp.json"), "broken {").unwrap();
    let skill = context.project.join(".agents/skills/portone-cli/SKILL.md");
    let before = std::fs::read(&skill).unwrap();
    let request = Request {
        agents: vec![],
        scope: None,
        update: true,
        dry_run: false,
    };
    assert!(
        execute(
            &Localizer::english(),
            &request,
            &context,
            || Ok(()),
            || Ok(bundle("b"))
        )
        .is_err()
    );
    assert_eq!(std::fs::read(skill).unwrap(), before);
}

#[test]
fn update_without_receipts_does_not_guess_from_files_or_fetch() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    std::fs::create_dir_all(context.project.join(".agents/skills/portone-cli")).unwrap();
    let request = Request {
        agents: vec![],
        scope: None,
        update: true,
        dry_run: false,
    };
    execute(
        &Localizer::english(),
        &request,
        &context,
        || panic!("nothing to install"),
        || panic!("nothing to download"),
    )
    .unwrap();
    assert!(!context.project.join(".portone").exists());
    assert!(!context.user_receipt.exists());
}

#[test]
fn repeated_setup_merges_targets_and_preserves_unselected_agents_and_project_files() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    std::fs::write(context.project.join("user.txt"), "dirty project").unwrap();
    install(&context, vec![Agent::Codex], Scope::Project);
    let codex = std::fs::read(context.project.join(".codex/config.toml")).unwrap();
    install(&context, vec![Agent::Opencode], Scope::Project);
    let receipt = Receipt::load(
        &context.project.join(".portone/setup.json"),
        Scope::Project,
        &context.project,
    )
    .unwrap()
    .unwrap();
    assert_eq!(receipt.agents(), [Agent::Codex, Agent::Opencode]);
    assert_eq!(
        std::fs::read(context.project.join(".codex/config.toml")).unwrap(),
        codex
    );
    assert!(!context.project.join(".claude").exists());
    assert_eq!(
        std::fs::read(context.project.join("user.txt")).unwrap(),
        b"dirty project"
    );
}

#[test]
fn failed_runtime_checks_do_not_fetch_or_change_project() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let request = Request {
        agents: vec![Agent::Codex],
        scope: Some(Scope::Project),
        update: false,
        dry_run: false,
    };
    assert!(
        execute(
            &Localizer::english(),
            &request,
            &context,
            || anyhow::bail!("missing npx"),
            || panic!("runtime failed")
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(&context.project).unwrap().count(), 0);
}
