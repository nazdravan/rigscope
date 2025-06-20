//! UI strings in English and Romanian. Every visible text goes through here.

use rigscope_engine::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Memory,
    Processor,
    System,
    Unknown,
    NeedsRoot,
    Empty,
    Estimated,
    Compatible,
    UpTo,
    MaxPerSlot,
    MixedTypes,
    Copy,
    InstalledMemory,
    MaxCapacity,
    Ecc,
    Slot,
    Slots,
    SlotDetails,
    Bank,
    Size,
    Type,
    FormFactor,
    RatedSpeed,
    ConfiguredSpeed,
    Manufacturer,
    PartNumber,
    Serial,
    Ranks,
    Voltage,
    Width,
    NeedsRootBanner,
    ReadElevated,
    NoDmi,
    Model,
    Vendor,
    Socket,
    CoresThreads,
    BaseClock,
    MaxClock,
    Microcode,
    Cache,
    CurrentFrequency,
    FromCpuinfo,
    Flags,
    Os,
    Kernel,
    Hostname,
    Uptime,
    Board,
    Bios,
    Version,
    Date,
    Live,
    File,
    View,
    Edit,
    LanguageMenu,
    Refresh,
    Undo,
    DemoMode,
    Storage,
    DiskSize,
    DiskKind,
    Interface,
    Firmware,
    Removable,
    Yes,
    No,
    NoDisks,
    Graphics,
    PciId,
    PciAddress,
    Driver,
    Vram,
    PcieLink,
    BoardVariant,
    PrimaryDisplay,
    NoGpus,
    Network,
    NetKind,
    Mac,
    State,
    Speed,
    Duplex,
    Mtu,
    NoInterfaces,
    Battery,
    OnMains,
    Technology,
    Status,
    Charge,
    Health,
    FullCapacity,
    DesignCapacity,
    CurrentEnergy,
    Cycles,
    NoBattery,
    Derived,
    Sensors,
    DiskHealth,
    SmartNeedsRoot,
    NeedsAdmin,
    SmartNeedsAdmin,
    EstimatedFrequency,
    NotElevated,
    Warning,
    NoSensors,
}

