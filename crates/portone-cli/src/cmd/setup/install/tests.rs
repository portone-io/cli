use super::super::model::{McpServer, Revision, SkillBundle};
use super::*;

fn bundle() -> Bundle {
    Bundle {
        revision: Revision {
            repository: REPOSITORY.into(),
            reference: "v1".into(),
            commit: "a".repeat(40),
        },
        skills: SKILL_NAMES
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    SkillBundle {
                        tree_sha: "b".repeat(40),
                        files: BTreeMap::from([(
                            PathBuf::from("SKILL.md"),
                            format!("# {name}\n").into_bytes(),
                        )]),
                    },
                )
            })
            .collect(),
        mcp: McpServer {
            command: "npx".into(),
            args: vec!["-y".into(), "@portone/mcp-server".into()],
            env: BTreeMap::new(),
        },
    }
}

fn target(root: &Path, agent: Agent) -> Destination {
    let (skills, mcp) = match agent {
        Agent::ClaudeCode => (".claude/skills", ".mcp.json"),
        Agent::Codex => (".agents/skills", ".codex/config.toml"),
        Agent::GithubCopilot => (".agents/skills", ".mcp.json"),
        Agent::VscodeCopilot => (".agents/skills", ".vscode/mcp.json"),
        Agent::Opencode => (".agents/skills", "opencode.json"),
        _ => unreachable!(),
    };
    Destination {
        agent,
        skills_dir: root.join(skills),
        mcp_path: root.join(mcp),
    }
}

#[test]
fn receipt_and_install_reports_preserve_first_selected_agent_order() {
    for agents in [
        vec![Agent::Opencode, Agent::Codex, Agent::ClaudeCode],
        vec![Agent::Codex, Agent::ClaudeCode],
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let path = root.join(".portone/setup.json");
        let saved = receipt(root, &agents);
        assert_eq!(saved.agents(), agents);
        let report = saved.prepare(&bundle(), &[]).unwrap().apply(&path, false);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.successful_agents, agents);
        let config_positions = agents
            .iter()
            .map(|agent| {
                let expected = target(root, *agent).mcp_path;
                report
                    .items
                    .iter()
                    .position(|item| item.path == expected)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(config_positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            report.items[0]
                .path
                .starts_with(root.join(".agents/skills"))
        );
        let loaded = Receipt::load(&path, Scope::Project, root).unwrap().unwrap();
        assert_eq!(loaded.agents(), agents);
        let selected = vec![Agent::ClaudeCode, agents[0]];
        let report = loaded
            .prepare(&bundle(), &selected)
            .unwrap()
            .apply(&path, true);
        assert_eq!(report.successful_agents, selected);
        assert!(
            report.items[0]
                .path
                .starts_with(root.join(".claude/skills"))
        );
        let first_mcp = report
            .items
            .iter()
            .position(|item| item.path == root.join(".mcp.json"))
            .unwrap();
        let shared_skills = report
            .items
            .iter()
            .position(|item| item.path.starts_with(root.join(".agents/skills")))
            .unwrap();
        assert!(first_mcp < shared_skills);
    }
}

#[test]
fn readers_never_see_a_missing_mcp_file_during_replacement_or_rollback() {
    use std::sync::{
        Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    fs::write(&path, b"old").unwrap();
    let start = Barrier::new(2);
    let done = AtomicBool::new(false);
    let missing = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            start.wait();
            while !done.load(Ordering::Acquire) {
                match fs::read(&path) {
                    Ok(bytes) => assert!(bytes == b"old" || bytes == b"new"),
                    Err(error) if error.kind() == ErrorKind::NotFound => {
                        missing.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(error) => panic!("Unexpected read failure: {error}"),
                }
            }
        });
        start.wait();
        for _ in 0..100 {
            let mut replacement =
                Replacement::install(&path, &Content::File(b"new".to_vec())).unwrap();
            replacement.rollback().unwrap();
        }
        done.store(true, Ordering::Release);
    });
    assert_eq!(
        missing.load(Ordering::Relaxed),
        0,
        "Readers observed the target missing between file renames"
    );
    assert_eq!(fs::read(path).unwrap(), b"old");
}

fn receipt(root: &Path, agents: &[Agent]) -> Receipt {
    let mut receipt = Receipt::new(Scope::Project, root);
    receipt
        .add_targets(&agents.iter().map(|a| target(root, *a)).collect::<Vec<_>>())
        .unwrap();
    receipt
}

