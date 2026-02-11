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

#[test]
fn test_map_literal_and_field_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map.cog");
    std::fs::write(&path, r#"flow main():
    m = {"name": "cognos", "ver": "0.1"}
    emit(m.name)
    emit(m.ver)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "cognos\n0.1");
}

#[test]
fn test_map_truthy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("maptruthy.cog");
    std::fs::write(&path, r#"flow main():
    empty = {}
    full = {"a": 1}
    if empty:
        emit("empty truthy")
    else:
        emit("empty falsy")
    if full:
        emit("full truthy")
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "empty falsy\nfull truthy");
}

// ─── Type tests: all primitives ───

#[test]
fn test_all_types() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("types.cog");
    std::fs::write(&path, "flow main():\n    s = \"hello\"\n    emit(s)\n    i = 42\n    emit(i)\n    f = 3.14\n    emit(f)\n    b = true\n    emit(b)\n    l = [1, 2, 3]\n    emit(l)\n    m = {\"a\": 1, \"b\": 2}\n    emit(m)\n").unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "42");
    assert_eq!(lines[2], "3.14");
    assert_eq!(lines[3], "true");
    assert_eq!(lines[4], "[1, 2, 3]");
    assert_eq!(lines[5], r#"{"a": 1, "b": 2}"#);
}

// ─── Type operation tests ───

#[test]
fn test_string_operations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("strops.cog");
    std::fs::write(&path, concat!(
        "flow main():\n",
        "    a = \"hello\" + \" \" + \"world\"\n",
        "    emit(a)\n",
        "    emit(a.length)\n",
        "    if \"abc\" == \"abc\":\n",
        "        emit(\"eq works\")\n",
        "    if \"abc\" != \"xyz\":\n",
        "        emit(\"neq works\")\n",
        "    if \"non-empty\":\n",
        "        emit(\"truthy\")\n",
    )).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "hello world");
    assert_eq!(lines[1], "11");
    assert_eq!(lines[2], "eq works");
    assert_eq!(lines[3], "neq works");
    assert_eq!(lines[4], "truthy");
}

#[test]
fn test_int_operations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intops.cog");
    std::fs::write(&path, r#"flow main():
    emit(10 + 5)
    emit(10 - 3)
    emit(10 == 10)
    emit(10 != 5)
    emit(3 < 5)
    emit(5 > 3)
    emit(3 <= 3)
    emit(3 >= 4)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "15");
    assert_eq!(lines[1], "7");
    assert_eq!(lines[2], "true");
    assert_eq!(lines[3], "true");
    assert_eq!(lines[4], "true");
    assert_eq!(lines[5], "true");
    assert_eq!(lines[6], "true");
    assert_eq!(lines[7], "false");
}

#[test]
fn test_bool_operations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boolops.cog");
    std::fs::write(&path, r#"flow main():
    emit(true and true)
    emit(true and false)
    emit(false or true)
    emit(false or false)
    emit(not false)
    emit(not true)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["true", "false", "true", "false", "true", "false"]);
}

#[test]
fn test_list_operations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("listops.cog");
    std::fs::write(&path, r#"flow main():
    l = [10, 20, 30]
    emit(l.length)
    emit(l)

    # Empty list is falsy
    empty = []
    if empty:
        emit("truthy")
    else:
        emit("falsy")

    # Non-empty is truthy
    if l:
        emit("truthy")
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "3");
    assert_eq!(lines[1], "[10, 20, 30]");
    assert_eq!(lines[2], "falsy");
    assert_eq!(lines[3], "truthy");
}

#[test]
fn test_map_field_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mapfield.cog");
    std::fs::write(&path, r#"flow main():
    person = {"name": "Reza", "age": 30, "active": true}
    emit(person.name)
    emit(person.age)
    emit(person.active)
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["Reza", "30", "true"]);
}

// ─── Type error tests (all in one) ───

/// Helper: run a .cog snippet, expect failure, return stderr
fn expect_error(code: &str) -> std::string::String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("err.cog");
    std::fs::write(&path, code).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_ne!(output.status.code().unwrap(), 0, "expected error for:\n{}", code);
    std::string::String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn test_type_errors() {
    // String + Int → error
    let err = expect_error(r#"flow main():
    x = "hello" + 42
"#);
    assert!(err.contains("unsupported operation"), "string + int: {}", err);

    // String - String → error
    let err = expect_error(r#"flow main():
    x = "a" - "b"
"#);
    assert!(err.contains("unsupported operation"), "string - string: {}", err);

    // Int + String → error
    let err = expect_error(r#"flow main():
    x = 42 + "hello"
"#);
    assert!(err.contains("unsupported operation"), "int + string: {}", err);

    // Bool + Bool → error
    let err = expect_error(r#"flow main():
    x = true + false
"#);
    assert!(err.contains("unsupported operation"), "bool + bool: {}", err);

    // Division by zero
    let err = expect_error(r#"flow main():
    x = 10 / 0
"#);
    assert!(err.contains("division by zero"), "div by zero: {}", err);

    // Undefined variable
    let err = expect_error(r#"flow main():
    emit(x)
"#);
    assert!(err.contains("undefined variable"), "undef: {}", err);

    // Field access on non-map/non-string
    let err = expect_error(r#"flow main():
    x = 42
    emit(x.length)
"#);
    assert!(err.contains("cannot access field"), "field on int: {}", err);

    // Map missing key
    let err = expect_error("flow main():\n    m = {\"a\": 1}\n    emit(m.b)\n");
    assert!(err.contains("no key"), "missing key: {}", err);

    // Unknown function
    let err = expect_error(r#"flow main():
    x = foobar()
"#);
    assert!(err.contains("unknown function"), "unknown fn: {}", err);

    // Parse error
    let err = expect_error("this is not valid cognos\n");
    assert!(err.contains("Parse error") || err.contains("expected"), "parse error: {}", err);

    // Int comparison with String → error
    let err = expect_error(r#"flow main():
    x = 42 == "hello"
"#);
    assert!(err.contains("unsupported operation"), "int == string: {}", err);
}

// ─── Truthiness tests ───

#[test]
fn test_truthiness_all_types() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("truthy.cog");
    std::fs::write(&path, concat!(
        "flow main():\n",
        "    if not false:\n",
        "        emit(\"false is falsy\")\n",
        "    if not 0:\n",
        "        emit(\"0 is falsy\")\n",
        "    if not \"\":\n",
        "        emit(\"empty string is falsy\")\n",
        "    if not []:\n",
        "        emit(\"empty list is falsy\")\n",
        "    if not {}:\n",
        "        emit(\"empty map is falsy\")\n",
        "    if true:\n",
        "        emit(\"true is truthy\")\n",
        "    if 1:\n",
        "        emit(\"1 is truthy\")\n",
        "    if \"x\":\n",
        "        emit(\"non-empty string is truthy\")\n",
        "    if [1]:\n",
        "        emit(\"non-empty list is truthy\")\n",
        "    if {\"a\": 1}:\n",
        "        emit(\"non-empty map is truthy\")\n",
    )).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec![
        "false is falsy",
        "0 is falsy",
        "empty string is falsy",
        "empty list is falsy",
        "empty map is falsy",
        "true is truthy",
        "1 is truthy",
        "non-empty string is truthy",
        "non-empty list is truthy",
        "non-empty map is truthy",
    ]);
}
