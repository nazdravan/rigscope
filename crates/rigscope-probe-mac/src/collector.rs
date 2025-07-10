use crate::{memory, system, MacSource};
use rigscope_model::{Collector, CpuInfo, DmiDump, DmiSource, Platform, SensorInfo, Snapshot};

/// The macOS collector. Memory, board and processor come from `system_profiler` and `sysctl`;
/// the other sections are not read on macOS yet and stay unknown.
pub struct MacCollector<S: MacSource> {
    src: S,
}

impl<S: MacSource> MacCollector<S> {
    pub fn new(src: S) -> Self {
        MacCollector { src }
    }

    pub fn source(&self) -> &S {
        &self.src
    }
}

impl<S: MacSource> Collector for MacCollector<S> {
    /// `elevated` is ignored: nothing on macOS needs a privileged read.
    fn collect(&mut self, _elevated: Option<&DmiDump>, now_unix: u64) -> Snapshot {
        let (memory, has_profile) = memory::read_memory(&self.src);
        Snapshot {
            taken_at_unix: now_unix,
            system: system::read_system(&self.src, now_unix),
            board: system::read_board(&self.src),
            memory,
            cpu: system::read_cpu(&self.src),
            dmi_source: if has_profile {
                DmiSource::Firmware
            } else {
                DmiSource::None
            },
            platform: Platform::MacOs,
            ..Snapshot::default()
        }
    }

    fn refresh_live(&mut self, _cpu: &mut CpuInfo) {}

    fn refresh_sensors(&mut self, _sensors: &mut SensorInfo) {}
}