#[test]
fn installs_deduplicated_skills_and_restores_drift_and_removed_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let path = root.join(".portone/setup.json");
    let first = receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&path, false);
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    assert_eq!(first.items.len(), 5);
    assert_eq!(first.successful_agents, [Agent::ClaudeCode]);
    let managed = root.join(".claude/skills/portone-cli");
    fs::write(managed.join("SKILL.md"), "local edit").unwrap();
    fs::write(managed.join("stale.md"), "stale").unwrap();
    fs::create_dir_all(root.join(".claude/skills/personal")).unwrap();
    fs::write(root.join(".claude/skills/personal/SKILL.md"), "personal").unwrap();
    fs::remove_file(root.join(".mcp.json")).unwrap();
    let next = Receipt::load(&path, Scope::Project, root)
        .unwrap()
        .unwrap()
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&path, false);
    assert!(next.errors.is_empty(), "{:?}", next.errors);
    assert_eq!(
        fs::read_to_string(managed.join("SKILL.md")).unwrap(),
        "# portone-cli\n"
    );
    assert!(!managed.join("stale.md").exists());
    assert!(root.join(".claude/skills/personal/SKILL.md").exists());
    assert!(root.join(".mcp.json").exists());
    let stored: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(stored["artifacts"].as_array().unwrap().len(), 5);
    assert!(
        stored["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["last_success"]["commit"] == "a".repeat(40))
    );
}

#[test]
fn dry_run_never_creates_receipt_or_target_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let report = receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&root.join(".portone/setup.json"), true);
    assert!(report.errors.is_empty());
    assert!(
        report
            .items
            .iter()
            .all(|item| item.state == InstallState::WouldUpdate)
    );
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn malformed_mcp_prevents_all_prepared_writes() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join(".mcp.json"), "{ broken").unwrap();
    assert!(
        receipt(temp.path(), &[Agent::ClaudeCode, Agent::Codex])
            .prepare(&bundle(), &[])
            .is_err()
    );
    assert!(!temp.path().join(".agents").exists());
    assert!(!temp.path().join(".portone").exists());
}

#[test]
fn project_receipt_relocates_and_user_receipt_retains_absolute_destinations() {
    let temp = tempfile::tempdir().unwrap();
    let old = temp.path().join("old");
    let new = temp.path().join("new");
    fs::create_dir(&old).unwrap();
    let receipt_path = old.join(".portone/setup.json");
    assert!(
        receipt(&old, &[Agent::ClaudeCode])
            .prepare(&bundle(), &[])
            .unwrap()
            .apply(&receipt_path, false)
            .errors
            .is_empty()
    );
    fs::rename(&old, &new).unwrap();
    fs::remove_file(new.join(".claude/skills/portone-cli/SKILL.md")).unwrap();
    let receipt_path = new.join(".portone/setup.json");
    let loaded = Receipt::load(&receipt_path, Scope::Project, &new)
        .unwrap()
        .unwrap();
    assert!(
        loaded
            .prepare(&bundle(), &[])
            .unwrap()
            .apply(&receipt_path, false)
            .errors
            .is_empty()
    );
    assert!(new.join(".claude/skills/portone-cli/SKILL.md").exists());
    assert!(!old.exists());

    let home = temp.path().join("home");
    let mut user = Receipt::new(Scope::User, &home);
    user.add_targets(&[Destination {
        agent: Agent::Codex,
        skills_dir: home.join(".agents/skills"),
        mcp_path: home.join("custom-codex/config.toml"),
    }])
    .unwrap();
    let user_path = home.join("portone/setup.json");
    assert!(
        user.prepare(&bundle(), &[])
            .unwrap()
            .apply(&user_path, false)
            .errors
            .is_empty()
    );
    let loaded = Receipt::load(&user_path, Scope::User, &temp.path().join("different-home"))
        .unwrap()
        .unwrap();
    let report = loaded
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&user_path, true);
    assert!(report.items.iter().all(|item| item.path.starts_with(&home)));
}

#[test]
fn shared_skill_targets_are_deduplicated() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let report = receipt(root, &[Agent::GithubCopilot, Agent::VscodeCopilot])
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&root.join(".portone/setup.json"), false);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.items.len(), 6);
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".vscode/mcp.json")).unwrap()).unwrap();
    assert!(json["servers"]["portone"].is_object());
    assert!(root.join(".mcp.json").exists());
}

#[test]
fn filtering_a_shared_mcp_keeps_requirements_of_unselected_agents() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let receipt_path = root.join(".portone/setup.json");
    let report = receipt(root, &[Agent::ClaudeCode, Agent::GithubCopilot])
        .prepare(&bundle(), &[Agent::ClaudeCode])
        .unwrap()
        .apply(&receipt_path, false);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.items.len(), 5);
    assert!(!root.join(".agents/skills").exists());
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".mcp.json")).unwrap()).unwrap();
    assert_eq!(
        json["mcpServers"]["portone"]["tools"],
        serde_json::json!(["*"])
    );
    let receipt = Receipt::load(&receipt_path, Scope::Project, root)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.agents(), [Agent::ClaudeCode, Agent::GithubCopilot]);
}

