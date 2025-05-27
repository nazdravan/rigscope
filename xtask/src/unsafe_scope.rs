//! `unsafe` is allowed only in `rigscope-win`, and there only in the listed modules.

/// Files of `rigscope-win` that may contain `#[allow(unsafe_code)]` / `unsafe`.
pub const ALLOWED: &[&str] = &[
    "crates/rigscope-win/src/ffi/firmware.rs",
    "crates/rigscope-win/src/ffi/handle.rs",
    "crates/rigscope-win/src/ffi/registry.rs",
    "crates/rigscope-win/src/ffi/system.rs",
    "crates/rigscope-win/src/ffi/pdh.rs",
    "crates/rigscope-win/src/ffi/setupapi.rs",
    "crates/rigscope-win/src/ffi/dxgi.rs",
    "crates/rigscope-win/src/ffi/d3dkmt.rs",
    "crates/rigscope-win/src/ffi/net.rs",
];

/// Violations: a manifest without the workspace lints (outside rigscope-win), or a source file
/// mentioning `unsafe` outside the allowed list. `files` = (repo-relative path, contents).
pub fn check(manifests: &[(String, String)], files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, toml) in manifests {
        if path == "crates/rigscope-win/Cargo.toml" {
            if !toml.contains("unsafe_code = \"deny\"") {
                out.push(format!("{path}: must set unsafe_code = \"deny\""));
            }
            continue;
        }
        if !toml.contains("[lints]") || !toml.contains("workspace = true") {
            out.push(format!(
                "{path}: must use [lints] workspace = true (unsafe_code = forbid)"
            ));
        }
    }
    for (path, src) in files {
        let mentions = src.lines().any(|l| mentions_unsafe(&code_only(l)));
        if mentions && !ALLOWED.contains(&path.as_str()) {
            out.push(format!(
                "{path}: unsafe outside xtask/src/unsafe_scope.rs ALLOWED"
            ));
        }
    }
    out
}

/// The code of one line: string literals, char literals and `//` comments removed. Raw strings
/// (`r"..."`, `r#"..."#`) are treated as code.
fn code_only(line: &str) -> String {
    let c: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while let Some(&ch) = c.get(i) {
        match ch {
            '/' if c.get(i + 1) == Some(&'/') => break,
            '"' => {
                i += 1;
                while let Some(&x) = c.get(i) {
                    i += 1;
                    if x == '\\' {
                        i += 1;
                    } else if x == '"' {
                        break;
                    }
                }
                out.push(' ');
            }
            '\'' => {
                // A char literal ('x' or '\n'); anything else is a lifetime and stays.
                let end = if c.get(i + 1) == Some(&'\\') {
                    c.iter()
                        .skip(i + 2)
                        .position(|&x| x == '\'')
                        .map(|p| i + 2 + p)
                } else if c.get(i + 2) == Some(&'\'') {
                    Some(i + 2)
                } else {
                    None
                };
                if let Some(e) = end {
                    i = e + 1;
                    out.push(' ');
                } else {
                    out.push(ch);
                    i += 1;
                }
            }
            _ => {
                out.push(ch);
                i += 1;
            }
        }
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `unsafe` as a whole word, or the `allow(unsafe_code)` attribute, in already-stripped code.
fn mentions_unsafe(code: &str) -> bool {
    if code.contains("allow(unsafe_code)") {
        return true;
    }
    code.match_indices("unsafe").any(|(at, w)| {
        let before = code.get(..at).and_then(|s| s.chars().next_back());
        let after = code.get(at + w.len()..).and_then(|s| s.chars().next());
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_unsafe_outside_the_list() {
        let m = vec![(
            "crates/a/Cargo.toml".to_string(),
            "[lints]\nworkspace = true\n".to_string(),
        )];
        let ok = vec![(
            "crates/rigscope-win/src/ffi/handle.rs".to_string(),
            "unsafe { x() }".to_string(),
        )];
        assert!(check(&m, &ok).is_empty());
        let bad = vec![(
            "crates/rigscope-probe/src/x.rs".to_string(),
            "let y = unsafe { x() };".to_string(),
        )];
        assert_eq!(check(&m, &bad).len(), 1);
        let comment = vec![(
            "crates/rigscope-probe/src/x.rs".to_string(),
            "// no unsafe here".to_string(),
        )];
        assert!(check(&m, &comment).is_empty());
        let no_lints = vec![("crates/b/Cargo.toml".to_string(), "[package]\n".to_string())];
        assert_eq!(check(&no_lints, &[]).len(), 1);
    }

    #[test]
    fn looks_at_code_not_literals() {
        let m: Vec<(String, String)> = Vec::new();
        let v = |src: &str| check(&m, &[("crates/x/src/x.rs".to_string(), src.to_string())]).len();
        assert_eq!(v("Err(bad(\"unsafe path in the blob\"))"), 0);
        assert_eq!(v("let s = \"a \\\" unsafe { }\";"), 0);
        assert_eq!(v("unsafe fn f() {}"), 1);
        assert_eq!(v("#![allow(unsafe_code)]"), 1);
        assert_eq!(v("#![deny(unsafe_code)]"), 0);
        assert_eq!(v("let c = '\"'; let y = unsafe { x() };"), 1);
    }
}