pub fn tr(lang: Language, key: Key) -> &'static str {
    use Key::*;
    match lang {
        Language::En => match key {
            Memory => "Memory",
            Processor => "Processor",
            System => "System",
            Unknown => "Unknown",
            NeedsRoot => "Needs root",
            NeedsAdmin => "Needs administrator",
            SmartNeedsAdmin => "Disk health (SMART) needs administrator rights: rigscope is running without them.",
            EstimatedFrequency => "estimated: % processor performance × base clock",
            NotElevated => "running without administrator rights",
            Empty => "Empty",
            Estimated => "estimated",
            Compatible => "Compatible",
            UpTo => "up to",
            MaxPerSlot => "per slot",
            MixedTypes => "mixed memory types installed",
            Copy => "Copy",
            InstalledMemory => "Installed memory (seen by the OS)",
            MaxCapacity => "Maximum supported by the board",
            Ecc => "Error correction",
            Slot => "Slot",
            Slots => "Slots",
            SlotDetails => "Slot details",
            Bank => "Bank",
            Size => "Size",
            Type => "Type",
            FormFactor => "Form factor",
            RatedSpeed => "Rated speed",
            ConfiguredSpeed => "Running at",
            Manufacturer => "Manufacturer",
            PartNumber => "Part number",
            Serial => "Serial number",
            Ranks => "Ranks",
            Voltage => "Voltage",
            Width => "Data width",
            NeedsRootBanner => "Slot details need root: the BIOS memory table is readable only by root.",
            ReadElevated => "Read with elevated rights",
            NoDmi => "No DMI table: this looks like a virtual machine or container, so slot details are not available.",
            Model => "Model",
            Vendor => "Vendor",
            Socket => "Socket",
            CoresThreads => "Cores / threads",
            BaseClock => "Base clock",
            MaxClock => "Max clock",
            Microcode => "Microcode",
            Cache => "Cache",
            CurrentFrequency => "Current frequency per core",
            FromCpuinfo => "from /proc/cpuinfo (no cpufreq)",
            Flags => "Instruction set extensions",
            Os => "Operating system",
            Kernel => "Kernel",
            Hostname => "Hostname",
            Uptime => "Uptime",
            Board => "Motherboard",
            Bios => "BIOS",
            Version => "Version",
            Date => "Date",
            Live => "Live",
            File => "File",
            View => "View",
            Edit => "Edit",
            LanguageMenu => "Language",
            Refresh => "Refresh",
            Undo => "Undo",
            DemoMode => "Demo mode: showing a sample machine, not this computer.",
            Storage => "Storage",
            DiskSize => "Capacity",
            DiskKind => "Type",
            Interface => "Interface",
            Firmware => "Firmware",
            Removable => "Removable",
            Yes => "yes",
            No => "no",
            NoDisks => "No disks found in /sys/block.",
            Graphics => "Graphics",
            PciId => "PCI ID",
            PciAddress => "PCI address",
            Driver => "Driver",
            Vram => "Video memory",
            PcieLink => "PCIe link",
            BoardVariant => "Board variant",
            PrimaryDisplay => "Primary display (boot)",
            NoGpus => "No graphics cards found in /sys/class/drm.",
            Network => "Network",
            NetKind => "Type",
            Mac => "MAC address",
            State => "State",
            Speed => "Link speed",
            Duplex => "Duplex",
            Mtu => "MTU",
            NoInterfaces => "No physical network interfaces found.",
            Battery => "Battery",
            OnMains => "On mains power",
            Technology => "Chemistry",
            Status => "Status",
            Charge => "Charge",
            Health => "Health",
            FullCapacity => "Full charge capacity",
            DesignCapacity => "Design capacity",
            CurrentEnergy => "Energy now",
            Cycles => "Charge cycles",
            NoBattery => "No battery: this machine runs from mains power only.",
            Derived => "derived",
            Sensors => "Sensors",
            DiskHealth => "Health (SMART)",
            SmartNeedsRoot => "Disk health (SMART) needs root: smartctl reads it through the elevated helper.",
            Warning => "Warning",
            NoSensors => "No sensors found in /sys/class/hwmon.",
        },
        Language::Ro => match key {
            Memory => "Memorie",
            Processor => "Procesor",
            System => "Sistem",
            Unknown => "Necunoscut",
            NeedsRoot => "Necesită root",
            NeedsAdmin => "Necesită administrator",
            SmartNeedsAdmin => "Sănătatea discurilor (SMART) necesită drepturi de administrator: rigscope rulează fără ele.",
            EstimatedFrequency => "estimat: % performanță procesor × frecvența de bază",
            NotElevated => "rulează fără drepturi de administrator",
            Empty => "Gol",
            Estimated => "estimat",
            Compatible => "Compatibil",
            UpTo => "până la",
            MaxPerSlot => "pe slot",
            MixedTypes => "sunt instalate tipuri diferite de memorie",
            Copy => "Copiază",
            InstalledMemory => "Memorie instalată (văzută de sistem)",
            MaxCapacity => "Maximul suportat de placa de bază",
            Ecc => "Corecția erorilor",
            Slot => "Slot",
            Slots => "Sloturi",
            SlotDetails => "Detalii sloturi",
            Bank => "Bancă",
            Size => "Capacitate",
            Type => "Tip",
            FormFactor => "Format",
            RatedSpeed => "Frecvență nominală",
            ConfiguredSpeed => "Rulează la",
            Manufacturer => "Producător",
            PartNumber => "Cod produs",
            Serial => "Număr de serie",
            Ranks => "Ranguri",
            Voltage => "Tensiune",
            Width => "Lățime date",
            NeedsRootBanner => "Detaliile sloturilor necesită root: tabela de memorie din BIOS poate fi citită doar de root.",
            ReadElevated => "Citește cu drepturi ridicate",
            NoDmi => "Nu există tabelă DMI: pare o mașină virtuală sau un container, deci detaliile sloturilor lipsesc.",
            Model => "Model",
            Vendor => "Producător",
            Socket => "Socket",
            CoresThreads => "Nuclee / fire",
            BaseClock => "Frecvență de bază",
            MaxClock => "Frecvență maximă",
            Microcode => "Microcod",
            Cache => "Cache",
            CurrentFrequency => "Frecvența curentă pe nucleu",
            FromCpuinfo => "din /proc/cpuinfo (fără cpufreq)",
            Flags => "Extensii ale setului de instrucțiuni",
            Os => "Sistem de operare",
            Kernel => "Kernel",
            Hostname => "Nume calculator",
            Uptime => "Timp de funcționare",
            Board => "Placă de bază",
            Bios => "BIOS",
            Version => "Versiune",
            Date => "Dată",
            Live => "Live",
            File => "Fișier",
            View => "Vizualizare",
            Edit => "Editare",
            LanguageMenu => "Limbă",
            Refresh => "Reîmprospătează",
            Undo => "Anulează",
            DemoMode => "Mod demonstrativ: se afișează o mașină de exemplu, nu acest calculator.",
            Storage => "Stocare",
            DiskSize => "Capacitate",
            DiskKind => "Tip",
            Interface => "Interfață",
            Firmware => "Firmware",
            Removable => "Detașabil",
            Yes => "da",
            No => "nu",
            NoDisks => "Nu s-au găsit discuri în /sys/block.",
            Graphics => "Placă video",
            PciId => "ID PCI",
            PciAddress => "Adresă PCI",
            Driver => "Driver",
            Vram => "Memorie video",
            PcieLink => "Legătură PCIe",
            BoardVariant => "Varianta plăcii",
            PrimaryDisplay => "Afișaj principal (la pornire)",
            NoGpus => "Nu s-au găsit plăci video în /sys/class/drm.",
            Network => "Rețea",
            NetKind => "Tip",
            Mac => "Adresă MAC",
            State => "Stare",
            Speed => "Viteza legăturii",
            Duplex => "Duplex",
            Mtu => "MTU",
            NoInterfaces => "Nu s-au găsit interfețe de rețea fizice.",
            Battery => "Baterie",
            OnMains => "Alimentat de la rețea",
            Technology => "Chimie",
            Status => "Stare",
            Charge => "Încărcare",
            Health => "Sănătate",
            FullCapacity => "Capacitate la încărcare completă",
            DesignCapacity => "Capacitate de design",
            CurrentEnergy => "Energie acum",
            Cycles => "Cicluri de încărcare",
            NoBattery => "Nu există baterie: calculatorul funcționează doar de la rețea.",
            Derived => "derivat",
            Sensors => "Senzori",
            DiskHealth => "Sănătate (SMART)",
            SmartNeedsRoot => "Sănătatea discurilor (SMART) necesită root: smartctl o citește prin helperul cu drepturi ridicate.",
            Warning => "Atenție",
            NoSensors => "Nu s-au găsit senzori în /sys/class/hwmon.",
        },
    }
}

