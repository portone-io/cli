use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use portone_cli::i18n::{Language, Localizer};

pub fn run(dir: &Path, check_only: bool) -> io::Result<Vec<PathBuf>> {
    let mut stale = Vec::new();
    for (language, subdir) in [(Language::English, ""), (Language::Korean, "ko")] {
        let dir = dir.join(subdir);
        let pages = render_all(language);
        if check_only {
            stale.extend(check(&dir, &pages)?);
        } else {
            write(&dir, &pages)?;
        }
    }
    stale.sort();
    Ok(stale)
}

fn render_all(language: Language) -> BTreeMap<String, String> {
    let mut root = portone_cli::cmd::help::command(&Localizer::new(language));
    root.build();
    let mut pages = BTreeMap::new();
    walk(&mut root, vec!["portone".to_string()], &mut pages, language);
    let index = render_index(&pages, language);
    pages.insert("index.md".to_string(), index);
    pages
}

fn walk(
    cmd: &mut Command,
    path: Vec<String>,
    pages: &mut BTreeMap<String, String>,
    language: Language,
) {
    let page = render_page(cmd, &path, language);
    pages.insert(format!("{}.md", path.join("_")), page);
    for sub in cmd.get_subcommands_mut() {
        if sub.get_name() == "help" || sub.is_hide_set() {
            continue;
        }
        let mut child_path = path.clone();
        child_path.push(sub.get_name().to_string());
        walk(sub, child_path, pages, language);
    }
}

fn text(language: Language, english: &'static str, korean: &'static str) -> &'static str {
    match language {
        Language::English => english,
        Language::Korean => korean,
    }
}

fn language_links(file: &str, language: Language) -> String {
    match language {
        Language::English => format!("English | [한국어](ko/{file})"),
        Language::Korean => format!("[English](../{file}) | 한국어"),
    }
}

fn render_page(cmd: &mut Command, path: &[String], language: Language) -> String {
    let full_name = path.join(" ");
    let mut page = String::new();
    let _ = writeln!(page, "# {full_name}");
    let _ = writeln!(
        page,
        "\n{}",
        language_links(&format!("{}.md", path.join("_")), language)
    );
    let description = text(language, "Description", "설명");

    let about = cmd
        .get_long_about()
        .or_else(|| cmd.get_about())
        .map(|text| text.to_string());
    if let Some(about) = about {
        let _ = writeln!(page, "\n{about}");
    }

    let usage = cmd.render_usage().to_string();
    let usage = usage.strip_prefix("Usage: ").unwrap_or(&usage).to_string();
    let _ = writeln!(page, "\n```\n{usage}\n```");

    let subcommands: Vec<(String, String)> = cmd
        .get_subcommands()
        .filter(|sub| sub.get_name() != "help" && !sub.is_hide_set())
        .map(|sub| {
            (
                sub.get_name().to_string(),
                sub.get_about().map(|s| s.to_string()).unwrap_or_default(),
            )
        })
        .collect();
    if !subcommands.is_empty() {
        let _ = writeln!(
            page,
            "\n## {}\n\n| {} | {description} |\n| --- | --- |",
            text(language, "Commands", "명령어"),
            text(language, "Command", "명령어")
        );
        for (name, about) in subcommands {
            let _ = writeln!(
                page,
                "| [{full_name} {name}]({}_{name}.md) | {} |",
                path.join("_"),
                escape_cell(&about)
            );
        }
    }

    let positionals: Vec<&Arg> = cmd
        .get_positionals()
        .filter(|arg| !arg.is_hide_set())
        .collect();
    if !positionals.is_empty() {
        let _ = writeln!(
            page,
            "\n## {}\n\n| {} | {description} |\n| --- | --- |",
            text(language, "Arguments", "인자"),
            text(language, "Argument", "인자")
        );
        for arg in positionals {
            let _ = writeln!(
                page,
                "| `<{}>` | {} |",
                value_name(arg),
                escape_cell(&help_text(arg, language))
            );
        }
    }

    let options: Vec<&Arg> = cmd
        .get_arguments()
        .filter(|arg| !arg.is_positional() && !arg.is_hide_set() && !is_builtin(arg))
        .collect();
    if !options.is_empty() {
        let _ = writeln!(
            page,
            "\n## {}\n\n| {} | {description} |\n| --- | --- |",
            text(language, "Options", "옵션"),
            text(language, "Option", "옵션")
        );
        for arg in options {
            let _ = writeln!(
                page,
                "| `{}` | {} |",
                option_syntax(arg),
                escape_cell(&help_text(arg, language))
            );
        }
    }

    if let Some(examples) = cmd.get_after_long_help() {
        let examples = examples.to_string();
        let _ = writeln!(
            page,
            "\n## {}\n\n```sh\n{}\n```",
            text(language, "Examples", "예제"),
            examples.trim_end()
        );
    }

    if path.len() > 1 {
        let parent = &path[..path.len() - 1];
        let _ = writeln!(
            page,
            "\n## {}\n\n- [{}]({}.md)",
            text(language, "See also", "참고"),
            parent.join(" "),
            parent.join("_")
        );
    }

    page
}