#[test]
fn every_adapter_destination_installs_and_reloads_in_both_scopes() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    let home = temp.path().join("home");
    for (scope, root, path) in [
        (
            Scope::Project,
            &project,
            project.join(".portone/setup.json"),
        ),
        (Scope::User, &home, home.join("portone/setup.json")),
    ] {
        let targets = Agent::ALL
            .into_iter()
            .map(|agent| {
                super::super::adapters::resolve_destination(
                    agent,
                    scope,
                    &project,
                    &home,
                    &BTreeMap::new(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut receipt = Receipt::new(scope, root);
        receipt.add_targets(&targets).unwrap();
        let report = receipt.prepare(&bundle(), &[]).unwrap().apply(&path, false);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.successful_agents, Agent::ALL);
        let report = Receipt::load(&path, scope, root)
            .unwrap()
            .unwrap()
            .prepare(&bundle(), &[])
            .unwrap()
            .apply(&path, true);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(
            report
                .items
                .iter()
                .all(|item| item.state == InstallState::Unchanged)
        );
    }
}

#[test]
fn rejects_wrong_scope_traversal_and_unknown_receipt_schema() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let path = root.join(".portone/setup.json");
    receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&path, false);
    assert!(Receipt::load(&path, Scope::User, root).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["targets"][0]["skills_dir"] = "../escape".into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Receipt::load(&path, Scope::Project, root).is_err());
    value["version"] = 500.into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Receipt::load(&path, Scope::Project, root).is_err());
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_and_preserves_unrelated_configuration_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let external = tempfile::tempdir().unwrap();
    symlink(external.path(), root.join(".claude")).unwrap();
    assert!(
        receipt(root, &[Agent::ClaudeCode])
            .prepare(&bundle(), &[])
            .is_err()
    );
    fs::remove_file(root.join(".claude")).unwrap();
    fs::write(root.join(".mcp.json"), "{\n // keep\n \"other\": true\n}\n").unwrap();
    fs::set_permissions(root.join(".mcp.json"), fs::Permissions::from_mode(0o640)).unwrap();
    let report = receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&root.join(".portone/setup.json"), false);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let mcp = fs::read_to_string(root.join(".mcp.json")).unwrap();
    assert!(mcp.contains("// keep"));
    assert!(mcp.contains("\"other\": true"));
    assert_eq!(
        fs::metadata(root.join(".mcp.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
}

#[test]
fn pending_receipt_failure_does_not_replace_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("blocked"), "file").unwrap();
    let report = receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle(), &[])
        .unwrap()
        .apply(&root.join("blocked/setup.json"), false);
    assert!(!report.errors.is_empty());
    assert!(report.successful_agents.is_empty());
    assert!(!root.join(".claude").exists());
}

#[test]
fn individual_artifact_failure_keeps_other_agents_updatable() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let path = root.join(".portone/setup.json");
    let prepared = receipt(root, &[Agent::ClaudeCode, Agent::Codex])
        .prepare(&bundle(), &[])
        .unwrap();
    fs::write(root.join(".claude"), "blocked after preflight").unwrap();
    let report = prepared.apply(&path, false);
    assert_eq!(report.errors.len(), 4);
    assert_eq!(report.successful_agents, [Agent::Codex]);
    assert!(root.join(".agents/skills/portone-cli/SKILL.md").exists());
    let loaded = Receipt::load(&path, Scope::Project, root).unwrap().unwrap();
    assert_eq!(loaded.agents(), [Agent::ClaudeCode, Agent::Codex]);
    assert!(
        loaded
            .artifacts
            .iter()
            .filter(|a| a.path.starts_with(".claude"))
            .all(|a| a.last_success.is_none())
    );
    fs::remove_file(root.join(".claude")).unwrap();
    let report = loaded
        .prepare(&bundle(), &[Agent::ClaudeCode])
        .unwrap()
        .apply(&path, false);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(root.join(".claude/skills/portone-cli/SKILL.md").exists());
}

#[test]
fn failed_success_receipt_save_rolls_back_corresponding_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let prepared = receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle(), &[])
        .unwrap();
    // This path becomes a file when its MCP artifact is replaced, forcing a
    // real receipt write failure after replacement without permission mocks.
    let path = root.join(".mcp.json/receipt.json");
    let report = prepared.apply(&path, false);
    assert!(!report.errors.is_empty());
    assert!(report.successful_agents.is_empty());
    assert!(root.join(".mcp.json").is_dir());
    let loaded = Receipt::load(&path, Scope::Project, root).unwrap().unwrap();
    let mcp = loaded
        .artifacts
        .iter()
        .find(|a| a.path == Path::new(".mcp.json"))
        .unwrap();
    assert!(mcp.last_success.is_none());
}

#[test]
fn rejects_overlapping_user_artifacts_before_any_installation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut receipt = Receipt::new(Scope::User, root);
    let target = Destination {
        agent: Agent::Codex,
        skills_dir: root.join(".agents/skills"),
        mcp_path: root.join(".agents/skills/portone-cli/config.toml"),
    };
    assert!(receipt.add_targets(&[target]).is_err());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn a_managed_empty_directory_is_replaced_by_an_empty_source_file() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let target = root.join(".claude/skills/portone-cli");
    fs::create_dir_all(target.join("empty.md")).unwrap();
    fs::write(target.join("SKILL.md"), "# portone-cli\n").unwrap();
    let mut bundle = bundle();
    bundle
        .skills
        .get_mut("portone-cli")
        .unwrap()
        .files
        .insert(PathBuf::from("empty.md"), vec![]);
    let report = receipt(root, &[Agent::ClaudeCode])
        .prepare(&bundle, &[])
        .unwrap()
        .apply(&root.join(".portone/setup.json"), false);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(target.join("empty.md").is_file());
}
