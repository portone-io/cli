use assert_cmd::Command;
use predicates::prelude::*;

fn portone() -> Command {
    let mut command = Command::cargo_bin("portone").expect("portone binary not found");
    command.env("PORTONE_LANG", "en").env("NO_COLOR", "1");
    command
}

#[test]
fn setup_requires_explicit_agent_and_scope_without_a_terminal_and_hides_legacy_flag() {
    portone()
        .args(["setup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--agent"))
        .stdout(predicate::str::contains("--assistant"))
        .stdout(predicate::str::contains("--scope"))
        .stdout(predicate::str::contains("--allow-dirty").not());
    for args in [
        vec!["setup", "--allow-dirty"],
        vec!["setup", "--agent", "codex"],
        vec!["setup", "--scope", "project"],
    ] {
        portone()
            .args(args)
            .assert()
            .code(1)
            .stderr(predicate::str::contains(
                "--agent and --scope are required in non-interactive environments",
            ));
    }
}

#[test]
fn invalid_later_legacy_selection_is_rejected_before_runtime_or_network_access() {
    for args in [
        vec!["setup", "--assistant", "claude,gemini"],
        vec!["setup", "--assistant", "claude", "--assistant", "gemini"],
    ] {
        let root = tempfile::tempdir().unwrap();
        portone()
            .current_dir(root.path())
            .env("PATH", "")
            .args(args)
            .assert()
            .code(1)
            .stderr(predicate::str::contains("Unsupported assistant: gemini"));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[test]
fn update_without_receipts_needs_no_runtime_network_or_write_access() {
    let root = tempfile::tempdir().unwrap();
    for extra in [
        vec![],
        vec!["--dry-run"],
        vec!["--agent", "opencode", "--scope", "user"],
    ] {
        portone()
            .current_dir(root.path())
            .env("PORTONE_CONFIG_DIR", root.path().join("config"))
            .env("PATH", "")
            .arg("setup")
            .arg("update")
            .args(extra)
            .assert()
            .success()
            .stdout(predicate::str::contains("No recorded installations match"));
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn update_help_exposes_filters_and_preview_in_both_languages() {
    for language in ["en", "ko"] {
        portone()
            .env("PORTONE_LANG", language)
            .args(["setup", "update", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--agent"))
            .stdout(predicate::str::contains("--scope"))
            .stdout(predicate::str::contains("--dry-run"));
    }
}
