use crate::{WinSource, cpu, gpu, net, power, sensors, storage, system};
use rigscope_model::{Collector, CpuInfo, DmiDump, Platform, SensorInfo, Snapshot};

/// The Windows collector. Keeps the GPU list from the last collect for the cheap ticks.
pub struct WinCollector<W: WinSource> {
    src: W,
    gpus: Vec<(String, u64)>,
}

impl<W: WinSource> WinCollector<W> {
    pub fn new(src: W) -> Self {
        WinCollector {
            src,
            gpus: Vec::new(),
        }
    }

    pub fn source(&self) -> &W {
        &self.src
    }
}

impl<W: WinSource> Collector for WinCollector<W> {
    /// `elevated` is ignored: on Windows rigscope runs as administrator, there is no helper.
    fn collect(&mut self, _elevated: Option<&DmiDump>, now_unix: u64) -> Snapshot {
        let fw = system::read_firmware(&self.src);
        self.gpus = gpu::gpu_luids(&self.src);
        Snapshot {
            taken_at_unix: now_unix,
            system: system::read_system(&self.src),
            board: system::read_board(&fw),
            memory: system::read_memory(&self.src, &fw),
            cpu: cpu::read_cpu(&mut self.src, &fw),
            storage: storage::read_storage(&self.src),
            gpu: gpu::read_gpus(&self.src),
            net: net::read_net(&self.src),
            power: power::read_power(&self.src),
            sensors: sensors::read_sensors(&mut self.src, &self.gpus),
            dmi_source: fw.source,
            platform: Platform::Windows,
            ..Snapshot::default()
        }
    }

    fn refresh_live(&mut self, cpu: &mut CpuInfo) {
        cpu::refresh_cpu(&mut self.src, cpu);
    }

    fn refresh_sensors(&mut self, s: &mut SensorInfo) {
        sensors::refresh_sensors(&mut self.src, &self.gpus, s);
    }
}