/// Banner while a saved snapshot is shown instead of this machine.
pub fn opened_snapshot(lang: Language, path: &str) -> String {
    match lang {
        Language::En => {
            format!("Showing the saved snapshot {path}, not this computer. Refresh reads this computer again.")
        }
        Language::Ro => format!(
            "Se afișează snapshot-ul salvat {path}, nu acest calculator. Reîmprospătează citește din nou acest calculator."
        ),
    }
}

/// "1 of 2 slots free" / "1 din 2 sloturi libere".
pub fn slots_free(lang: Language, free: u16, total: u16) -> String {
    match lang {
        Language::En => format!("{free} of {total} slots free"),
        Language::Ro => format!("{free} din {total} sloturi libere"),
    }
}

/// Reasons the probe records in `Field::Unavailable` are English; translate the known ones.
pub fn reason(lang: Language, reason: &str) -> String {
    if lang == Language::Ro {
        // Windows reasons carry a variable part: "<api> failed (0x00000005)".
        if let Some((api, code)) = reason
            .strip_suffix(')')
            .and_then(|r| r.rsplit_once(" failed ("))
        {
            return format!("{api} a eșuat ({code})");
        }
        if let Some(rest) = reason.strip_prefix("SMBIOS table unreadable: ") {
            return format!("tabela SMBIOS nu a putut fi citită: {rest}");
        }
    }
    let ro = match reason {
        "the firmware has no SMBIOS table (virtual machine?)" => {
            "firmware-ul nu are tabelă SMBIOS (mașină virtuală?)"
        }
        "not exposed by the kernel without root" => "nu e expus de kernel fără root",
        "no DMI table (virtual machine or container?)" => {
            "nu există tabelă DMI (mașină virtuală sau container?)"
        }
        "pci.ids not installed (hwdata package)" => "pci.ids nu e instalat (pachetul hwdata)",
        "not exposed by the nvidia driver" => "nu e expusă de driverul nvidia",
        "uses system memory" => "folosește memoria sistemului",
        "built in, no PCIe link" => "integrat, fără legătură PCIe",
        "not reported for Wi-Fi" => "nu e raportată pentru Wi-Fi",
        "does not apply to Wi-Fi" => "nu se aplică la Wi-Fi",
        "this kind of device has no SMART data" => "acest tip de dispozitiv nu are date SMART",
        "smartctl not installed (smartmontools package)" => {
            "smartctl nu e instalat (pachetul smartmontools)"
        }
        "smartctl timed out" => "smartctl nu a răspuns la timp",
        "smartctl output could not be read" => "rezultatul smartctl nu a putut fi citit",
        "WMI did not answer" => "WMI nu a răspuns",
        "the USB bridge does not pass SMART through" => "puntea USB nu transmite SMART",
        "behind a RAID controller" => "în spatele unui controler RAID",
        "virtual disk (no SMART)" => "disc virtual (fără SMART)",
        "unreadable NVMe health log" => "jurnalul de sănătate NVMe nu a putut fi citit",
        "unreadable SMART data" => "datele SMART nu au putut fi citite",
        "not available on Windows: needs a kernel driver" => {
            "indisponibil pe Windows: necesită un driver de kernel"
        }
        "the battery reports relative capacity, not mWh" => {
            "bateria raportează capacitate relativă, nu mWh"
        }
        "the graphics driver reports no temperature" => "driverul grafic nu raportează temperatura",
        other => other,
    };
    match lang {
        Language::En => reason.to_string(),
        Language::Ro => ro.to_string(),
    }
}

