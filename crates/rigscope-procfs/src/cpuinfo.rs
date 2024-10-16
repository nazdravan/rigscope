/// What rigscope takes from `/proc/cpuinfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CpuinfoSummary {
    pub model: Option<String>,
    pub vendor: Option<String>,
    /// Number of `processor` entries.
    pub logical: u32,
    /// Unique `(physical id, core id)` pairs; `None` when the kernel does not report them (ARM).
    pub physical_cores: Option<u32>,
    pub microcode: Option<String>,
    pub flags: Vec<String>,
    /// `(processor number, cpu MHz)` for entries that report a frequency.
    pub mhz_per_cpu: Vec<(u32, u32)>,
}

pub fn parse_cpuinfo(text: &str) -> CpuinfoSummary {
    let mut out = CpuinfoSummary::default();
    let mut cores: Vec<(String, String)> = Vec::new();
    let mut saw_topology = false;

    for block in text.split("\n\n") {
        let mut processor: Option<u32> = None;
        let mut physical_id: Option<&str> = None;
        let mut core_id: Option<&str> = None;
        for line in block.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "processor" => processor = value.parse().ok(),
                "model name" if out.model.is_none() => {
                    out.model = rigscope_model::clean_text(value)
                }
                "vendor_id" if out.vendor.is_none() => {
                    out.vendor = rigscope_model::clean_text(value)
                }
                "microcode" if out.microcode.is_none() => {
                    out.microcode = rigscope_model::clean_text(value)
                }
                "flags" | "Features" if out.flags.is_empty() => {
                    out.flags = value
                        .split_whitespace()
                        .take(512)
                        .map(str::to_string)
                        .collect();
                }
                "physical id" => physical_id = Some(value),
                "core id" => core_id = Some(value),
                "cpu MHz" => {
                    let mhz = value.split('.').next().and_then(|v| v.parse::<u32>().ok());
                    if let (Some(cpu), Some(mhz)) = (processor, mhz) {
                        if out.mhz_per_cpu.len() < crate::MAX_CPUS {
                            out.mhz_per_cpu.push((cpu, mhz));
                        }
                    }
                }
                _ => {}
            }
        }
        if processor.is_some() {
            out.logical = out.logical.saturating_add(1);
        }
        if let (Some(p), Some(c)) = (physical_id, core_id) {
            saw_topology = true;
            let pair = (p.to_string(), c.to_string());
            if !cores.contains(&pair) && cores.len() < crate::MAX_CPUS {
                cores.push(pair);
            }
        }
    }
    if saw_topology {
        out.physical_cores = u32::try_from(cores.len()).ok();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTEL: &str = "processor\t: 0
vendor_id\t: GenuineIntel
cpu family\t: 6
model name\t: Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz
microcode\t: 0xf4
cpu MHz\t\t: 1800.003
physical id\t: 0
core id\t\t: 0
flags\t\t: fpu sse2 avx avx2

processor\t: 1
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz
cpu MHz\t\t: 2100.5
physical id\t: 0
core id\t\t: 1
flags\t\t: fpu sse2 avx avx2

processor\t: 2
model name\t: Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz
cpu MHz\t\t: 900.0
physical id\t: 0
core id\t\t: 0

processor\t: 3
model name\t: Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz
physical id\t: 0
core id\t\t: 1
";

    const ARM: &str = "processor\t: 0
BogoMIPS\t: 108.00
Features\t: fp asimd evtstrm aes
CPU implementer\t: 0x41
CPU part\t: 0xd0b

processor\t: 1
BogoMIPS\t: 108.00
Features\t: fp asimd evtstrm aes
";

    #[test]
    fn intel_four_threads_two_cores() {
        let s = parse_cpuinfo(INTEL);
        assert_eq!(
            s.model.as_deref(),
            Some("Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz")
        );
        assert_eq!(s.vendor.as_deref(), Some("GenuineIntel"));
        assert_eq!(s.logical, 4);
        assert_eq!(s.physical_cores, Some(2));
        assert_eq!(s.microcode.as_deref(), Some("0xf4"));
        assert!(s.flags.contains(&"avx2".to_string()));
        assert_eq!(s.mhz_per_cpu, vec![(0, 1800), (1, 2100), (2, 900)]);
    }

    #[test]
    fn arm_without_physical_id() {
        let s = parse_cpuinfo(ARM);
        assert_eq!(s.logical, 2);
        assert_eq!(s.physical_cores, None);
        assert_eq!(s.model, None);
        assert!(s.flags.contains(&"asimd".to_string()));
    }

    #[test]
    fn empty_input() {
        assert_eq!(parse_cpuinfo(""), CpuinfoSummary::default());
    }
}
