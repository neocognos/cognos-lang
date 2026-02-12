/// Structured tracing for Cognos runtime diagnostics.
/// Outputs JSONL events to a trace file or stderr.

use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
pub enum TraceLevel {
    Metrics,  // default: latency, sizes, counts
    Full,     // includes prompt, response, command output
}

pub struct Tracer {
    output: Mutex<Box<dyn Write + Send>>,
    start: Instant,
    turn: Mutex<u32>,
    pub level: TraceLevel,
}

impl Tracer {
    pub fn new_file(path: &str, level: TraceLevel) -> std::io::Result<Self> {
        let file = std::fs::File::create(path)?;
        Ok(Self {
            output: Mutex::new(Box::new(std::io::BufWriter::new(file))),
            start: Instant::now(),
            turn: Mutex::new(0),
            level,
        })
    }

    pub fn new_stderr(level: TraceLevel) -> Self {
        Self {
            output: Mutex::new(Box::new(std::io::stderr())),
            start: Instant::now(),
            turn: Mutex::new(0),
            level,
        }
    }

    pub fn increment_turn(&self) -> u32 {
        let mut turn = self.turn.lock().unwrap_or_else(|e| e.into_inner());
        *turn += 1;
        *turn
    }

    pub fn current_turn(&self) -> u32 {
        *self.turn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn emit(&self, event: TraceEvent) {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        let ts = chrono_now();
        let turn = self.current_turn();

        let is_full = self.level == TraceLevel::Full;

        let json = match event {
            TraceEvent::LlmCall { model, provider, latency_ms, prompt_chars, response_chars, has_tool_calls, error, prompt, response, system } => {
                let mut j = serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "llm_call",
                    "model": model, "provider": provider,
                    "latency_ms": latency_ms,
                    "prompt_chars": prompt_chars,
                    "response_chars": response_chars,
                    "has_tool_calls": has_tool_calls,
                    "error": error,
                });
                if is_full {
                    if let Some(p) = prompt { j["prompt"] = serde_json::Value::String(p); }
                    if let Some(r) = response { j["response"] = serde_json::Value::String(r); }
                    if let Some(s) = system { j["system"] = serde_json::Value::String(s); }
                }
                j
            }
            TraceEvent::ToolExec { name, args_summary, latency_ms, result_chars, success, error } => {
                serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "tool_exec",
                    "tool": name, "args": args_summary,
                    "latency_ms": latency_ms,
                    "result_chars": result_chars,
                    "success": success,
                    "error": error,
                })
            }
            TraceEvent::FlowStart { name } => {
                serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "flow_start", "flow": name,
                })
            }
            TraceEvent::FlowEnd { name, duration_ms } => {
                serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "flow_end", "flow": name,
                    "duration_ms": duration_ms,
                })
            }
            TraceEvent::IoOp { operation, handle_type, path, bytes, content } => {
                let mut j = serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "io",
                    "op": operation, "handle": handle_type,
                    "path": path, "bytes": bytes,
                });
                if is_full {
                    if let Some(c) = content { j["content"] = serde_json::Value::String(c); }
                }
                j
            }
            TraceEvent::ShellExec { command, latency_ms, exit_code, output_chars, output } => {
                let mut j = serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "shell_exec",
                    "command": command,
                    "latency_ms": latency_ms,
                    "exit_code": exit_code,
                    "output_chars": output_chars,
                });
                if is_full {
                    if let Some(o) = output { j["output"] = serde_json::Value::String(o); }
                }
                j
            }
            TraceEvent::Context { history_len, context_chars } => {
                serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "context",
                    "history_len": history_len,
                    "context_chars": context_chars,
                })
            }
            TraceEvent::Error { category, message, flow } => {
                serde_json::json!({
                    "ts": ts, "elapsed_ms": elapsed_ms, "turn": turn,
                    "event": "error",
                    "category": category, "message": message, "flow": flow,
                })
            }
        };

        if let Ok(mut out) = self.output.lock() {
            let _ = writeln!(out, "{}", json);
            let _ = out.flush();
        }
    }
}

pub enum TraceEvent {
    LlmCall {
        model: String,
        provider: String,
        latency_ms: u64,
        prompt_chars: usize,
        response_chars: usize,
        has_tool_calls: bool,
        error: Option<String>,
        // Full level only
        prompt: Option<String>,
        response: Option<String>,
        system: Option<String>,
    },
    ToolExec {
        name: String,
        args_summary: String,
        latency_ms: u64,
        result_chars: usize,
        success: bool,
        error: Option<String>,
    },
    FlowStart {
        name: String,
    },
    FlowEnd {
        name: String,
        duration_ms: u64,
    },
    IoOp {
        operation: String,
        handle_type: String,
        path: Option<String>,
        bytes: usize,
        content: Option<String>,
    },
    ShellExec {
        command: String,
        latency_ms: u64,
        exit_code: i32,
        output_chars: usize,
        output: Option<String>,
    },
    Context {
        history_len: usize,
        context_chars: usize,
    },
    Error {
        category: String,
        message: String,
        flow: Option<String>,
    },
}

fn chrono_now() -> String {
    // Simple ISO timestamp without chrono dependency
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Good enough for tracing — exact formatting not critical
    format!("{}", secs)
}
