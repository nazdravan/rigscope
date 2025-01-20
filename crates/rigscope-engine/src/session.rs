use crate::Env;
use rigscope_model::Snapshot;
use rigscope_probe::DmiDump;
use serde::{Deserialize, Serialize};

/// Most snapshots kept for undo.
pub const HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionId {
    Memory,
    Cpu,
    System,
}

/// UI language (the UI owns the strings; the engine only remembers the choice).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Ro,
}

pub struct Session {
    pub snapshot: Snapshot,
    pub history: Vec<Snapshot>,
    pub section: SectionId,
    pub live: bool,
    pub language: Language,
    /// Last user-facing message (errors from commands, "copied", ...).
    pub status: Option<String>,
    /// DMI dump from the elevated helper, reused by refresh so slot details stay.
    pub dmi_dump: Option<DmiDump>,
    /// The privileged helper is running (waiting for authentication).
    pub elevating: bool,
    pub env: Box<dyn Env>,
}

impl Session {
    /// Replaces the snapshot, keeping the old one for undo.
    pub fn replace_snapshot(&mut self, next: Snapshot) {
        let prev = std::mem::replace(&mut self.snapshot, next);
        self.history.push(prev);
        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
    }
}
