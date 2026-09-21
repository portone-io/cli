pub mod pager;

use std::io::{IsTerminal, Write};

pub struct IoStreams {
    pub out: Box<dyn Write>,
    pub err: Box<dyn Write>,
    pub stdout_is_tty: bool,
    pub stdin_is_tty: bool,
    pub stderr_is_tty: bool,
    no_color: bool,
    color_forced: bool,
}

impl IoStreams {
    pub fn detect() -> Self {
        Self {
            out: Box::new(std::io::stdout()),
            err: Box::new(std::io::stderr()),
            stdout_is_tty: std::io::stdout().is_terminal(),
            stdin_is_tty: std::io::stdin().is_terminal(),
            stderr_is_tty: std::io::stderr().is_terminal(),
            no_color: std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
            color_forced: std::env::var_os("CLICOLOR_FORCE")
                .is_some_and(|v| !v.is_empty() && v != "0"),
        }
    }

    pub fn color_enabled(&self) -> bool {
        self.color_forced || (self.stdout_is_tty && !self.no_color)
    }

    pub fn can_prompt(&self) -> bool {
        self.stdin_is_tty && self.stderr_is_tty
    }
}
