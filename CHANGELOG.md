# Changelog

All notable changes to rigscope. The notes of each version are the text of its GitHub release.

## Unreleased

### Changed
- The code uses Rust edition 2024 (was 2021). No behaviour changes.

## [0.2.1] — 2025-07-21

### Added
- Windows 10 (1809 or later, x86_64): real hardware read — memory slots from the firmware SMBIOS table,
  processor topology and caches, disks with SMART (NVMe and SATA), graphics with exact VRAM and
  temperature, network, battery. A portable `rigscope-<version>-windows-x86_64.exe` in the release.
- macOS reads the machine instead of showing a sample, through `system_profiler` and `sysctl`:
  memory modules (size, type, speed, maker, part number), the processor, the board model, the
  operating system, SATA and NVMe disks with the drive's SMART verdict, graphics cards, and
  Ethernet and Wi-Fi adapters. Battery and Sensors are not read on macOS yet. Apple silicon has
  no memory slots, so only the total is shown there.
- `rigscope --capture-fixture DIR` (Windows elevated, or macOS) saves a masked fixture; `--fixture
  DIR` opens any kind of fixture on any OS. `cargo xtask windows-icon` builds the Windows icon.

### Changed
- "Needs root" is "Needs administrator" on Windows snapshots. Snapshots saved by 0.2.0 still open.
- Fixtures no longer contain `:` or symlinks (`%3A` encoding plus `links.txt`); `.gitattributes`
  forces LF.

### Fixed
- Windows: a disk that does not implement the SMART IOCTL (a virtual or RAID disk answers
  `ERROR_INVALID_FUNCTION`) now reads as not available instead of an error.
- The macOS disk image and the Windows `.exe` are in the release (0.2.0 had neither).

### Known limits
- macOS: Battery and Sensors are not read, and USB disks are not listed. The memory speed is the
  running speed only (macOS gives no rated speed, voltage or ranks); disks show the SMART verdict
  without attributes; network adapters show no link state or speed; the JEDEC maker table is
  short, so an unlisted code is shown as printed. Tested on one Intel iMac.
- On Windows the CPU temperature and the fans are not shown: both need a kernel driver.
- The Windows `.exe` is not code-signed (SmartScreen warns) and asks for administrator rights at
  each start.

## [0.2.0] — 2025-05-05

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

## [0.1.0] — 2025-04-29

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