/// Label for a command id (menus, tooltips).
pub fn command_label(lang: Language, label_key: &str) -> String {
    let (en, ro) = match label_key {
        "cmd.refresh" => ("Refresh", "Reîmprospătează"),
        "cmd.undo" => ("Undo", "Anulează"),
        "cmd.live" => ("Live updates", "Actualizare live"),
        "cmd.elevate" => ("Read with elevated rights", "Citește cu drepturi ridicate"),
        "cmd.copy_compat" => (
            "Copy compatibility line",
            "Copiază linia de compatibilitate",
        ),
        "cmd.copy_report" => ("Copy report", "Copiază raportul"),
        "cmd.export_text" => ("Export report…", "Exportă raportul…"),
        "cmd.save" => ("Save snapshot…", "Salvează snapshot…"),
        "cmd.open" => ("Open snapshot…", "Deschide snapshot…"),
        other => (other, other),
    };
    match lang {
        Language::En => en.to_string(),
        Language::Ro => ro.to_string(),
    }
}

/// ECC values come from the SMBIOS decoder in English.
pub fn ecc(lang: Language, value: &str) -> String {
    let ro = match value {
        "None" => "Fără",
        "Parity" => "Paritate",
        "Single-bit ECC" => "ECC pe un bit",
        "Multi-bit ECC" => "ECC pe mai mulți biți",
        other => other,
    };
    match lang {
        Language::En => value.to_string(),
        Language::Ro => ro.to_string(),
    }
}

