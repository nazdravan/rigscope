use crate::Source;
use rigscope_model::{Collector, CpuInfo, DmiDump, Platform, SensorInfo, Snapshot};

/// The Linux collector: `/proc`, `/sys` and SMBIOS through a [`Source`].
#[derive(Debug, Clone)]
pub struct LinuxCollector<S: Source> {
    src: S,
}

impl<S: Source> LinuxCollector<S> {
    pub fn new(src: S) -> Self {
        LinuxCollector { src }
    }

    /// The underlying source. The app does not use it: its elevated read keeps its own
    /// `Source` (`env::Elevation::Linux`) and reads the disk list from that.
    pub fn source(&self) -> &S {
        &self.src
    }
}

impl<S: Source> Collector for LinuxCollector<S> {
    fn collect(&mut self, elevated: Option<&DmiDump>, now_unix: u64) -> Snapshot {
        Snapshot {
            platform: Platform::Linux,
            ..crate::build_snapshot(&self.src, elevated, now_unix)
        }
    }

    fn refresh_live(&mut self, cpu: &mut CpuInfo) {
        crate::refresh_live(&self.src, cpu);
    }

    fn refresh_sensors(&mut self, sensors: &mut SensorInfo) {
        crate::refresh_sensors(&self.src, sensors);
    }
}
