use crate::{CpuInfo, DmiDump, SensorInfo, Snapshot};

/// Reads the machine. One implementation per OS (`rigscope-probe` for Linux,
/// `rigscope-probe-win` for Windows); the engine sees only this trait.
pub trait Collector {
    /// A full snapshot. `elevated`: what the privileged read returned (Linux only), if any.
    fn collect(&mut self, elevated: Option<&DmiDump>, now_unix: u64) -> Snapshot;
    /// Re-reads the per-core frequency. Must be cheap (it runs every UI tick).
    fn refresh_live(&mut self, cpu: &mut CpuInfo);
    /// Re-reads the sensor values. Must be cheap (it runs every UI tick).
    fn refresh_sensors(&mut self, sensors: &mut SensorInfo);
}