/// "1 virtual interface not listed" / "2 interfețe virtuale nelistate".
pub fn hidden_virtual(lang: Language, n: u32) -> String {
    match (lang, n) {
        (Language::En, 1) => "1 virtual interface not listed".into(),
        (Language::En, n) => format!("{n} virtual interfaces not listed"),
        (Language::Ro, 1) => "1 interfață virtuală nelistată".into(),
        (Language::Ro, n) => format!("{n} interfețe virtuale nelistate"),
    }
}

/// Enumerated values the kernel writes in English (battery status, link state, duplex).
pub fn kernel_value(lang: Language, value: &str) -> String {
    let ro = match value {
        "Charging" => "Se încarcă",
        "Discharging" => "Se descarcă",
        "Full" => "Încărcată complet",
        "Not charging" => "Nu se încarcă",
        "up" => "activă",
        "down" => "inactivă",
        "dormant" => "în așteptare",
        "lowerlayerdown" => "fără legătură fizică",
        "full" => "complet",
        "half" => "semi",
        other => other,
    };
    match lang {
        Language::En => value.to_string(),
        Language::Ro => ro.to_string(),
    }
}

/// "10 flags" / "10 extensii".
pub fn flag_count(lang: Language, n: usize) -> String {
    match lang {
        Language::En => format!("{n} flags"),
        Language::Ro => format!("{n} extensii"),
    }
}

/// Status-bar messages come from the engine in English; translate the known shapes.
pub fn status(lang: Language, msg: &str) -> String {
    if lang == Language::En {
        return msg.to_string();
    }
    const PREFIXES: &[(&str, &str)] = &[
        ("command not available: ", "comandă indisponibilă: "),
        ("bad parameters: ", "parametri greșiți: "),
        ("unknown command: ", "comandă necunoscută: "),
        ("copied: ", "copiat: "),
        ("saved ", "salvat "),
        ("opened ", "deschis "),
        ("clipboard: ", "clipboard: "),
        ("showing ", "se afișează "),
    ];
    const WHOLE: &[(&str, &str)] = &[
        ("report copied", "raport copiat"),
        ("waiting for authentication…", "se așteaptă autentificarea…"),
        ("cancelled", "anulat"),
        (
            "not authorized or pkexec not available",
            "neautorizat sau pkexec lipsește",
        ),
        (
            "timed out waiting for authentication",
            "timpul de autentificare a expirat",
        ),
        (
            "internal error; the previous state was restored",
            "eroare internă; starea anterioară a fost restaurată",
        ),
        ("nothing to undo", "nimic de anulat"),
        (
            "the full DMI table is already loaded",
            "tabela DMI completă e deja încărcată",
        ),
        (
            "already waiting for authentication",
            "se așteaptă deja autentificarea",
        ),
        (
            "elevated reading needs Linux with pkexec",
            "citirea cu drepturi ridicate cere Linux cu pkexec",
        ),
        (
            "on Windows rigscope runs as administrator from the start; there is no separate elevated read",
            "pe Windows rigscope rulează ca administrator de la pornire; nu există o citire separată cu drepturi ridicate",
        ),
        ("no memory slot data", "nu există date despre sloturi"),
        (
            "this machine has no DMI table",
            "acest calculator nu are tabelă DMI",
        ),
    ];
    let whole = |m: &str| {
        WHOLE
            .iter()
            .find(|(en, _)| *en == m)
            .map(|(_, ro)| (*ro).to_string())
    };
    if let Some(t) = whole(msg) {
        return t;
    }
    // Set by the engine on Windows when a disk needs elevation (`elevation_status`).
    if msg == tr(Language::En, Key::NotElevated) {
        return tr(lang, Key::NotElevated).to_string();
    }
    let msg = &msg.replace(
        "; refresh to return to this computer",
        "; reîmprospătează ca să revii la acest calculator",
    );
    for (en, ro) in PREFIXES {
        if let Some(rest) = msg.strip_prefix(en) {
            return format!("{ro}{}", status(lang, rest));
        }
    }
    msg.to_string()
}