fn render_index(pages: &BTreeMap<String, String>, language: Language) -> String {
    let mut index = String::new();
    let _ = writeln!(
        index,
        "# {}",
        text(language, "PortOne CLI reference", "PortOne CLI 명령어 참조")
    );
    let _ = writeln!(index, "\n{}", language_links("index.md", language));
    let _ = writeln!(index);
    for file in pages.keys() {
        let name = file.trim_end_matches(".md").replace('_', " ");
        let _ = writeln!(index, "- [{name}]({file})");
    }
    index
}

fn write(dir: &Path, pages: &BTreeMap<String, String>) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if is_unknown_md(&path, pages) {
            fs::remove_file(&path)?;
        }
    }
    for (file, content) in pages {
        fs::write(dir.join(file), content)?;
    }
    Ok(())
}

fn check(dir: &Path, pages: &BTreeMap<String, String>) -> io::Result<Vec<PathBuf>> {
    let mut stale = Vec::new();
    for (file, content) in pages {
        let path = dir.join(file);
        match fs::read_to_string(&path) {
            Ok(existing) => {
                if existing.replace("\r\n", "\n") != *content {
                    stale.push(path);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => stale.push(path),
            Err(err) => return Err(err),
        }
    }
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if is_unknown_md(&path, pages) {
                stale.push(path);
            }
        }
    }
    stale.sort();
    Ok(stale)
}

fn is_unknown_md(path: &Path, pages: &BTreeMap<String, String>) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
        && !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| pages.contains_key(name))
}

fn is_builtin(arg: &Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

fn option_syntax(arg: &Arg) -> String {
    let mut syntax = String::new();
    if let Some(short) = arg.get_short() {
        let _ = write!(syntax, "-{short}");
    }
    if let Some(long) = arg.get_long() {
        if !syntax.is_empty() {
            syntax.push_str(", ");
        }
        let _ = write!(syntax, "--{long}");
    }
    if arg.get_action().takes_values() {
        if arg
            .get_num_args()
            .is_some_and(|range| range.min_values() == 0)
        {
            let _ = write!(syntax, " [<{}>]", value_name(arg));
        } else {
            let _ = write!(syntax, " <{}>", value_name(arg));
        }
    }
    syntax
}

fn value_name(arg: &Arg) -> String {
    arg.get_value_names()
        .and_then(|names| names.first())
        .map(|name| name.to_string())
        .unwrap_or_else(|| arg.get_id().to_string().to_uppercase())
}

fn help_text(arg: &Arg, language: Language) -> String {
    let mut help = arg
        .get_help()
        .map(|help| help.to_string())
        .unwrap_or_default();
    if arg.get_action().takes_values() && !arg.is_hide_possible_values_set() {
        let values: Vec<String> = arg
            .get_possible_values()
            .iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_string())
            .collect();
        if !values.is_empty() {
            if !help.is_empty() {
                help.push(' ');
            }
            let _ = write!(
                help,
                "[{}: {}]",
                text(language, "possible values", "가능한 값"),
                values.join(", ")
            );
        }
    }
    help
}

fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct OutputDir(PathBuf);

    impl OutputDir {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "portone-gen-docs-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for OutputDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn index_links_root_and_nested_commands_without_help_pages() {
        let pages = render_all(Language::English);
        let index = &pages["index.md"];
        assert!(index.contains("[portone](portone.md)"));
        assert!(
            index.contains("[portone payment webhook resend](portone_payment_webhook_resend.md)")
        );
        assert!(pages.keys().all(|file| !file.contains("_help")));
    }

    #[test]
    fn api_page_lists_flattened_auth_options() {
        let pages = render_all(Language::English);
        let api = &pages["portone_api.md"];
        assert!(api.contains("`-X, --method <METHOD>`"));
        assert!(api.contains("`--profile <NAME>`"));
        assert!(api.contains("`<ENDPOINT>`"));
        assert!(!api.contains("--help"));
    }

    #[test]
    fn nested_payment_pages_include_full_paths_and_inherited_options() {
        let pages = render_all(Language::English);
        let webhook = &pages["portone_payment_webhook_resend.md"];
        assert!(webhook.starts_with("# portone payment webhook resend\n"));
        assert!(
            webhook.contains("portone payment webhook resend [OPTIONS] <PAYMENT_ID>"),
            "{webhook}"
        );
        assert!(webhook.contains("`--webhook-id <WEBHOOK_ID>`"));
        assert!(webhook.contains("`--store <STORE_ID>`"));
        assert!(webhook.contains("`--profile <NAME>`"));
        assert!(webhook.contains("`--json [<FIELDS>]`"));
        assert!(webhook.contains("[portone payment webhook](portone_payment_webhook.md)"));
        let list = &pages["portone_payment_list.md"];
        assert!(list.contains("`--version <VERSION>`"));
        assert!(list.contains("`-L, --limit <LIMIT>`"));
    }

    #[test]
    fn api_page_wraps_examples_in_code_fence() {
        let pages = render_all(Language::English);
        let api = &pages["portone_api.md"];
        assert!(api.contains("\n## Examples\n\n```sh\n# "), "{api}");
        assert!(api.contains("$ portone api graphql"));
        assert!(api.contains("```\n\n## See also"), "{api}");
    }

    #[test]
    fn cells_escape_pipes() {
        let pages = render_all(Language::English);
        let setup = &pages["portone_setup.md"];
        assert!(setup.contains("(claude \\| codex \\| both)"));
    }

    #[test]
    fn completion_page_lists_possible_shells() {
        let pages = render_all(Language::English);
        let completion = &pages["portone_completion.md"];
        assert!(
            completion.contains("[possible values: bash, elvish, fish, powershell, zsh]"),
            "{completion}"
        );
    }

    fn executable_lines(page: &str) -> Vec<&str> {
        let mut in_code = false;
        page.lines()
            .filter(|line| {
                if line.starts_with("```") {
                    in_code = !in_code;
                    return false;
                }
                in_code && !line.trim_start().starts_with('#') && !line.trim().is_empty()
            })
            .collect()
    }

    #[test]
    fn translations_preserve_pages_command_syntax_and_executable_examples() {
        let english = render_all(Language::English);
        let korean = render_all(Language::Korean);
        assert_eq!(
            english.keys().collect::<Vec<_>>(),
            korean.keys().collect::<Vec<_>>()
        );
        for (file, english_page) in &english {
            let korean_page = &korean[file];
            assert_eq!(
                executable_lines(english_page),
                executable_lines(korean_page),
                "{file}"
            );
            // The first table cell contains command links, argument names, or flag syntax.
            let syntax = |page: &str| -> Vec<String> {
                page.lines()
                    .filter(|line| line.starts_with("| `") || line.starts_with("| ["))
                    .map(|line| line.split(" | ").next().unwrap().to_string())
                    .collect()
            };
            assert_eq!(syntax(english_page), syntax(korean_page), "{file}");
            assert!(!english_page.contains('\u{1b}'), "{file}");
            assert!(!korean_page.contains('\u{1b}'), "ko/{file}");
        }
    }

    #[test]
    fn korean_pages_translate_labels_descriptions_and_metadata() {
        let pages = render_all(Language::Korean);
        assert!(pages["index.md"].starts_with("# PortOne CLI 명령어 참조\n"));
        let root = &pages["portone.md"];
        assert!(root.contains("## 명령어\n\n| 명령어 | 설명 |"));
        assert!(root.contains("PortOne 인증 관리"));
        let api = &pages["portone_api.md"];
        assert!(api.contains("## 인자\n\n| 인자 | 설명 |"));
        assert!(api.contains("## 옵션\n\n| 옵션 | 설명 |"));
        assert!(api.contains("## 예제\n\n```sh\n# 결제 조회"));
        assert!(api.contains("## 참고"));
        let completion = &pages["portone_completion.md"];
        assert_eq!(
            completion
                .matches("[가능한 값: bash, elvish, fish, powershell, zsh]")
                .count(),
            1
        );
        assert!(!completion.contains("possible values"));
    }

    #[test]
    fn generated_links_resolve_within_custom_output_directory() {
        let dir = OutputDir::new();
        run(&dir.0, false).unwrap();
        for (language, subdir) in [(Language::English, ""), (Language::Korean, "ko")] {
            for (file, page) in render_all(language) {
                let language_link = match language {
                    Language::English => format!("[한국어](ko/{file})"),
                    Language::Korean => format!("[English](../{file})"),
                };
                assert!(page.contains(&language_link), "{subdir}/{file}");
                for link in page.split("](").skip(1) {
                    let target = link.split(')').next().unwrap();
                    if target.contains("://") {
                        continue;
                    }
                    assert!(
                        dir.0.join(subdir).join(target).is_file(),
                        "{subdir}/{file}: {target}"
                    );
                }
            }
        }
    }

    #[test]
    fn check_reports_missing_locales_without_creating_output() {
        let dir = OutputDir::new();
        let output = dir.0.join("reference");
        let stale = run(&output, true).unwrap();
        assert_eq!(stale.len(), render_all(Language::English).len() * 2);
        assert!(stale.contains(&output.join("index.md")));
        assert!(stale.contains(&output.join("ko/index.md")));
        assert!(!output.exists());
    }

    #[test]
    fn generation_repairs_missing_changed_and_obsolete_pages_in_both_languages() {
        let dir = OutputDir::new();
        run(&dir.0, false).unwrap();
        assert!(run(&dir.0, true).unwrap().is_empty());
        let mut expected = Vec::new();
        for subdir in ["", "ko"] {
            let output = dir.0.join(subdir);
            let missing = output.join("portone.md");
            let changed = output.join("index.md");
            let obsolete = output.join("obsolete.md");
            fs::remove_file(&missing).unwrap();
            fs::write(&changed, "outdated").unwrap();
            fs::write(&obsolete, "obsolete command").unwrap();
            fs::write(output.join("notes.txt"), "keep this file").unwrap();
            expected.extend([missing, changed, obsolete]);
        }
        expected.sort();
        assert_eq!(run(&dir.0, true).unwrap(), expected);
        run(&dir.0, false).unwrap();
        assert!(run(&dir.0, true).unwrap().is_empty());
        for subdir in ["", "ko"] {
            let output = dir.0.join(subdir);
            assert!(!output.join("obsolete.md").exists());
            assert_eq!(
                fs::read_to_string(output.join("notes.txt")).unwrap(),
                "keep this file"
            );
            // Generated files checked out with CRLF remain current on Windows.
            let index = output.join("index.md");
            let contents = fs::read_to_string(&index).unwrap().replace('\n', "\r\n");
            fs::write(index, contents).unwrap();
        }
        assert!(run(&dir.0, true).unwrap().is_empty());
    }
}
