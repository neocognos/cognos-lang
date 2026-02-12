/// Environment trait — abstracts all I/O the interpreter needs.
/// RealEnv talks to the OS. MockEnv returns canned responses.

use anyhow::Result;

pub trait Env {
    fn read_stdin(&mut self) -> Result<String>;
    fn write_stdout(&mut self, content: &str) -> Result<()>;
    fn read_file(&self, path: &str) -> Result<String>;
    fn write_file(&mut self, path: &str, content: &str) -> Result<()>;
    fn exec_shell(&mut self, command: &str) -> Result<ShellResult>;
    fn call_llm(&mut self, request: LlmRequest) -> Result<LlmResponse>;
    fn http_get(&self, url: &str) -> Result<String>;
    fn http_post(&self, url: &str, body: &str) -> Result<String>;

    fn allow_shell(&self) -> bool;

    /// Returns true for mock/test environments.
    fn is_mock(&self) -> bool { false }

    /// Collect stdout buffer (for testing). Returns None for real env.
    fn captured_stdout(&self) -> Option<Vec<String>> { None }
}

pub struct ShellResult {
    pub stdout: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub system: String,
    pub prompt: String,
    pub tools: Option<Vec<serde_json::Value>>,
    pub format: Option<String>,
    pub history: Vec<(String, String)>, // (role, content)
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub raw_json: Option<serde_json::Value>,
}

// ─── RealEnv ───

pub struct RealEnv {
    pub allow_shell: bool,
}

impl RealEnv {
    pub fn new(allow_shell: bool) -> Self {
        Self { allow_shell }
    }
}

impl Env for RealEnv {
    fn is_mock(&self) -> bool { false }
    fn read_stdin(&mut self) -> Result<String> {
        use std::io::BufRead;
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        if line.is_empty() { anyhow::bail!("end of input"); }
        Ok(line.trim_end().to_string())
    }

    fn write_stdout(&mut self, content: &str) -> Result<()> {
        println!("{}", content);
        Ok(())
    }

    fn read_file(&self, path: &str) -> Result<String> {
        std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read '{}': {}", path, e))
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<()> {
        std::fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("cannot write '{}': {}", path, e))
    }

