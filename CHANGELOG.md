# Changelog

All notable changes to rigscope. The notes of each version are the text of its GitHub release.

## [0.1.0] — 2026-11-03

First release.

### Added
- Sections: Memory (every slot, compatibility line for buying RAM), Processor, Storage (with SMART
  health), Graphics, Network, Battery, Sensors, System with Board and BIOS.
- English and Romanian interface; export to text and JSON with serials and MAC addresses masked.
- Live refresh of frequencies, temperatures and fans.
- Root-only data (SMBIOS table, SMART) through a small helper that polkit authorizes.
- Packages: `.deb` (installs the helper and the polkit policy), AppImage and `.tar.gz` for
  Linux x86_64, and a macOS disk image that shows a built-in demo machine.

### Known limits
- Not yet run on real Linux hardware: the authentication dialog, the clipboard and the file
  dialogs were exercised only in a container.
- The AppImage and the `.tar.gz` need `install-helper.sh` (run once with `sudo`) before
  "Read more (root)" works.
