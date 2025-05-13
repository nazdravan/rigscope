use crate::fsutil::{list_dir, map_io, read_capped, MAX_DIR_ENTRIES};
use crate::{ReadError, Source};
use std::path::PathBuf;

/// File in a fixture root listing its symlinks, one per line: `<link> TAB <target>`, both absolute
/// fixture paths (`/sys/block/sda TAB /sys/devices/pci0000:00/.../block/sda`). Git cannot carry the
/// real symlinks to every OS, so the fixtures keep them as data.
pub const LINKS_FILE: &str = "links.txt";

/// Most lines read from [`LINKS_FILE`].
const MAX_LINKS: usize = 10_000;

/// Most link replacements while resolving one path; more means a loop.
const MAX_HOPS: usize = 40;

/// One path component as stored on disk: '%' becomes "%25", ':' becomes "%3A" (portable to NTFS).
pub fn encode_component(c: &str) -> String {
    c.replace('%', "%25").replace(':', "%3A")
}

/// Inverse of [`encode_component`].
pub fn decode_component(c: &str) -> String {
    c.replace("%3A", ":").replace("%25", "%")
}

fn components(path: &str) -> Result<Vec<&str>, ReadError> {
    let mut out = Vec::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        if part == ".." || part == "." {
            return Err(ReadError::NotFound);
        }
        out.push(part);
    }
    Ok(out)
}

fn join(parts: &[&str]) -> String {
    format!("/{}", parts.join("/"))
}

/// Reads absolute paths below a fixture directory (`<root>/proc/cpuinfo`, ...). Names on disk
/// are encoded with [`encode_component`] and symlinks come from [`LINKS_FILE`].
#[derive(Debug, Clone)]
pub struct FixtureSource {
    root: PathBuf,
    /// (link, target), both normalized absolute fixture paths.
    links: Vec<(String, String)>,
}

impl FixtureSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let links = std::fs::read_to_string(root.join(LINKS_FILE))
            .map(|text| parse_links(&text))
            .unwrap_or_default();
        FixtureSource { root, links }
    }

    /// Replaces the longest linked prefix until none is left; `..` components are refused.
    fn resolve_links(&self, path: &str) -> Result<String, ReadError> {
        let mut current = join(&components(path)?);
        for _ in 0..MAX_HOPS {
            let hit = self
                .links
                .iter()
                .filter(|(link, _)| is_prefix(link, &current))
                .max_by_key(|(link, _)| link.len());
            match hit {
                None => return Ok(current),
                Some((link, target)) => {
                    let rest = current.get(link.len()..).unwrap_or("");
                    current = format!("{}{rest}", target.trim_end_matches('/'));
                    if current.is_empty() {
                        current.push('/');
                    }
                }
            }
        }
        Err(ReadError::NotFound)
    }

    /// The on-disk location of an already resolved fixture path.
    fn on_disk(&self, resolved: &str) -> PathBuf {
        let mut out = self.root.clone();
        for part in resolved.split('/').filter(|p| !p.is_empty()) {
            out.push(encode_component(part));
        }
        out
    }
}

/// True when `link` is `path` or one of its parent directories, by whole components.
fn is_prefix(link: &str, path: &str) -> bool {
    path.strip_prefix(link)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/') || link == "/")
}

fn parse_links(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines().take(MAX_LINKS) {
        let Some((link, target)) = line.split_once('\t') else {
            continue;
        };
        let (Ok(link), Ok(target)) = (components(link), components(target)) else {
            continue;
        };
        if !link.is_empty() {
            out.push((join(&link), join(&target)));
        }
    }
    out
}

impl Source for FixtureSource {
    fn read(&self, path: &str) -> Result<Vec<u8>, ReadError> {
        read_capped(&self.on_disk(&self.resolve_links(path)?))
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, ReadError> {
        let asked = join(&components(dir)?);
        let resolved = self.resolve_links(dir)?;
        let (mut names, mut found) = match list_dir(&self.on_disk(&resolved)) {
            Ok(names) => (names.iter().map(|n| decode_component(n)).collect(), true),
            Err(ReadError::NotFound) => (Vec::new(), false),
            Err(e) => return Err(e),
        };
        if resolved == "/" {
            names.retain(|n: &String| n != LINKS_FILE);
        }
        for (link, _) in &self.links {
            if let Some((parent, name)) = link.rsplit_once('/') {
                let parent = if parent.is_empty() { "/" } else { parent };
                if parent == asked || parent == resolved {
                    names.push(name.to_string());
                    found = true;
                }
            }
        }
        if !found {
            return Err(ReadError::NotFound);
        }
        names.sort();
        names.dedup();
        names.truncate(MAX_DIR_ENTRIES);
        Ok(names)
    }

    fn canonical(&self, path: &str) -> Result<String, ReadError> {
        let resolved = self.resolve_links(path)?;
        // Probe through the filesystem so a dangling link is reported as missing.
        std::fs::metadata(self.on_disk(&resolved)).map_err(map_io)?;
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::Source;

    fn scratch(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("rigscope-fixture-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn components_round_trip() {
        assert_eq!(encode_component("pci0000:00"), "pci0000%3A00");
        assert_eq!(encode_component("50%"), "50%25");
        assert_eq!(decode_component("pci0000%3A00"), "pci0000:00");
        assert_eq!(decode_component(&encode_component("a:%:b")), "a:%:b");
    }

    #[test]
    fn links_and_encoded_names_behave_like_sysfs() {
        let root = scratch("links");
        let dev = root.join("sys/devices/pci0000%3A00/0000%3A00%3A17.0/block/sda");
        std::fs::create_dir_all(&dev).unwrap();
        std::fs::write(dev.join("size"), "100\n").unwrap();
        std::fs::create_dir_all(root.join("sys/block")).unwrap();
        std::fs::write(
            root.join(LINKS_FILE),
            "/sys/block/sda\t/sys/devices/pci0000:00/0000:00:17.0/block/sda\n",
        )
        .unwrap();
        let src = FixtureSource::new(&root);
        assert_eq!(src.read("/sys/block/sda/size").unwrap(), b"100\n");
        assert_eq!(
            src.read("/sys/devices/pci0000:00/0000:00:17.0/block/sda/size")
                .unwrap(),
            b"100\n"
        );
        assert_eq!(src.list("/sys/block").unwrap(), vec!["sda"]);
        assert_eq!(src.list("/sys/devices").unwrap(), vec!["pci0000:00"]);
        assert_eq!(
            src.canonical("/sys/block/sda").unwrap(),
            "/sys/devices/pci0000:00/0000:00:17.0/block/sda"
        );
        assert_eq!(src.read("/sys/block/sdab/size"), Err(ReadError::NotFound));
        assert!(!src.list("/").unwrap().contains(&LINKS_FILE.to_string()));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn link_loops_end_in_not_found() {
        let root = scratch("loop");
        std::fs::write(root.join(LINKS_FILE), "/a\t/b\n/b\t/a\n").unwrap();
        assert_eq!(
            FixtureSource::new(&root).read("/a/x"),
            Err(ReadError::NotFound)
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