    fn exec_shell(&mut self, command: &str) -> Result<ShellResult> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()?;
        Ok(ShellResult {
            stdout: String::from_utf8_lossy(&output.stdout).trim_end().to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    fn call_llm(&mut self, _request: LlmRequest) -> Result<LlmResponse> {
        // LLM calls are still handled by the interpreter (complex routing logic)
        // This is a placeholder — the interpreter calls this only for MockEnv
        anyhow::bail!("RealEnv.call_llm should not be called directly")
    }

    fn http_get(&self, url: &str) -> Result<String> {
        let resp = reqwest::blocking::get(url)
            .map_err(|e| anyhow::anyhow!("HTTP GET error: {}", e))?;
        Ok(resp.text().unwrap_or_default())
    }

    fn http_post(&self, url: &str, body: &str) -> Result<String> {
        let client = reqwest::blocking::Client::new();
        let resp = client.post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .map_err(|e| anyhow::anyhow!("HTTP POST error: {}", e))?;
        Ok(resp.text().unwrap_or_default())
    }

    fn allow_shell(&self) -> bool { self.allow_shell }
}

// ─── MockEnv ───

pub struct MockEnv {
    pub stdin_lines: Vec<String>,
    stdin_index: usize,
    pub stdout_buffer: Vec<String>,
    pub files: std::collections::HashMap<String, String>,
    pub shell_responses: std::collections::HashMap<String, String>,
    pub llm_responses: Vec<LlmResponse>,
    llm_index: usize,
    pub allow_shell: bool,
}

impl MockEnv {
    pub fn new() -> Self {
        Self {
            stdin_lines: Vec::new(),
            stdin_index: 0,
            stdout_buffer: Vec::new(),
            files: std::collections::HashMap::new(),
            shell_responses: std::collections::HashMap::new(),
            llm_responses: Vec::new(),
            llm_index: 0,
            allow_shell: true,
        }
    }

    pub fn from_json(json: &serde_json::Value) -> Result<Self> {
        let mut env = Self::new();

        if let Some(stdin) = json.get("stdin").and_then(|v| v.as_array()) {
            env.stdin_lines = stdin.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(files) = json.get("files").and_then(|v| v.as_object()) {
            for (k, v) in files {
                if let Some(content) = v.as_str() {
                    env.files.insert(k.clone(), content.to_string());
                }
            }
        }

        if let Some(shell) = json.get("shell").and_then(|v| v.as_object()) {
            for (k, v) in shell {
                if let Some(output) = v.as_str() {
                    env.shell_responses.insert(k.clone(), output.to_string());
                }
            }
        }

        if let Some(llm) = json.get("llm_responses").and_then(|v| v.as_array()) {
            for resp in llm {
                if let Some(s) = resp.as_str() {
                    env.llm_responses.push(LlmResponse {
                        content: s.to_string(),
                        tool_calls: None,
                        raw_json: None,
                    });
                } else if resp.is_object() {
                    let content = resp.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let tool_calls = resp.get("tool_calls").and_then(|v| v.as_array()).map(|arr| arr.clone());
                    env.llm_responses.push(LlmResponse {
                        content,
                        tool_calls,
                        raw_json: Some(resp.clone()),
                    });
                }
            }
        }

        if let Some(allow) = json.get("allow_shell").and_then(|v| v.as_bool()) {
            env.allow_shell = allow;
        }

        Ok(env)
    }
}

impl Env for MockEnv {
    fn is_mock(&self) -> bool { true }

    fn read_stdin(&mut self) -> Result<String> {
        if self.stdin_index >= self.stdin_lines.len() {
            anyhow::bail!("end of input");
        }
        let line = self.stdin_lines[self.stdin_index].clone();
        self.stdin_index += 1;
        Ok(line)
    }

    fn write_stdout(&mut self, content: &str) -> Result<()> {
        self.stdout_buffer.push(content.to_string());
        Ok(())
    }

    fn read_file(&self, path: &str) -> Result<String> {
        self.files.get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("cannot read '{}': No such file or directory (os error 2)", path))
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<()> {
        self.files.insert(path.to_string(), content.to_string());
        log::info!("MockEnv: write_file({}, {} bytes)", path, content.len());
        Ok(())
    }

    fn exec_shell(&mut self, command: &str) -> Result<ShellResult> {
        // Try exact match first, then prefix match
        if let Some(output) = self.shell_responses.get(command) {
            return Ok(ShellResult { stdout: output.clone(), exit_code: 0 });
        }
        // Try matching just the base command (before |)
        let base = command.split('|').next().unwrap_or(command).trim();
        if let Some(output) = self.shell_responses.get(base) {
            return Ok(ShellResult { stdout: output.clone(), exit_code: 0 });
        }
        Ok(ShellResult { stdout: format!("mock: command '{}' not configured", command), exit_code: 1 })
    }

    fn call_llm(&mut self, _request: LlmRequest) -> Result<LlmResponse> {
        if self.llm_index >= self.llm_responses.len() {
            anyhow::bail!("MockEnv: no more LLM responses (used {})", self.llm_index);
        }
        let resp = self.llm_responses[self.llm_index].clone();
        self.llm_index += 1;
        Ok(resp)
    }

    fn http_get(&self, url: &str) -> Result<String> {
        self.files.get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MockEnv: no mock for GET {}", url))
    }

    fn http_post(&self, url: &str, _body: &str) -> Result<String> {
        self.files.get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MockEnv: no mock for POST {}", url))
    }

    fn allow_shell(&self) -> bool { self.allow_shell }

    fn captured_stdout(&self) -> Option<Vec<String>> {
        Some(self.stdout_buffer.clone())
    }
}
