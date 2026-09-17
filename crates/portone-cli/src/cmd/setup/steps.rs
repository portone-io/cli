use std::path::Path;
use std::process::Command;

use crate::i18n::LocalizedContext;

pub trait CommandRunner {
    fn run_capture(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String>;
    fn run_capture_stdout(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        self.run_capture(cmd, cwd)
    }
}

pub struct ShellRunner;

fn shell_command(cmd: &str) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(cmd);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("sh");
        command.arg("-c").arg(cmd);
        command
    }
}

impl CommandRunner for ShellRunner {
    fn run_capture(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        let output = shell_command(cmd)
            .current_dir(cwd)
            .output()
            .with_lcontext(|| crate::message!("setup-command-run-failed", command = cmd))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            anyhow::bail!(crate::message!(
                "setup-command-output-failed",
                command = cmd,
                output = combined.trim_end()
            ));
        }
        Ok(combined)
    }

    fn run_capture_stdout(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        let output = shell_command(cmd)
            .current_dir(cwd)
            .output()
            .with_lcontext(|| crate::message!("setup-command-run-failed", command = cmd))?;
        if !output.status.success() {
            let detail = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            anyhow::bail!(crate::message!(
                "setup-command-output-failed",
                command = cmd,
                output = detail.trim_end()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

pub fn check_runtime(runner: &dyn CommandRunner, cwd: &Path) -> anyhow::Result<()> {
    for (command, requirement) in [
        ("node --version", "Node.js"),
        ("npx --version", "npx (npm)"),
    ] {
        runner.run_capture_stdout(command, cwd).with_lcontext(|| {
            crate::message!("setup-runtime-required", requirement = requirement)
        })?;
    }
    Ok(())
}
