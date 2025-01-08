//! Local control channel (`--control PORT`): line-delimited JSON on 127.0.0.1, used by agents
//! and `cargo xtask screenshot` to drive the app and capture the window.

use eframe::egui;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::Duration;

/// Longest request line accepted.
pub const MAX_REQUEST: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// `{"method":"run","command":"view.section","params":{"id":"cpu"}}`
    Run { command: String, params: Value },
    /// `{"method":"ui.screenshot","path":"/out/x.png"}`
    Screenshot { path: String },
    /// `{"method":"commands"}`
    Commands,
    /// `{"method":"quit"}`
    Quit,
}

pub fn parse_request(line: &str) -> Result<Request, String> {
    if line.len() > MAX_REQUEST {
        return Err(format!("request too long (max {MAX_REQUEST} bytes)"));
    }
    let v: Value = serde_json::from_str(line.trim()).map_err(|e| format!("invalid JSON: {e}"))?;
    let obj = v.as_object().ok_or("request must be a JSON object")?;
    let string = |key: &str| {
        obj.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("missing string \"{key}\""))
    };
    match string("method")?.as_str() {
        "run" => {
            let params = obj
                .get("params")
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));
            if !params.is_object() {
                return Err("\"params\" must be an object".into());
            }
            Ok(Request::Run {
                command: string("command")?,
                params,
            })
        }
        "ui.screenshot" => Ok(Request::Screenshot {
            path: string("path")?,
        }),
        "commands" => Ok(Request::Commands),
        "quit" => Ok(Request::Quit),
        other => Err(format!("unknown method: {other}")),
    }
}

/// A parsed request plus where to send the one-line JSON reply.
pub struct Incoming {
    pub req: Request,
    pub reply: mpsc::Sender<String>,
}

/// How long a connection waits for the UI thread to answer.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// Starts the listener thread. Requests are forwarded to the UI thread, which wakes up via
/// `ctx.request_repaint()`.
pub fn spawn(port: u16, ctx: egui::Context) -> std::io::Result<mpsc::Receiver<Incoming>> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let (tx, rx) = mpsc::channel::<Incoming>();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let tx = tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || serve(stream, &tx, &ctx));
        }
    });
    Ok(rx)
}

fn serve(stream: TcpStream, tx: &mpsc::Sender<Incoming>, ctx: &egui::Context) {
    let Ok(write_half) = stream.try_clone() else {
        return;
    };
    let mut writer = std::io::BufWriter::new(write_half);
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        match reader
            .by_ref()
            .take(MAX_REQUEST as u64 + 1)
            .read_line(&mut line)
        {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let answer = match parse_request(&line) {
            Err(e) => json!({ "error": e }).to_string(),
            Ok(req) => {
                let (reply_tx, reply_rx) = mpsc::channel();
                if tx
                    .send(Incoming {
                        req,
                        reply: reply_tx,
                    })
                    .is_err()
                {
                    return;
                }
                ctx.request_repaint();
                reply_rx.recv_timeout(REPLY_TIMEOUT).unwrap_or_else(|_| {
                    json!({ "error": "timed out waiting for the UI" }).to_string()
                })
            }
        };
        if writeln!(writer, "{answer}")
            .and_then(|()| writer.flush())
            .is_err()
        {
            return;
        }
    }
}

/// Frames to wait after the last command before reading pixels, so layout has settled.
const SCREENSHOT_SETTLE_FRAMES: u8 = 3;

/// Wraps the UI and serves control requests on the UI thread.
pub struct Controlled {
    pub inner: rigscope_ui::RigscopeUi,
    rx: mpsc::Receiver<Incoming>,
    pending: Vec<(String, mpsc::Sender<String>, u8)>,
}

impl Controlled {
    pub fn new(inner: rigscope_ui::RigscopeUi, rx: mpsc::Receiver<Incoming>) -> Self {
        Controlled {
            inner,
            rx,
            pending: Vec::new(),
        }
    }