/// Tooltip for a value that is missing: why, and what can be done.
pub fn missing_hint(lang: Language, value: &str) -> Option<&'static str> {
    if value == tr(lang, Key::Unknown) {
        Some(match lang {
            Language::En => "The firmware or the kernel did not report this value.",
            Language::Ro => "Firmware-ul sau kernelul nu a raportat această valoare.",
        })
    } else if value == tr(lang, Key::NeedsRoot) {
        Some(match lang {
            Language::En => "Readable only by root: use Read with elevated rights in the Memory section.",
            Language::Ro => "Poate fi citit doar de root: folosește Citește cu drepturi ridicate în secțiunea Memorie.",
        })
    } else if value == tr(lang, Key::NeedsAdmin) {
        Some(match lang {
            Language::En => "Readable only with administrator rights: run rigscope as administrator.",
            Language::Ro => "Poate fi citit doar cu drepturi de administrator: rulează rigscope ca administrator.",
        })
    } else {
        None
    }
}

/// [`missing_hint`] for whichever UI language the value is in.
pub fn missing_hint_any(value: &str) -> Option<&'static str> {
    missing_hint(Language::En, value).or_else(|| missing_hint(Language::Ro, value))
}

/// SMART summary for the UI: the model's English summary, or a Romanian one.
pub fn health_line(lang: Language, h: &rigscope_model::DiskHealth) -> String {
    if lang == Language::En {
        return rigscope_model::health_summary(h).join(", ");
    }
    let mut out = Vec::new();
    match h.passed {
        Some(true) => out.push("SMART: OK".to_string()),
        Some(false) => out.push("SMART: DEFECT".to_string()),
        None => {}
    }
    if let Some(t) = h.temperature_c {
        out.push(format!("{t} °C"));
    }
    if let Some(v) = h.power_on_hours {
        out.push(format!("{v} h de funcționare"));
    }
    if let Some(v) = h.power_cycles {
        out.push(format!("{v} porniri"));
    }
    if let Some(v) = h.percent_used {
        out.push(format!("{v} % din durabilitate consumată"));
    }
    if let Some(v) = h.available_spare {
        out.push(format!("{v} % rezervă"));
    }
    if let Some(v) = h.written_bytes {
        out.push(format!("{} scriși", rigscope_model::format_written(v)));
    }
    for (v, what) in [
        (h.reallocated_sectors, "sectoare realocate"),
        (h.pending_sectors, "sectoare în așteptare"),
        (h.uncorrectable_sectors, "sectoare necorectabile"),
        (h.media_errors, "erori de mediu"),
    ] {
        if let Some(n) = v.filter(|n| *n > 0) {
            out.push(format!("{n} {what}"));
        }
    }
    out.join(", ")
}

/// A [`rigscope_model::DiskHealth::warnings`] item in the UI language.
pub fn warning(lang: Language, w: &str) -> String {
    let ro = match w {
        "drive reports SMART failure" => "discul raportează defect SMART",
        "rated endurance nearly used up" => "durabilitatea nominală e aproape consumată",
        "reallocated sectors" => "sectoare realocate",
        "sectors pending reallocation" => "sectoare care așteaptă realocarea",
        "uncorrectable sectors" => "sectoare necorectabile",
        "media errors" => "erori de mediu",
        other => other,
    };
    match lang {
        Language::En => w.to_string(),
        Language::Ro => ro.to_string(),
    }
}
