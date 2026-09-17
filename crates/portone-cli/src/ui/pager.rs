use crate::i18n::Localizer;
use std::io::{BufWriter, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

enum Output<'a> {
    Pager {
        child: Child,
        stdin: Option<BufWriter<ChildStdin>>,
    },
    Passthrough(&'a mut dyn Write),
}

pub struct Pager<'a> {
    output: Output<'a>,
}

impl<'a> Pager<'a> {
    pub fn start(
        out: &'a mut dyn Write,
        err: &mut dyn Write,
        tty: bool,
        enabled: bool,
    ) -> Pager<'a> {
        Self::start_localized(out, err, tty, enabled, crate::i18n::english())
    }

    pub fn start_localized(
        out: &'a mut dyn Write,
        err: &mut dyn Write,
        tty: bool,
        enabled: bool,
        localizer: &Localizer,
    ) -> Pager<'a> {
        if enabled && tty {
            let command = resolve_command(
                std::env::var("PORTONE_PAGER").ok(),
                std::env::var("PAGER").ok(),
            );
            if let Some(command) = command {
                match spawn_pager(&command, localizer) {
                    Ok((child, stdin)) => {
                        return Pager {
                            output: Output::Pager {
                                child,
                                stdin: Some(BufWriter::new(stdin)),
                            },
                        };
                    }
                    Err(e) => {
                        let _ = writeln!(
                            err,
                            "{}",
                            crate::tr!(localizer, "core-pager-start", error = e.to_string())
                        );
                    }
                }
            }
        }
        Pager {
            output: Output::Passthrough(out),
        }
    }

    pub fn writer(&mut self) -> &mut dyn Write {
        match &mut self.output {
            Output::Pager { stdin, .. } => stdin.as_mut().expect("pager already finished"),
            Output::Passthrough(out) => &mut **out,
        }
    }

    pub fn finish(&mut self) -> std::io::Result<()> {
        match &mut self.output {
            Output::Pager { child, stdin } => {
                let flushed = match stdin.take() {
                    Some(mut writer) => writer.flush(),
                    None => Ok(()),
                };
                let waited = child.wait();
                flushed?;
                waited.map(|_| ())
            }
            Output::Passthrough(out) => out.flush(),
        }
    }
}

impl Write for Pager<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer().flush()
    }
}

fn resolve_command(portone_pager: Option<String>, pager: Option<String>) -> Option<String> {
    let command = match portone_pager {
        Some(value) => value,
        None => pager.unwrap_or_default(),
    };
    if command.is_empty() || command == "cat" {
        None
    } else {
        Some(command)
    }
}

fn spawn_pager(pager_command: &str, localizer: &Localizer) -> std::io::Result<(Child, ChildStdin)> {
    let invalid =
        |msg: &str| std::io::Error::new(std::io::ErrorKind::InvalidInput, msg.to_string());
    let words = split_shell_words(pager_command)
        .ok_or_else(|| invalid(&crate::tr!(localizer, "core-pager-invalid")))?;
    let Some((program, args)) = words.split_first() else {
        return Err(invalid(&crate::tr!(localizer, "core-pager-empty")));
    };
    let mut command = Command::new(program);
    command.args(args);
    command.env_remove("PAGER");
    if std::env::var_os("LESS").is_none() {
        command.env("LESS", "FRX");
    }
    if std::env::var_os("LV").is_none() {
        command.env("LV", "-c");
    }
    command.stdin(Stdio::piped());
    let mut child = command.spawn()?;
    let stdin = child.stdin.take().expect("piped stdin");
    Ok((child, stdin))
}

fn split_shell_words(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => current.push(ch),
                        None => return None,
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(escaped @ ('"' | '\\' | '$' | '`')) => current.push(escaped),
                            Some(other) => {
                                current.push('\\');
                                current.push(other);
                            }
                            None => return None,
                        },
                        Some(ch) => current.push(ch),
                        None => return None,
                    }
                }
            }
            '\\' => {
                in_word = true;
                current.push(chars.next()?);
            }
            ch => {
                in_word = true;
                current.push(ch);
            }
        }
    }
    if in_word {
        words.push(current);
    }
    Some(words)
}
