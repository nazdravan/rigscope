use crate::{EngineError, Env, SectionId, Session};
use serde_json::Value;

/// A registered user-visible action.
#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub id: &'static str,
    /// Key into the UI translation table.
    pub label_key: &'static str,
    /// Menu path, e.g. `"File"`; empty = not in a menu.
    pub menu: &'static str,
    /// e.g. `"Ctrl+Z"`, `"F5"`.
    pub shortcut: Option<&'static str>,
    pub params_doc: &'static str,
    pub enabled: fn(&Session) -> Result<(), String>,
    pub run: fn(&mut Session, &Value) -> Result<Value, EngineError>,
}

pub struct App {
    pub session: Session,
    registry: Vec<CommandSpec>,
}

impl App {
    /// Creates the session and takes the first snapshot.
    pub fn new(env: Box<dyn Env>) -> Self {
        let snapshot = rigscope_probe::build_snapshot(env.source(), None, env.now_unix());
        App {
            session: Session {
                snapshot,
                history: Vec::new(),
                section: SectionId::Memory,
                live: true,
                language: crate::Language::En,
                status: None,
                dmi_dump: None,
                elevating: false,
                env,
            },
            registry: crate::commands::builtin(),
        }
    }

    pub fn specs(&self) -> &[CommandSpec] {
        &self.registry
    }

    /// Adds or replaces a command.
    pub fn register(&mut self, spec: CommandSpec) {
        self.registry.retain(|s| s.id != spec.id);
        self.registry.push(spec);
    }

    fn spec(&self, id: &str) -> Option<CommandSpec> {
        self.registry.iter().find(|s| s.id == id).copied()
    }

    /// `Ok` when the command exists and can run now, else the reason.
    pub fn is_enabled(&self, id: &str) -> Result<(), String> {
        let spec = self
            .spec(id)
            .ok_or_else(|| format!("unknown command: {id}"))?;
        (spec.enabled)(&self.session)
    }

    /// Runs a command under the panic guard: if it panics, the snapshot, history and view are
    /// restored and [`EngineError::Panicked`] is returned.
    pub fn run(&mut self, id: &str, params: Value) -> Result<Value, EngineError> {
        let spec = self
            .spec(id)
            .ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;
        if let Err(reason) = (spec.enabled)(&self.session) {
            let err = EngineError::Disabled(reason);
            self.session.status = Some(err.to_string());
            return Err(err);
        }

        let saved = (
            self.session.snapshot.clone(),
            self.session.history.clone(),
            self.session.section,
            self.session.live,
            self.session.language,
            self.session.dmi_dump.clone(),
        );
        let session = &mut self.session;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (spec.run)(session, &params)
        }));
        match outcome {
            Ok(result) => {
                if let Err(e) = &result {
                    self.session.status = Some(e.to_string());
                }
                result
            }
            Err(_) => {
                let (snapshot, history, section, live, language, dmi_dump) = saved;
                self.session.snapshot = snapshot;
                self.session.history = history;
                self.session.section = section;
                self.session.live = live;
                self.session.language = language;
                self.session.dmi_dump = dmi_dump;
                let err = EngineError::Panicked;
                self.session.status = Some(format!("{id}: {err}"));
                Err(err)
            }
        }
    }
}
