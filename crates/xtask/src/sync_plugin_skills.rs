use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

struct SkillSync {
    source: &'static str,
    destinations: &'static [&'static str],
}

const SKILL_SYNCS: &[SkillSync] = &[
    SkillSync {
        source: "skills/portone-cli",
        destinations: &[
            "plugins/portone-codex/skills/portone-cli",
            "plugins/portone-integration/skills/portone-cli",
        ],
    },
    SkillSync {
        source: "skills/portone-guide",
        destinations: &[
            "plugins/portone-codex/skills/portone-guide",
            "plugins/portone-integration/skills/portone-guide",
        ],
    },
    SkillSync {
        source: "skills/payment-code-generator",
        destinations: &["plugins/portone-codex/skills/payment-code-generator"],
    },
    SkillSync {
        source: "skills/integration-validator",
        destinations: &["plugins/portone-codex/skills/integration-validator"],
    },
];

#[derive(Debug, Eq, PartialEq)]
enum Entry {
    Directory,
    File(Vec<u8>),
}

impl Entry {
    fn same_kind(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Directory, Self::Directory) | (Self::File(_), Self::File(_))
        )
    }
}

type Tree = BTreeMap<PathBuf, Entry>;

#[derive(Debug, Eq, PartialEq)]
pub struct Difference {
    kind: &'static str,
    path: PathBuf,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.path.display())
    }
}

/// Check every tree before writing, so unsupported entries cannot produce a
/// partially synchronized bundle. Only the generated skill roots are managed.
pub fn run(workspace: &Path, check: bool) -> io::Result<Vec<Difference>> {
    let syncs = SKILL_SYNCS
        .iter()
        .map(|sync| {
            let source = workspace.join(sync.source);
            let expected = read_tree(&source)?;
            if expected.get(Path::new("")) != Some(&Entry::Directory) {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("canonical skill directory not found: {}", source.display()),
                ));
            }
            let destinations = sync
                .destinations
                .iter()
                .map(|destination| {
                    let path = workspace.join(destination);
                    let actual = read_tree(&path)?;
                    Ok((path, actual))
                })
                .collect::<io::Result<Vec<_>>>()?;
            Ok((expected, destinations))
        })
        .collect::<io::Result<Vec<_>>>()?;

    let mut differences = Vec::new();
    for (expected, destinations) in &syncs {
        for (path, actual) in destinations {
            for (relative, entry) in expected {
                let kind = match actual.get(relative) {
                    None => "missing",
                    Some(existing) if existing != entry => "different",
                    Some(_) => continue,
                };
                differences.push(Difference {
                    kind,
                    path: entry_path(path, relative),
                });
            }
            for relative in actual.keys().filter(|key| !expected.contains_key(*key)) {
                differences.push(Difference {
                    kind: "stale",
                    path: entry_path(path, relative),
                });
            }
        }
    }
    differences.sort_by(|left, right| left.path.cmp(&right.path));

    if !check {
        for (expected, destinations) in &syncs {
            for (path, actual) in destinations {
                write_tree(path, expected, actual)?;
            }
        }
    }
    Ok(differences)
}

fn read_tree(root: &Path) -> io::Result<Tree> {
    let mut tree = Tree::new();
    match fs::symlink_metadata(root) {
        Ok(_) => read_entry(root, Path::new(""), &mut tree)?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    Ok(tree)
}

fn read_entry(root: &Path, relative: &Path, tree: &mut Tree) -> io::Result<()> {
    let path = entry_path(root, relative);
    let kind = fs::symlink_metadata(&path)?.file_type();
    if kind.is_file() {
        tree.insert(relative.to_path_buf(), Entry::File(fs::read(&path)?));
    } else if kind.is_dir() {
        tree.insert(relative.to_path_buf(), Entry::Directory);
        for child in fs::read_dir(path)? {
            read_entry(root, &relative.join(child?.file_name()), tree)?;
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "only regular files and directories are supported (no symlinks): {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn entry_path(root: &Path, relative: &Path) -> PathBuf {
    // Joining an empty path adds a trailing separator, which follows a root
    // symlink and fails for a root file on Unix.
    if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

fn write_tree(root: &Path, expected: &Tree, actual: &Tree) -> io::Result<()> {
    // Descendants precede parents so obsolete directories are empty when removed.
    for (relative, entry) in actual.iter().rev() {
        if expected
            .get(relative)
            .is_some_and(|desired| desired.same_kind(entry))
        {
            continue;
        }
        let path = entry_path(root, relative);
        match entry {
            Entry::Directory => fs::remove_dir(path)?,
            Entry::File(_) => fs::remove_file(path)?,
        }
    }
    for (relative, entry) in expected {
        if actual.get(relative) == Some(entry) {
            continue;
        }
        let path = entry_path(root, relative);
        match entry {
            Entry::Directory => fs::create_dir_all(path)?,
            Entry::File(content) => fs::write(path, content)?,
        }
    }
    Ok(())
}
