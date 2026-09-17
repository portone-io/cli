pub mod jsoncolor;
pub mod resource;

use std::io::Write;

use anyhow::anyhow;
use jaq_json::Val;
use serde_json::Value;

use crate::error::CliError;

type JqData = jaq_core::data::JustLut<Val>;
type JqFilter = jaq_core::Filter<JqData>;

pub struct Pipeline {
    jq: Option<JqFilter>,
    slurp_pages: Option<Vec<Value>>,
    color: bool,
    tty: bool,
}

impl Pipeline {
    pub fn new(jq: Option<&str>, slurp: bool, color: bool, tty: bool) -> anyhow::Result<Pipeline> {
        Ok(Pipeline {
            jq: jq.map(compile_jq).transpose()?,
            slurp_pages: slurp.then(Vec::new),
            color,
            tty,
        })
    }

    pub fn emit_json(&mut self, w: &mut dyn Write, bytes: &[u8]) -> Result<(), CliError> {
        if let Some(filter) = &self.jq {
            run_jq(filter, w, bytes, self.color, self.tty)
        } else if let Some(pages) = &mut self.slurp_pages {
            let value: Value = serde_json::from_slice(bytes).map_err(|e| {
                CliError::Other(anyhow!(crate::message!("core-response-parse", error = e)))
            })?;
            pages.push(value);
            Ok(())
        } else {
            emit_json_plain(w, bytes, self.color)
        }
    }

    pub fn finish(&mut self, w: &mut dyn Write) -> Result<(), CliError> {
        let Some(pages) = self.slurp_pages.take() else {
            return Ok(());
        };
        let array = Value::Array(pages);
        if self.color {
            jsoncolor::write_colored(w, &array, "  ", 0)?;
        } else if self.tty {
            serde_json::to_writer_pretty(&mut *w, &array).map_err(|e| CliError::Other(e.into()))?;
            writeln!(w)?;
        } else {
            serde_json::to_writer(&mut *w, &array).map_err(|e| CliError::Other(e.into()))?;
            writeln!(w)?;
        }
        Ok(())
    }
}

pub fn emit_json_plain(w: &mut dyn Write, bytes: &[u8], color: bool) -> Result<(), CliError> {
    if color {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| {
            CliError::Other(anyhow!(crate::message!("core-response-parse", error = e)))
        })?;
        jsoncolor::write_colored(w, &value, "  ", 0)?;
    } else {
        w.write_all(bytes)?;
    }
    Ok(())
}

pub fn emit_raw(
    w: &mut dyn Write,
    bytes: &[u8],
    tty: bool,
    allow_escape: bool,
) -> Result<(), CliError> {
    if !allow_escape {
        let head = &bytes[..bytes.len().min(512)];
        if head.contains(&0) {
            if tty {
                return Err(CliError::Other(anyhow!(crate::message!(
                    "core-output-binary"
                ))));
            }
        } else if bytes.contains(&0x1B) {
            return Err(CliError::Other(anyhow!(crate::message!(
                "core-output-escapes"
            ))));
        }
    }
    w.write_all(bytes)?;
    Ok(())
}

pub fn escape_controls(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        // Preserve ordinary text formatting, including separators between errors.
        if ch.is_control() && !matches!(ch, '\n' | '\t') {
            escaped.extend(ch.escape_default());
        } else {
            escaped.push(ch);
        }
    }
    escaped
}

fn compile_jq(code: &str) -> anyhow::Result<JqFilter> {
    use jaq_core::load::{Arena, File, Loader};

    let program = File { code, path: () };
    let loader = Loader::new(
        jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs()),
    );
    let arena = Arena::default();
    let modules = loader.load(&arena, program).map_err(|errs| {
        anyhow!(crate::message!(
            "core-jq-invalid",
            error = format!("{errs:?}")
        ))
    })?;
    jaq_core::Compiler::default()
        .with_funs(
            jaq_core::funs()
                .chain(jaq_std::funs())
                .chain(jaq_json::funs()),
        )
        .compile(modules)
        .map_err(|errs| {
            anyhow!(crate::message!(
                "core-jq-invalid",
                error = format!("{errs:?}")
            ))
        })
}

fn run_jq(
    filter: &JqFilter,
    w: &mut dyn Write,
    bytes: &[u8],
    color: bool,
    tty: bool,
) -> Result<(), CliError> {
    let input = jaq_json::read::parse_single(bytes)
        .map_err(|e| CliError::Other(anyhow!(crate::message!("core-response-parse", error = e))))?;
    let ctx = jaq_core::Ctx::<JqData>::new(&filter.lut, jaq_core::Vars::new([]));
    for result in filter.id.run((ctx, input)).map(jaq_core::unwrap_valr) {
        let val = result.map_err(|e| CliError::Other(anyhow!("jq: {e}")))?;
        emit_jq_val(w, &val, color, tty)?;
    }
    Ok(())
}

fn emit_jq_val(w: &mut dyn Write, val: &Val, color: bool, tty: bool) -> Result<(), CliError> {
    match val {
        Val::TStr(bytes) | Val::BStr(bytes) => {
            w.write_all(bytes.as_ref())?;
            writeln!(w)?;
        }
        Val::Null => writeln!(w)?,
        Val::Bool(_) | Val::Num(_) => writeln!(w, "{val}")?,
        Val::Arr(_) | Val::Obj(_) => {
            if color || tty {
                let value: Value = serde_json::from_str(&val.to_string()).map_err(|e| {
                    CliError::Other(anyhow!(crate::message!("core-jq-render", error = e)))
                })?;
                if color {
                    jsoncolor::write_colored(w, &value, "  ", 0)?;
                } else {
                    serde_json::to_writer_pretty(&mut *w, &value)
                        .map_err(|e| CliError::Other(e.into()))?;
                    writeln!(w)?;
                }
            } else {
                writeln!(w, "{val}")?;
            }
        }
    }
    Ok(())
}
