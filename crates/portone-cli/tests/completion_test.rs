use assert_cmd::Command;
use predicates::prelude::*;

fn portone() -> Command {
    let mut command = Command::cargo_bin("portone").expect("portone binary not found");
    command.env("PORTONE_LANG", "en");
    command
}

#[test]
fn zsh_script_starts_with_compdef() {
    portone()
        .arg("completion")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("#compdef portone"));
}

#[test]
fn bash_script_defines_portone_function() {
    portone()
        .arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_portone"));
}

#[test]
fn unknown_shell_fails_with_usage_error() {
    portone().arg("completion").arg("nushell").assert().code(2);
}

#[test]
fn completion_includes_payment_workflows_and_store_defaults() {
    portone()
        .args(["completion", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c portone"))
        .stdout(predicate::str::contains("transactions"))
        .stdout(predicate::str::contains("resend"))
        .stdout(predicate::str::contains("set-default"))
        .stdout(predicate::str::contains("all-stores"))
        .stdout(predicate::str::contains("current-cancellable-amount"))
        .stdout(predicate::str::contains("webhook-id"));
}

#[test]
fn fish_completion_scopes_payment_id_to_the_five_payment_targets() {
    let output = portone().args(["completion", "fish"]).output().unwrap();
    assert!(output.status.success());
    let script = String::from_utf8(output.stdout).unwrap();
    let payment_id = script
        .lines()
        .filter(|line| line.contains("-l payment-id "))
        .collect::<Vec<_>>();
    let paths = [
        "portone payment view",
        "portone payment transactions",
        "portone payment cancel",
        "portone payment webhook list",
        "portone payment webhook resend",
    ];
    assert_eq!(payment_id.len(), paths.len(), "{payment_id:#?}");
    for path in paths {
        let condition = format!("__fish_portone_command_is \\'{path}\\'");
        assert!(
            payment_id.iter().any(|line| line.contains(&condition)),
            "missing {path}: {payment_id:#?}"
        );
    }
}

#[test]
fn fish_completion_scopes_nested_options_to_exact_command_paths() {
    let output = portone().args(["completion", "fish"]).output().unwrap();
    assert!(output.status.success());
    let script = String::from_utf8(output.stdout).unwrap();
    let webhook = script
        .lines()
        .find(|line| line.contains("-l webhook-id "))
        .unwrap();
    assert!(
        webhook.contains("portone payment webhook resend"),
        "{webhook}"
    );
    for line in script.lines().filter(|line| line.contains("-l limit ")) {
        assert!(line.contains("portone payment list"), "{line}");
        assert!(!line.contains("webhook"), "{line}");
    }
    let json = script
        .lines()
        .filter(|line| line.contains("-l json "))
        .collect::<Vec<_>>();
    assert!(
        json.iter()
            .any(|line| line.contains("portone payment webhook list"))
    );
    assert!(
        json.iter()
            .any(|line| line.contains("portone payment webhook resend"))
    );
    assert!(script.contains("argparse -s 'profile=' 'base-url=' 'store=' 'h/help'"));
    let script = script.lines().map(str::trim).collect::<Vec<_>>().join("\n");
    assert!(script.contains("case 'list' 'ls'\nset path 'portone payment list'"));
    assert!(script.contains("case 'list' 'ls'\nset path 'portone payment webhook list'"));
    assert!(!script.contains("__fish_seen_subcommand_from"));
}
