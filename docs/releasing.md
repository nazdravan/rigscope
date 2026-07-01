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
| `rigscope-<v>-linux-x86_64.deb` | Ubuntu 22.04 (glibc 2.35) | Installs `/usr/bin/rigscope`, the desktop file and the icon. |
| `rigscope-<v>-linux-x86_64.AppImage` | Ubuntu 22.04 | One file, no install; the root-only data (SMBIOS, SMART) works from it too. |
| `rigscope-<v>-linux-x86_64.tar.gz` | Ubuntu 22.04 | Binary and licences. |
| `rigscope-<v>-macos-universal.dmg` | macOS | arm64 + x86_64, ad-hoc signed, not notarized (right click -> Open). Reads memory, processor, board, system, disks, graphics and network through `system_profiler`; battery and sensors are not read yet. |
| `rigscope-<v>-windows-x86_64-portable.zip` | `Windows x86_64` job | The folder `rigscope-<v>-windows-x86_64-portable/` with `rigscope.exe`, `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD-PARTY.txt` (the font licence) and `README.md`. Built with `Compress-Archive` in a pwsh step, staged outside `dist/` so only the release files are uploaded. |
| `rigscope-<v>-windows-x86_64.exe` | `Windows x86_64` job, `windows-2025` runner | Portable, nothing to install. x86_64 MSVC build with a static CRT (`RUSTFLAGS=-C target-feature=+crt-static`); `build.rs` embeds the `requireAdministrator` manifest, icon and version (needs `rc.exe` from the runner's Windows SDK). Not code-signed, so SmartScreen warns; users verify it with `SHA256SUMS.txt`. |
| `SHA256SUMS.txt` | | Checksums of all of the above. |

The `Windows x86_64` job builds with `cargo build --release --locked -p rigscope --target
x86_64-pc-windows-msvc`, copies `rigscope.exe` to `dist/rigscope-<v>-windows-x86_64.exe`, makes the
portable zip next to it and uploads both as the `release-windows-x86_64` artifact; the release job waits for it like for the
others. There is no signing step.

## The in-app updater

Help → Check for updates (the `app-update` crate) finds its file by the exact names above and
checks it against `SHA256SUMS.txt` (`<sum>  <name>` lines) of the same release, so neither the
names nor the `v<version>` tag may change. It updates the `.deb`, the AppImage, the `.app` from the
`.dmg` and the portable Windows folder (recognised by the `LICENSE-*` file next to `rigscope.exe`);
the bare `.exe` and the `.tar.gz` only get a link to the release.

## Root without a helper

"Read with elevated rights" runs `pkexec /bin/sh -c <script>`: the system shell (root-owned)
with a fixed script compiled into rigscope (`apps/rigscope/src/dump_dmi.rs`). No file of rigscope
runs as root, so no package has to install anything, and nothing a user process could swap after
the password is typed is ever executed. The script checks its arguments again (smartctl path
from a fixed list, kernel disk names) and only prints hex; the window parses it.

## Licences in the packages

Every package carries `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `THIRD-PARTY.txt` and
`OFL-NotoSans.txt` (from `assets/fonts/OFL.txt`). The packaging scripts copy them only through
`copy_docs` (`packaging/env.sh`); the Windows zip step in `release.yml` copies the same files.
The standalone Windows `.exe` carries none: the zip is the one with the licences.

- `THIRD-PARTY.txt` is generated, not in git: `packaging/licences.sh` (needs
  `cargo install cargo-about --locked --features cli --version <CARGO_ABOUT_VERSION from env.sh>`)
  writes it to `dist/` with the licence of every crate and of egui's fonts. In CI the `licences`
  job makes it on every run, and fails if a crate has a licence that is not in `about.toml`.
- After an egui update the checksums in `[epaint_default_fonts.clarify]` (`about.toml`) stop
  matching and `packaging/licences.sh` stops: check the crate's `fonts/*.txt` and update the sums
  (`shasum -a 256`).
- A new file in `assets/fonts/` must be named in `NOTICE`, with its licence: otherwise
  `cargo xtask licences` (part of `cargo xtask ci`) fails.

## Local checks

- `packaging/linux/package.sh`, `deb.sh`, `appimage.sh` on Linux; `packaging/macos/package.sh`
  on macOS; on Windows `cargo build --release -p rigscope` (the icon comes from
  `cargo xtask windows-icon`). Files land in `dist/release/` (not in git).
- Bump an action only together with its commit SHA and version comment (see `ci.yml`).
