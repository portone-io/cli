use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub struct Cache {
    dir: PathBuf,
    ttl: Duration,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope {
    status: u16,
    headers: Vec<(String, String)>,
}

impl Cache {
    pub fn new(ttl: Duration) -> Cache {
        Cache::with_dir(crate::config::paths::cache_dir().join("api"), ttl)
    }

    pub fn with_dir(dir: PathBuf, ttl: Duration) -> Cache {
        Cache { dir, ttl }
    }

    pub fn key(method: &str, url: &str, accept: &str, authorization: &str, body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{method}:{url}:{accept}:{authorization}:"));
        hasher.update(body);
        let digest = hasher.finalize();

        use std::fmt::Write as _;
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    }

    pub fn lookup(&self, key: &str) -> Option<CachedResponse> {
        let path = self.entry_path(key)?;
        let modified = fs::metadata(&path).ok()?.modified().ok()?;
        let fresh = match modified.checked_add(self.ttl) {
            Some(expires) => expires > SystemTime::now(),
            None => true,
        };
        if !fresh {
            return None;
        }

        let bytes = fs::read(&path).ok()?;
        let split = bytes.iter().position(|&b| b == b'\n')?;
        let envelope: Envelope = serde_json::from_slice(&bytes[..split]).ok()?;
        Some(CachedResponse {
            status: envelope.status,
            headers: envelope.headers,
            body: bytes[split + 1..].to_vec(),
        })
    }

    pub fn store(&self, key: &str, cacheable_method: bool, response: &CachedResponse) {
        if !cacheable_method {
            return;
        }
        if response.status >= 500 || response.status == 403 {
            return;
        }
        let Some(path) = self.entry_path(key) else {
            return;
        };
        let _ = write_entry(&path, response);
    }

    fn entry_path(&self, key: &str) -> Option<PathBuf> {
        let prefix = key.get(..2)?;
        let rest = key.get(2..)?;
        if rest.is_empty() {
            return None;
        }
        Some(self.dir.join(prefix).join(rest))
    }
}

fn write_entry(path: &Path, response: &CachedResponse) -> std::io::Result<()> {
    let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o755)
            .create(parent)?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(parent)?;

    let envelope = Envelope {
        status: response.status,
        headers: response.headers.clone(),
    };
    let mut contents = serde_json::to_vec(&envelope)?;
    contents.push(b'\n');
    contents.extend_from_slice(&response.body);

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(&contents)
}
