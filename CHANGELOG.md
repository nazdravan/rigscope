# Changelog

All notable changes to rigscope. The notes of each version are the text of its GitHub release.

## [0.2.0] — 2026-11-03

### Changed
- "Read with elevated rights" no longer needs a helper: it runs a fixed script through the system
  `/bin/sh` under pkexec, so it works straight from the AppImage and the `.tar.gz`.
  `install-helper.sh`, the polkit policy and the `--dump-dmi` mode are gone.
- Disk makers come from the model name when the kernel says only "ATA"; the GPU board maker
  comes from its subsystem id; built-in GPUs say they have no PCIe link.
- A banner stays while a saved snapshot is shown; long status messages show whole on hover.

### Fixed
- Wi-Fi duplex said "Unknown" (it does not apply); a stray "Fans" column in Sensors.

### Checked
- On real hardware (ASUS laptop, Arch, KDE Wayland): the password prompt, the SMBIOS slots and
  SMART against udev and smartctl, the clipboard on Wayland and X11, the file dialogs.

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
