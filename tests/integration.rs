/// Integration tests for Cognos.
/// Each test runs a .cog file and checks stdout/stderr/exit code.

use std::process::Command;
use std::path::PathBuf;

fn cognos_bin() -> PathBuf {
    // cargo test builds to target/debug
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("cognos");
    path
}

fn run_cog(file: &str, stdin: &str) -> (String, String, i32) {
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("run")
        .arg(format!("examples/{}", file))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if !stdin.is_empty() {
                child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
            }
            child.wait_with_output()
        })
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

fn parse_cog(file: &str) -> (String, String, i32) {
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("parse")
        .arg(format!("examples/{}", file))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

// ─── Run tests ───

#[test]
fn test_empty_program() {
    let (stdout, _stderr, code) = run_cog("empty.cog", "");
    assert_eq!(code, 0, "empty program should exit 0");
    assert_eq!(stdout, "", "empty program should produce no output");
}

#[test]
fn test_hello_world() {
    let (stdout, _stderr, code) = run_cog("hello.cog", "");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "Hello, World!");
}

#[test]
fn test_echo() {
    let (stdout, _stderr, code) = run_cog("echo.cog", "test input\n");
    assert_eq!(code, 0);
    // stdout includes the "> " prompt since stdin is piped
    assert!(stdout.contains("test input"));
}

// ─── Parse tests ───

#[test]
fn test_parse_empty() {
    let (stdout, _stderr, code) = parse_cog("empty.cog");
    assert_eq!(code, 0);
    assert!(stdout.contains("Parsed 1 flow(s)"));
    assert!(stdout.contains("flow main"));
}

#[test]
fn test_parse_general_assistant() {
    let (stdout, _stderr, code) = parse_cog("general-assistant.cog");
    assert_eq!(code, 0);
    assert!(stdout.contains("Parsed 1 flow(s)"));
    assert!(stdout.contains("flow general_assistant("));
    assert!(stdout.contains("think("));
    assert!(stdout.contains("emit("));
    assert!(stdout.contains("loop max=30"));
}

#[test]
fn test_parse_roundtrip_hello() {
    let (stdout, _stderr, code) = parse_cog("hello.cog");
    assert_eq!(code, 0);
    assert!(stdout.contains("emit(\"Hello, World!\")"));
}

// ─── Language feature tests ───

#[test]
fn test_variables_and_string_output() {
    // Create a temp .cog file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vars.cog");
    std::fs::write(&path, r#"flow main():
    x = "hello"
    y = "world"
    emit(x)
    emit(y)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("run")
        .arg(&path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "hello\nworld");
}

#[test]
fn test_if_else() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ifelse.cog");
    std::fs::write(&path, r#"flow main():
    x = true
    if x:
        emit("yes")
    else:
        emit("no")
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "yes");
}

#[test]
fn test_if_false_branch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("iffalse.cog");
    std::fs::write(&path, r#"flow main():
    x = false
    if x:
        emit("yes")
    else:
        emit("no")
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "no");
}

#[test]
fn test_loop_with_break() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loop.cog");
    std::fs::write(&path, r#"flow main():
    i = 0
    loop max=5:
        emit(i)
        i = i + 1
        if i == 3:
            break
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "0\n1\n2");
}

#[test]
fn test_int_arithmetic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("math.cog");
    std::fs::write(&path, r#"flow main():
    x = 10 + 5
    y = x - 3
    emit(y)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "12");
}

#[test]
fn test_string_comparison() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("strcmp.cog");
    std::fs::write(&path, r#"flow main():
    x = "hello"
    if x == "hello":
        emit("match")
    else:
        emit("no match")
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "match");
}

#[test]
fn test_run_shell_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shell.cog");
    std::fs::write(&path, r#"flow main():
    result = run("echo hello from shell")
    emit(result)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "hello from shell");
}

#[test]
fn test_pass_statement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pass.cog");
    std::fs::write(&path, r#"flow main():
    pass
    emit("after pass")
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "after pass");
}

// ─── Error tests ───

#[test]
fn test_undefined_variable_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("undef.cog");
    std::fs::write(&path, r#"flow main():
    emit(x)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_ne!(output.status.code().unwrap(), 0);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("undefined variable"));
}

#[test]
fn test_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.cog");
    std::fs::write(&path, "this is not valid cognos\n").unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_ne!(output.status.code().unwrap(), 0);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Parse error") || stderr.contains("expected"));
}
