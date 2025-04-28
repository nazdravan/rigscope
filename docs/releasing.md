# Releasing rigscope

A release is a version in `Cargo.toml`, a section in `CHANGELOG.md` and a push to `main`.
`.github/workflows/release.yml` does the rest.

## Steps

1. Pick the version (`0.2.0`, or `0.2.0-rc.1` for a pre-release) and set it in one place:

       packaging/version.sh set 0.2.0

   It edits `[workspace.package] version` in `Cargo.toml` and refreshes `Cargo.lock`.
2. Add a section to `CHANGELOG.md`, exactly `## [0.2.0] — YYYY-MM-DD`, with Added / Changed /
   Fixed. It becomes the release notes; check them with `packaging/changelog.sh 0.2.0`.
   Without the section the workflow stops before the builds.
3. Run `cargo xtask ci` and `cargo xtask linux`.
4. Commit and push to `main`. The workflow runs the tests; if `v0.2.0` is not published yet it
   builds every package, uploads them to a draft, writes `SHA256SUMS.txt` and publishes.

**The release commit must not change anything under `.github/workflows/`**: the workflow token
cannot push a tag that points at a commit with a modified workflow (HTTP 403). Change workflows
in an earlier commit.

A published release is never overwritten. If a published version is wrong, fix it and release
the next version.

## Test run

Actions -> Release -> Run workflow on any branch, with a version such as `0.2.0-rc.1`. It builds
and tests everything and keeps the files as workflow artifacts; it publishes nothing.

## What is built

| File | Where | Notes |
|---|---|---|
| `rigscope-<v>-linux-x86_64.deb` | Ubuntu 22.04 (glibc 2.35) | Installs `/usr/bin/rigscope`, the root-owned `/usr/libexec/rigscope/rigscope-helper` and the polkit policy, so the root-only data (SMBIOS, SMART) works. |
| `rigscope-<v>-linux-x86_64.AppImage` | Ubuntu 22.04 | One file. Cannot install the helper: run `install-helper.sh` from the `.tar.gz` once. |
| `rigscope-<v>-linux-x86_64.tar.gz` | Ubuntu 22.04 | Binary, licences, `install-helper.sh` and the policy. |
| `rigscope-<v>-macos-universal.dmg` | macOS | arm64 + x86_64, ad-hoc signed, not notarized (right click -> Open). Shows the built-in demo machine. |
| `SHA256SUMS.txt` | | Checksums of all of the above. |

Windows is not built: it is not a supported platform.

## Why the helper is a root-owned copy

`pkexec` runs the helper as root. The app refuses to elevate unless the program it would run is
owned by root and not writable by anyone else (otherwise any process of the user could replace
it after the password was typed). Development builds can opt out with
`RIGSCOPE_ALLOW_USER_HELPER=1`.

## Local checks

- `packaging/linux/package.sh`, `deb.sh`, `appimage.sh` on Linux; `packaging/macos/package.sh`
  on macOS. Files land in `dist/release/` (not in git).
- Bump an action only together with its commit SHA and version comment (see `ci.yml`).
