//! Packs a fixture directory into one blob (at build time, from `build.rs`) and unpacks it into a
//! directory (at run time), so a binary built on macOS can show the demo machine without the
//! source tree. Symlinks are kept, because the fixtures use them like /sys does.
//!
//! Blob: repeated entries `kind:u8 | path_len:u32 | path | data_len:u32 | data`, integers
//! little-endian, kind 0 = file (data = contents), 1 = symlink (data = relative target).

use std::io;
use std::path::{Component, Path, PathBuf};

const FILE: u8 = 0;
const LINK: u8 = 1;

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

fn put(out: &mut Vec<u8>, kind: u8, path: &str, data: &[u8]) -> io::Result<()> {
    let plen = u32::try_from(path.len()).map_err(|_| bad("path too long"))?;
    let dlen = u32::try_from(data.len()).map_err(|_| bad("file too large"))?;
    out.push(kind);
    out.extend_from_slice(&plen.to_le_bytes());
    out.extend_from_slice(path.as_bytes());
    out.extend_from_slice(&dlen.to_le_bytes());
    out.extend_from_slice(data);
    Ok(())
}

/// Packs everything under `root` (sorted, so the blob is reproducible).
#[cfg_attr(not(test), allow(dead_code))]
pub fn pack(root: &Path) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    walk(root, "", &mut out)?;
    Ok(out)
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<u8>) -> io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| bad("file name is not UTF-8"))?;
        let path = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            let target = std::fs::read_link(entry.path())?;
            let target = target.to_str().ok_or_else(|| bad("link is not UTF-8"))?;
            put(out, LINK, &path, target.as_bytes())?;
        } else if ty.is_dir() {
            walk(&entry.path(), &path, out)?;
        } else {
            put(out, FILE, &path, &std::fs::read(entry.path())?)?;
        }
    }
    Ok(())
}

fn take<'a>(blob: &'a [u8], at: &mut usize, n: usize) -> io::Result<&'a [u8]> {
    let end = at.checked_add(n).ok_or_else(|| bad("overflow"))?;
    let part = blob.get(*at..end).ok_or_else(|| bad("truncated blob"))?;
    *at = end;
    Ok(part)
}

fn take_u32(blob: &[u8], at: &mut usize) -> io::Result<usize> {
    let bytes: [u8; 4] = take(blob, at, 4)?
        .try_into()
        .map_err(|_| bad("truncated blob"))?;
    usize::try_from(u32::from_le_bytes(bytes)).map_err(|_| bad("overflow"))
}

/// Only plain relative components: no `..`, no root, no empty path.
fn safe_relative(path: &str) -> io::Result<PathBuf> {
    let p = Path::new(path);
    let ok = p.components().next().is_some()
        && p.components().all(|c| matches!(c, Component::Normal(_)));
    if ok {
        Ok(p.to_path_buf())
    } else {
        Err(bad("unsafe path in the blob"))
    }
}

/// Recreates the packed tree under `dest` (which must exist).
pub fn unpack(blob: &[u8], dest: &Path) -> io::Result<()> {
    let mut at = 0;
    while at < blob.len() {
        let kind = *take(blob, &mut at, 1)?
            .first()
            .ok_or_else(|| bad("empty"))?;
        let plen = take_u32(blob, &mut at)?;
        let path = std::str::from_utf8(take(blob, &mut at, plen)?)
            .map_err(|_| bad("path is not UTF-8"))?;
        let dlen = take_u32(blob, &mut at)?;
        let data = take(blob, &mut at, dlen)?;
        let target = dest.join(safe_relative(path)?);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match kind {
            FILE => std::fs::write(&target, data)?,
            LINK => {
                let link = std::str::from_utf8(data).map_err(|_| bad("link is not UTF-8"))?;
                if Path::new(link).is_absolute() {
                    return Err(bad("absolute link in the blob"));
                }
                make_link(link, &target)?;
            }
            _ => return Err(bad("unknown entry kind")),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_link(link: &str, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(link, target)
}

#[cfg(not(unix))]
fn make_link(_link: &str, _target: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "symlinks are not supported here",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(root: &Path) -> Vec<(String, String)> {
        // (relative path, contents or "-> target"), sorted.
        fn go(dir: &Path, rel: &str, out: &mut Vec<(String, String)>) {
            let mut es: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .map(|e| e.unwrap())
                .collect();
            es.sort_by_key(|e| e.file_name());
            for e in es {
                let p = format!("{rel}/{}", e.file_name().to_string_lossy());
                let ty = e.file_type().unwrap();
                if ty.is_symlink() {
                    let t = std::fs::read_link(e.path()).unwrap();
                    out.push((p, format!("-> {}", t.display())));
                } else if ty.is_dir() {
                    go(&e.path(), &p, out);
                } else {
                    let b = std::fs::read(e.path()).unwrap();
                    out.push((
                        p,
                        format!("{} bytes {:x?}", b.len(), b.get(..8.min(b.len()))),
                    ));
                }
            }
        }
        let mut out = Vec::new();
        go(root, "", &mut out);
        out
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rigscope-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_demo_fixture_survives_a_round_trip() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/desktop-ddr4");
        let blob = pack(&src).unwrap();
        let dest = scratch("roundtrip");
        unpack(&blob, &dest).unwrap();
        let (a, b) = (tree(&src), tree(&dest));
        let _ = std::fs::remove_dir_all(&dest);
        assert!(a.len() > 100, "fixture was found");
        assert_eq!(a, b);
    }

    #[test]
    fn unsafe_paths_are_refused() {
        let dest = scratch("unsafe");
        for path in ["../x", "/etc/x", "a/../../x", ""] {
            let mut blob = Vec::new();
            put(&mut blob, FILE, path, b"x").unwrap();
            assert!(unpack(&blob, &dest).is_err(), "{path:?}");
        }
        let mut blob = Vec::new();
        put(&mut blob, LINK, "l", b"/etc").unwrap();
        assert!(unpack(&blob, &dest).is_err(), "absolute link");
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn truncated_or_garbage_blobs_do_not_panic() {
        let dest = scratch("garbage");
        let mut blob = Vec::new();
        put(&mut blob, FILE, "a/b", b"hello").unwrap();
        for cut in 0..blob.len() {
            let _ = unpack(blob.get(..cut).unwrap_or(&[]), &dest);
        }
        assert!(unpack(&[9, 0, 0, 0, 0, 0, 0, 0, 0], &dest).is_err());
        let _ = std::fs::remove_dir_all(&dest);
    }
}