    fn handle(&mut self, ctx: &egui::Context, inc: Incoming) {
        let reply = match inc.req {
            Request::Run { command, params } => match self.inner.app.run(&command, params) {
                Ok(v) => json!({ "ok": v }),
                Err(e) => json!({ "error": e.to_string() }),
            },
            Request::Commands => {
                let list: Vec<Value> = self
                    .inner
                    .app
                    .specs()
                    .iter()
                    .map(|s| json!({ "id": s.id, "shortcut": s.shortcut, "params": s.params_doc }))
                    .collect();
                json!({ "ok": list })
            }
            Request::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                json!({ "ok": null })
            }
            Request::Screenshot { path } => {
                self.pending
                    .push((path, inc.reply, SCREENSHOT_SETTLE_FRAMES));
                ctx.request_repaint();
                return;
            }
        };
        let _ = inc.reply.send(reply.to_string());
    }

    fn advance_screenshots(&mut self, ctx: &egui::Context) {
        let mut still = Vec::new();
        for (path, reply, frames) in self.pending.drain(..) {
            if frames > 0 {
                still.push((path, reply, frames - 1));
                continue;
            }
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let result = match (u32::try_from(w), u32::try_from(h)) {
                    (Ok(w), Ok(h)) => image::save_buffer(
                        &path,
                        img.as_raw(),
                        w,
                        h,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|e| e.to_string()),
                    _ => Err("screenshot too large".to_string()),
                };
                let msg = match result {
                    Ok(()) => json!({ "ok": { "path": path, "width": w, "height": h } }),
                    Err(e) => json!({ "error": e }),
                };
                let _ = reply.send(msg.to_string());
            });
        }
        self.pending = still;
        if !self.pending.is_empty() {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for Controlled {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Ok(inc) = self.rx.try_recv() {
            self.handle(ctx, inc);
        }
        self.advance_screenshots(ctx);
        self.inner.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.inner.ui(ui, frame);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.inner.save(storage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_run_with_and_without_params() {
        assert_eq!(
            parse_request(r#"{"method":"run","command":"view.section","params":{"id":"cpu"}}"#),
            Ok(Request::Run {
                command: "view.section".into(),
                params: json!({"id":"cpu"})
            })
        );
        assert_eq!(
            parse_request(r#"{"method":"run","command":"snapshot.refresh"}"#),
            Ok(Request::Run {
                command: "snapshot.refresh".into(),
                params: json!({})
            })
        );
    }

    #[test]
    fn parses_screenshot_commands_quit() {
        assert_eq!(
            parse_request(r#"{"method":"ui.screenshot","path":"/tmp/a.png"}"#),
            Ok(Request::Screenshot {
                path: "/tmp/a.png".into()
            })
        );
        assert_eq!(
            parse_request(r#"{"method":"commands"}"#),
            Ok(Request::Commands)
        );
        assert_eq!(parse_request(" {\"method\":\"quit\"}\n"), Ok(Request::Quit));
    }

    #[test]
    fn rejects_bad_requests() {
        assert!(parse_request("not json").is_err());
        assert!(parse_request(r#"{"method":"run"}"#).is_err());
        assert!(parse_request(r#"{"method":"ui.screenshot"}"#).is_err());
        assert!(parse_request(r#"{"method":"explode"}"#).is_err());
        assert!(parse_request(r#"[1,2]"#).is_err());
        assert!(parse_request(r#"{"method":"run","command":"x","params":[1]}"#).is_err());
    }

    #[test]
    fn rejects_oversize() {
        let big = format!(
            r#"{{"method":"run","command":"{}"}}"#,
            "a".repeat(MAX_REQUEST)
        );
        assert!(parse_request(&big).unwrap_err().contains("too long"));
    }

    #[test]
    fn never_panics_on_arbitrary_text() {
        // Lightweight fuzz without proptest (binary crate): byte soup and truncations.
        let seed = r#"{"method":"run","command":"view.section","params":{"id":"cpu"}}"#;
        for cut in 0..seed.len() {
            let _ = parse_request(&seed[..cut]);
        }
        let alphabet = b"{}[]\":,amethodruncommandparams 0123\\";
        let mut x: u32 = 12345;
        for _ in 0..2000 {
            let s: String = (0..40)
                .map(|_| {
                    x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
                    char::from(alphabet[(x >> 16) as usize % alphabet.len()])
                })
                .collect();
            let _ = parse_request(&s);
        }
    }
}
