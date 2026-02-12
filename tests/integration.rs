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

fn run_inline(src: &str, stdin: &str) -> (String, String, i32) {
    use std::io::Write as _;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.cog");
    std::fs::write(&file, src).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("run")
        .arg(&file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if !stdin.is_empty() {
                child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
            }
            child.wait_with_output()
        })
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.code().unwrap_or(-1))
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
    assert!(stdout.contains("flow main"));
    assert!(stdout.contains("think("));
    assert!(stdout.contains("write("));
    assert!(stdout.contains("loop:"));
}

#[test]
fn test_parse_roundtrip_hello() {
    let (stdout, _stderr, code) = parse_cog("hello.cog");
    assert_eq!(code, 0);
    assert!(stdout.contains("write("));
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
fn test_exec_shell_with_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shell.cog");
    std::fs::write(&path, concat!(
        "flow shell(command: String) -> String:\n",
        "    \"Execute a shell command\"\n",
        "    return __exec_shell__(command)\n",
        "\n",
        "flow main():\n",
        "    result = shell(\"echo hello from shell\")\n",
        "    emit(result)\n",
    )).unwrap();

    let bin = cognos_bin();
    // Without --allow-shell: should fail
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("shell execution is disabled"));

    // With --allow-shell: should work
    let output = Command::new(&bin).arg("run").arg("--allow-shell").arg(&path).output().unwrap();
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

// ─── Multi-flow tests ───

#[test]
fn test_flow_calling_flow() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multiflow.cog");
    std::fs::write(&path, concat!(
        "flow double(x: String) -> String:\n",
        "    return x + x\n",
        "\n",
        "flow main():\n",
        "    result = double(\"ha\")\n",
        "    emit(result)\n",
    )).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "haha");
}

#[test]
fn test_flow_with_return_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("retflow.cog");
    std::fs::write(&path, concat!(
        "flow add(a: Int, b: Int) -> Int:\n",
        "    return a + b\n",
        "\n",
        "flow main():\n",
        "    x = add(10, 20)\n",
        "    emit(x)\n",
    )).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "30");
}

// ─── F-string tests ───

#[test]
fn test_fstring_basic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fstr.cog");
    std::fs::write(&path, "flow main():\n    name = \"Cognos\"\n    emit(f\"Hello, {name}!\")\n").unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Hello, Cognos!");
}

#[test]
fn test_fstring_with_expressions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fstrexpr.cog");
    std::fs::write(&path, "flow main():\n    x = 10\n    y = 20\n    emit(f\"{x} + {y} = {x + y}\")\n").unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "10 + 20 = 30");
}

// ─── REPL tests ───

#[test]
fn test_repl_basic() {
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("repl")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"1 + 2\nexit\n").unwrap();
            child.wait_with_output()
        })
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = std::string::String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3"), "REPL should output 3, got: {}", stdout);
}

#[test]
fn test_infinite_loop_with_break() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    i = 0\n",
        "    loop:\n",
        "        emit(i)\n",
        "        i = i + 1\n",
        "        if i == 3:\n",
        "            break\n",
    ));
    assert_eq!(out.trim(), "0\n1\n2");
}

// ─── For loop tests ───

#[test]
fn test_for_over_list() {
    let out = expect_run_ok("flow main():\n    for x in [10, 20, 30]:\n        emit(x)\n");
    assert_eq!(out.trim(), "10\n20\n30");
}

#[test]
fn test_for_over_string() {
    let out = expect_run_ok("flow main():\n    for ch in \"abc\":\n        emit(ch)\n");
    assert_eq!(out.trim(), "a\nb\nc");
}

#[test]
fn test_for_over_map_keys() {
    let out = expect_run_ok("flow main():\n    for k in {\"x\": 1, \"y\": 2}:\n        emit(k)\n");
    assert_eq!(out.trim(), "x\ny");
}

#[test]
fn test_for_with_break() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    for x in [1, 2, 3, 4, 5]:\n",
        "        if x == 3:\n",
        "            break\n",
        "        emit(x)\n",
    ));
    assert_eq!(out.trim(), "1\n2");
}

#[test]
fn test_for_with_continue() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    for x in [1, 2, 3, 4]:\n",
        "        if x == 2:\n",
        "            continue\n",
        "        emit(x)\n",
    ));
    assert_eq!(out.trim(), "1\n3\n4");
}

#[test]
fn test_for_iterate_non_iterable() {
    let err = expect_error("flow main():\n    for x in 42:\n        emit(x)\n");
    assert!(err.contains("cannot iterate"), "for over int: {}", err);
}

// ─── Indexing tests ───

#[test]
fn test_list_indexing() {
    let out = expect_run_ok("flow main():\n    items = [10, 20, 30]\n    emit(items[0])\n    emit(items[2])\n    emit(items[-1])\n");
    assert_eq!(out.trim(), "10\n30\n30");
}

#[test]
fn test_string_indexing() {
    let out = expect_run_ok("flow main():\n    s = \"hello\"\n    emit(s[0])\n    emit(s[-1])\n");
    assert_eq!(out.trim(), "h\no");
}

#[test]
fn test_map_indexing() {
    let out = expect_run_ok("flow main():\n    m = {\"x\": 42}\n    emit(m[\"x\"])\n");
    assert_eq!(out.trim(), "42");
}

#[test]
fn test_index_out_of_range() {
    let err = expect_error("flow main():\n    items = [1, 2]\n    emit(items[5])\n");
    assert!(err.contains("out of range"), "index oob: {}", err);
}

// ─── Method tests ───

#[test]
fn test_string_methods() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    s = \"Hello World\"\n",
        "    emit(s.upper())\n",
        "    emit(s.lower())\n",
        "    emit(s.contains(\"World\"))\n",
        "    emit(s.starts_with(\"Hello\"))\n",
        "    emit(s.ends_with(\"World\"))\n",
        "    emit(s.replace(\"World\", \"Cognos\"))\n",
        "    emit(s.split(\" \"))\n",
        "    emit(\"  hi  \".strip())\n",
    ));
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec![
        "HELLO WORLD", "hello world", "true", "true", "true",
        "Hello Cognos", "[Hello, World]", "hi"
    ]);
}

#[test]
fn test_list_methods() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    items = [3, 1, 2]\n",
        "    emit(items.contains(2))\n",
        "    emit(items.contains(99))\n",
        "    emit(items.join(\"-\"))\n",
        "    emit(items.reversed())\n",
    ));
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["true", "false", "3-1-2", "[2, 1, 3]"]);
}

#[test]
fn test_map_methods() {
    let out = expect_run_ok("flow main():\n    m = {\"a\": 1, \"b\": 2}\n    emit(m.keys())\n    emit(m.values())\n    emit(m.contains(\"a\"))\n    emit(m.contains(\"z\"))\n    emit(m.length)\n");
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["[a, b]", "[1, 2]", "true", "false", "2"]);
}

#[test]
fn test_unknown_method() {
    let err = expect_error("flow main():\n    s = \"hi\"\n    emit(s.foobar())\n");
    assert!(err.contains("no method"), "unknown method: {}", err);
}

#[test]
fn test_unary_minus() {
    let out = expect_run_ok("flow main():\n    emit(-5)\n    emit(-3 + 10)\n");
    assert_eq!(out.trim(), "-5\n7");
}

#[test]
fn test_list_concatenation() {
    let out = expect_run_ok("flow main():\n    a = [1, 2] + [3, 4]\n    emit(a)\n    emit(a.length)\n");
    assert_eq!(out.trim(), "[1, 2, 3, 4]\n4");
}

#[test]
fn test_list_concat_empty() {
    let out = expect_run_ok("flow main():\n    a = [] + [1]\n    emit(a)\n");
    assert_eq!(out.trim(), "[1]");
}

// ─── All examples parse test ───

#[test]
fn test_all_examples_parse() {
    let bin = cognos_bin();
    let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    for entry in std::fs::read_dir(&examples_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "cog").unwrap_or(false) {
            let output = Command::new(&bin).arg("parse").arg(&path).output().unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "Failed to parse {}: {}", path.display(), stderr);
        }
    }
}

// ─── Unicode tests ───

#[test]
fn test_unicode_in_comments() {
    let out = expect_run_ok("# This is a comment with em dash \u{2014} and emoji \u{1f600}\nflow main():\n    write(stdout, \"ok\")\n");
    assert_eq!(out.trim(), "ok");
}

#[test]
fn test_unicode_in_strings() {
    let out = expect_run_ok("flow main():\n    write(stdout, \"hello \u{1f30d}\")\n    write(stdout, \"caf\u{e9}\")\n");
    assert_eq!(out.trim(), "hello \u{1f30d}\ncaf\u{e9}");
}

#[test]
fn test_unicode_in_fstrings() {
    let out = expect_run_ok("flow main():\n    name = \"world \u{1f30d}\"\n    write(stdout, f\"hello {name}\")\n");
    assert_eq!(out.trim(), "hello world \u{1f30d}");
}

// ─── Native module tests ───

// math module removed — P11: lean core runtime

#[test]
fn test_mixed_int_float_arithmetic() {
    let out = expect_run_ok("flow main():\n    write(stdout, 1 + 2.5)\n    write(stdout, 10.0 / 3)\n    write(stdout, 2 * 3.14)\n");
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines[0], "3.5");
    assert!(lines[1].starts_with("3.333"));
    assert_eq!(lines[2], "6.28");
}

// ─── Tool tests ───

#[test]
fn test_invoke_calls_flow_by_name() {
    // invoke() calls a flow by string name with a Map of arguments
    let out = expect_run_ok("flow greet(name: String) -> String:\n    return f\"Hello, {name}!\"\n\nflow main():\n    result = invoke(\"greet\", {\"name\": \"World\"})\n    write(stdout, result)\n");
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn test_flow_docstring() {
    // Docstring is extracted but doesn't affect execution
    let out = expect_run_ok("flow main():\n    \"This is a docstring\"\n    write(stdout, \"ok\")\n");
    assert_eq!(out.trim(), "ok");
}

#[test]
fn test_invoke_no_args() {
    // invoke() with no arguments map
    let out = expect_run_ok("flow hello() -> String:\n    return \"hi\"\n\nflow main():\n    result = invoke(\"hello\", {})\n    write(stdout, result)\n");
    assert_eq!(out.trim(), "hi");
}

// ─── Handle I/O tests ───

#[test]
fn test_read_write_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt").to_str().unwrap().to_string();
    let src = format!(
        "flow main():\n    write(file(\"{}\"), \"hello cognos\")\n    content = read(file(\"{}\"))\n    emit(content)\n",
        path, path
    );
    let out = expect_run_ok(&src);
    assert_eq!(out.trim(), "hello cognos");
}

#[test]
fn test_file_handle_display() {
    let out = expect_run_ok("flow main():\n    f = file(\"test.txt\")\n    emit(f)\n");
    assert_eq!(out.trim(), "file(\"test.txt\")");
}

#[test]
fn test_stdin_handle() {
    let out = expect_run_ok("flow main():\n    emit(stdin)\n");
    assert_eq!(out.trim(), "stdin");
}

// ─── Type definition tests ───

#[test]
fn test_type_definition_parses() {
    let out = expect_run_ok(concat!(
        "type Person:\n",
        "    name: String\n",
        "    age: Int\n",
        "\n",
        "flow main():\n",
        "    emit(\"types work\")\n",
    ));
    assert_eq!(out.trim(), "types work");
}

#[test]
fn test_type_with_nested_types() {
    // Just test it parses — no runtime execution of types without LLM
    let src = concat!(
        "type Address:\n",
        "    street: String\n",
        "    city: String\n",
        "\n",
        "type Person:\n",
        "    name: String\n",
        "    address: Address\n",
        "    tags: List[String]\n",
        "\n",
        "flow main():\n",
        "    emit(\"nested types parse\")\n",
    );
    let out = expect_run_ok(src);
    assert_eq!(out.trim(), "nested types parse");
}

// ─── REPL edge case tests ───

#[test]
fn test_repl_all_inputs() {
    // Test a wide range of REPL inputs — none should crash
    let inputs = vec![
        // Bare keywords
        "emit\n",
        "emit()\n",
        "think\n",
        "run\n",
        "log\n",
        "flow\n",
        "if\n",
        "loop\n",
        "break\n",
        "continue\n",
        "pass\n",
        "return\n",
        "true\n",
        "false\n",
        // Valid expressions
        "1\n",
        "1 + 2\n",
        "\"hello\"\n",
        "true and false\n",
        "not true\n",
        "[1, 2, 3]\n",
        "3 * 4\n",
        "10 / 2\n",
        // Valid statements
        "x = 42\n",
        "emit(42)\n",
        "emit(\"hello\")\n",
        "log(\"test\")\n",
        // Empty input
        "\n",
        // Nonsense
        "!!!\n",
        "@#$\n",
    ];

    let bin = cognos_bin();
    for input in &inputs {
        let full = format!("{}exit\n", input);
        let output = Command::new(&bin)
            .arg("repl")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(full.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .unwrap();
        assert_eq!(output.status.code().unwrap(), 0,
            "REPL crashed on input: {:?}\nstderr: {}",
            input, std::string::String::from_utf8_lossy(&output.stderr));
    }
}

// ─── Parser edge case tests ───

/// Helper: parse a .cog snippet, expect success
fn expect_parse_ok(code: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.cog");
    std::fs::write(&path, code).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin).arg("parse").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0,
        "Parse failed for:\n{}\nstderr: {}", code,
        std::string::String::from_utf8_lossy(&output.stderr));
}

/// Helper: run a .cog snippet, expect success, return stdout
fn expect_run_ok(code: &str) -> std::string::String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.cog");
    std::fs::write(&path, code).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0,
        "Run failed for:\n{}\nstderr: {}", code,
        std::string::String::from_utf8_lossy(&output.stderr));
    std::string::String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_parse_all_statement_types() {
    // pass
    expect_parse_ok("flow main():\n    pass\n");
    // emit
    expect_parse_ok("flow main():\n    emit(1)\n");
    // assignment
    expect_parse_ok("flow main():\n    x = 1\n");
    // return
    expect_parse_ok("flow main() -> Int:\n    return 1\n");
    // break/continue in loop
    expect_parse_ok("flow main():\n    loop max=1:\n        break\n");
    expect_parse_ok("flow main():\n    loop max=1:\n        continue\n");
    // if/elif/else
    expect_parse_ok("flow main():\n    if true:\n        pass\n    elif false:\n        pass\n    else:\n        pass\n");
    // loop
    expect_parse_ok("flow main():\n    loop max=5:\n        pass\n");
    // bare expression
    expect_parse_ok("flow main():\n    log(\"hi\")\n");
}

#[test]
fn test_parse_all_expression_types() {
    // Literals
    expect_parse_ok("flow main():\n    x = 42\n");
    expect_parse_ok("flow main():\n    x = 3.14\n");
    expect_parse_ok("flow main():\n    x = \"hello\"\n");
    expect_parse_ok("flow main():\n    x = true\n");
    expect_parse_ok("flow main():\n    x = false\n");
    expect_parse_ok("flow main():\n    x = [1, 2, 3]\n");
    expect_parse_ok("flow main():\n    x = {\"a\": 1}\n");
    // F-string
    expect_parse_ok("flow main():\n    x = f\"hello {1 + 2}\"\n");
    // Empty list/map
    expect_parse_ok("flow main():\n    x = []\n");
    expect_parse_ok("flow main():\n    x = {}\n");
    // Nested
    expect_parse_ok("flow main():\n    x = [[1], [2]]\n");
    // Parenthesized
    expect_parse_ok("flow main():\n    x = (1 + 2) * 3\n");
}

#[test]
fn test_all_operators() {
    let out = expect_run_ok("flow main():\n    emit(2 + 3)\n    emit(10 - 4)\n    emit(3 * 5)\n    emit(10 / 2)\n    emit(1 == 1)\n    emit(1 != 2)\n    emit(1 < 2)\n    emit(2 > 1)\n    emit(1 <= 1)\n    emit(2 >= 3)\n    emit(true and true)\n    emit(true or false)\n    emit(not false)\n");
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["5", "6", "15", "5", "true", "true", "true", "true", "true", "false", "true", "true", "true"]);
}

#[test]
fn test_flow_signatures() {
    // No params, no return
    expect_parse_ok("flow a():\n    pass\n");
    // One param
    expect_parse_ok("flow a(x: Int):\n    pass\n");
    // Multiple params
    expect_parse_ok("flow a(x: Int, y: String, z: Bool):\n    pass\n");
    // Return type
    expect_parse_ok("flow a() -> Int:\n    return 1\n");
    // Params + return
    expect_parse_ok("flow a(x: Int) -> Int:\n    return x\n");
    // Generic type
    expect_parse_ok("flow a(x: List[Int]):\n    pass\n");
}

#[test]
fn test_multiple_flows() {
    let out = expect_run_ok(concat!(
        "flow add(a: Int, b: Int) -> Int:\n",
        "    return a + b\n",
        "\n",
        "flow mul(a: Int, b: Int) -> Int:\n",
        "    return a * b\n",
        "\n",
        "flow main():\n",
        "    emit(add(2, 3))\n",
        "    emit(mul(4, 5))\n",
        "    emit(add(mul(2, 3), 4))\n",
    ));
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["5", "20", "10"]);
}

#[test]
fn test_fstring_edge_cases() {
    // Empty f-string
    let out = expect_run_ok("flow main():\n    emit(f\"\")\n");
    assert_eq!(out.trim(), "");
    // Only expression
    let out = expect_run_ok("flow main():\n    emit(f\"{42}\")\n");
    assert_eq!(out.trim(), "42");
    // Multiple expressions
    let out = expect_run_ok("flow main():\n    x = 1\n    y = 2\n    emit(f\"{x}+{y}={x+y}\")\n");
    assert_eq!(out.trim(), "1+2=3");
    // Expression with field access
    let out = expect_run_ok("flow main():\n    s = \"hello\"\n    emit(f\"len={s.length}\")\n");
    assert_eq!(out.trim(), "len=5");
}

#[test]
fn test_nested_control_flow() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    i = 0\n",
        "    loop max=3:\n",
        "        j = 0\n",
        "        loop max=3:\n",
        "            if i == j:\n",
        "                emit(f\"{i},{j}\")\n",
        "            j = j + 1\n",
        "        i = i + 1\n",
    ));
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["0,0", "1,1", "2,2"]);
}

#[test]
fn test_kwargs_on_functions() {
    // think() kwargs are already tested via LLM, test parse only
    expect_parse_ok("flow main():\n    x = think(\"hi\", model=\"test\", system=\"be nice\")\n");
    expect_parse_ok("flow main():\n    x = think(\"hi\", tools=[], model=\"test\")\n");
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
    assert!(err.contains("not supported"), "string + int: {}", err);

    // String - String → error
    let err = expect_error(r#"flow main():
    x = "a" - "b"
"#);
    assert!(err.contains("not supported"), "string - string: {}", err);

    // Int + String → error
    let err = expect_error(r#"flow main():
    x = 42 + "hello"
"#);
    assert!(err.contains("not supported"), "int + string: {}", err);

    // Bool + Bool → error
    let err = expect_error(r#"flow main():
    x = true + false
"#);
    assert!(err.contains("not supported"), "bool + bool: {}", err);

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
    assert!(err.contains("not supported"), "int == string: {}", err);
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

// ─── Import ───

#[test]
fn test_import() {
    let (out, _, code) = run_cog("import-test.cog", "");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "Hello, World!");
}

// ─── Try/Catch ───

#[test]
fn test_try_catch_file_error() {
    let (out, _, code) = run_cog("try-catch.cog", "");
    assert_eq!(code, 0);
    assert!(out.contains("Caught: cannot read 'nonexistent.txt'"));
    assert!(out.contains("x = 42"));
}

#[test]
fn test_try_catch_inline() {
    let src = r#"
flow main():
    try:
        x = 1 / 0
    catch err:
        write(stdout, f"Error: {err}")
"#;
    let (out, _, code) = run_inline(src, "");
    assert_eq!(code, 0);
    assert!(out.contains("Error:"), "got: {}", out);
}

#[test]
fn test_try_catch_no_error() {
    let src = r#"
flow main():
    try:
        x = 42
    catch err:
        write(stdout, "should not print")
    write(stdout, f"x={x}")
"#;
    let (out, _, code) = run_inline(src, "");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "x=42");
}

// ─── Save/Load ───

#[test]
fn test_save_load() {
    let (out, _, code) = run_cog("session-save.cog", "");
    assert_eq!(code, 0);
    assert!(out.contains("name=test"));
    assert!(out.contains("count=42"));
    // Clean up
    let _ = std::fs::remove_file("session.json");
}

#[test]
fn test_load_missing_file_with_try() {
    let src = r#"
flow main():
    try:
        data = load("nonexistent_session.json")
    catch:
        data = []
    write(stdout, f"data={data}")
"#;
    let (out, _, code) = run_inline(src, "");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "data=[]");
}

// ─── Mock Environment ───

fn run_test(cog_file: &str, env_file: &str) -> (String, String, i32) {
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", &format!("examples/{}", cog_file), "--env", &format!("examples/mocks/{}", env_file)])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.code().unwrap_or(-1))
}

#[test]
fn test_mock_chat() {
    let (out, _, code) = run_test("chat.cog", "chat-test.json");
    assert_eq!(code, 0);
    assert!(out.contains("Hi there!"));
    assert!(out.contains("Pass ✓"));
}

#[test]
fn test_mock_shell_agent() {
    let (out, _, code) = run_test("shell-agent.cog", "shell-agent-test.json");
    assert_eq!(code, 0);
    assert!(out.contains("Thursday, February 12th"));
    assert!(out.contains("Pass ✓"));
}

#[test]
fn test_mock_env_no_network() {
    // Mock env should complete instantly without any network calls
    let start = std::time::Instant::now();
    let (_, _, code) = run_test("chat.cog", "chat-test.json");
    assert_eq!(code, 0);
    assert!(start.elapsed().as_millis() < 2000, "Mock should be instant, took {}ms", start.elapsed().as_millis());
}

// ─── For key, value in map ───

#[test]
fn test_for_key_value_map() {
    let (out, _, code) = run_cog("for-map.cog", "");
    assert_eq!(code, 0);
    assert!(out.contains("name = cognos"));
    assert!(out.contains("version = 0.5"));
    assert!(out.contains("0: a"));
    assert!(out.contains("2: c"));
}

// ─── String/List slicing ───

#[test]
fn test_slicing() {
    let (out, _, code) = run_cog("slice-test.cog", "");
    assert_eq!(code, 0);
    assert!(out.contains("Hello"));
    assert!(out.contains("World!"));
    assert!(out.contains("[2, 3]"));
    assert!(out.contains("[1, 2]"));
    assert!(out.contains("[4, 5]"));
}

#[test]
fn test_slice_inline() {
    let src = r#"
flow main():
    s = "abcdef"
    write(stdout, s[2:4])
    write(stdout, f"{[10,20,30,40][1:3]}")
"#;
    let (out, _, code) = run_inline(src, "");
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "cd\n[20, 30]");
}

// ─── Session persistence ───

#[test]
fn test_session_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("session.json");
    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"
flow main():
    x = 42
    name = "test"
"#).unwrap();

    let bin = cognos_bin();
    Command::new(&bin)
        .args(&["run", "--session", session.to_str().unwrap(), cog.to_str().unwrap()])
        .output().unwrap();

    assert!(session.exists(), "session file should be created");
    let content = std::fs::read_to_string(&session).unwrap();
    assert!(content.contains("42"));
    assert!(content.contains("test"));
}

// ─── Type validation on think() ───

#[test]
fn test_format_validation_missing_field() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Review:
    score: Int
    summary: String

flow main():
    result = think("review this", format="Review")
    write(stdout, f"score={result[\"score\"]}")
"#).unwrap();
    // LLM returns JSON missing 'summary' field
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"score\": 8}"]}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("missing field 'summary'"), "got: {}", stderr);
}

#[test]
fn test_format_validation_wrong_type() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Review:
    score: Int
    summary: String

flow main():
    result = think("review this", format="Review")
"#).unwrap();
    // LLM returns score as string instead of int
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"score\": \"high\", \"summary\": \"good\"}"]}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("field 'score': expected Int"), "got: {}", stderr);
}

#[test]
fn test_format_validation_pass() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Review:
    score: Int
    summary: String

flow main():
    result = think("review this", format="Review")
    write(stdout, f"score={result[\"score\"]}, summary={result[\"summary\"]}")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"score\": 8, \"summary\": \"solid code\"}"]}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(stdout.contains("score=8"), "got: {}", stdout);
    assert!(stdout.contains("summary=solid code"), "got: {}", stdout);
}

// ═══════════════════════════════════════════════════════
// Edge case tests — hardening
// ═══════════════════════════════════════════════════════

// ─── Slicing edge cases ───

#[test]
fn test_slice_negative_indices() {
    let out = expect_run_ok(r#"flow main():
    s = "abcdef"
    write(stdout, s[-3:])
    write(stdout, s[1:-1])
    items = [10, 20, 30, 40, 50]
    write(stdout, f"{items[-2:]}")
"#);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines[0], "def");
    assert_eq!(lines[1], "bcde");
    assert_eq!(lines[2], "[40, 50]");
}

#[test]
fn test_slice_out_of_bounds() {
    // Should clamp, not crash
    let out = expect_run_ok(r#"flow main():
    s = "abc"
    write(stdout, s[0:100])
    write(stdout, s[50:100])
    items = [1, 2, 3]
    write(stdout, f"{items[0:100]}")
    write(stdout, f"{items[50:100]}")
"#);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines[0], "abc");
    assert_eq!(lines[1], "");
    assert_eq!(lines[2], "[1, 2, 3]");
    assert_eq!(lines[3], "[]");
}

#[test]
fn test_slice_empty() {
    let out = expect_run_ok(r#"flow main():
    s = "hello"
    write(stdout, f">{s[3:3]}<")
    items = [1, 2, 3]
    write(stdout, f"{items[2:2]}")
"#);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines[0], "><");
    assert_eq!(lines[1], "[]");
}

#[test]
fn test_slice_on_empty() {
    let out = expect_run_ok(r#"flow main():
    s = ""
    write(stdout, f">{s[0:]}<")
    items = []
    write(stdout, f"{items[0:]}")
"#);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines[0], "><");
    assert_eq!(lines[1], "[]");
}

// ─── For loop edge cases ───

#[test]
fn test_for_empty_map() {
    let out = expect_run_ok(r#"flow main():
    for k, v in {}:
        write(stdout, "should not print")
    write(stdout, "done")
"#);
    assert_eq!(out.trim(), "done");
}

#[test]
fn test_for_empty_list() {
    let out = expect_run_ok(r#"flow main():
    for item in []:
        write(stdout, "should not print")
    write(stdout, "done")
"#);
    assert_eq!(out.trim(), "done");
}

#[test]
fn test_for_kv_on_non_iterable() {
    let err = expect_error(r#"flow main():
    for k, v in 42:
        pass
"#);
    assert!(err.contains("requires a Map or List") || err.contains("cannot iterate"), "got: {}", err);
}

// ─── Import edge cases ───

#[test]
fn test_circular_import() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.cog");
    let b = dir.path().join("b.cog");
    std::fs::write(&a, format!("import \"{}\"\nflow a_flow():\n    pass\n", b.to_str().unwrap())).unwrap();
    std::fs::write(&b, format!("import \"{}\"\nflow b_flow():\n    pass\n", a.to_str().unwrap())).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin).arg("run").arg(&a).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "circular import should fail");
    assert!(stderr.contains("circular import"), "expected circular import error, got: {}", stderr);
}

#[test]
fn test_import_not_found() {
    let err = expect_error(r#"import "nonexistent_file_12345.cog"
flow main():
    pass
"#);
    assert!(err.contains("cannot import") || err.contains("No such file"), "got: {}", err);
}

// ─── Try/catch edge cases ───

#[test]
fn test_nested_try_catch() {
    let out = expect_run_ok(r#"flow main():
    try:
        try:
            x = 1 / 0
        catch inner_err:
            write(stdout, f"inner: {inner_err}")
            y = 1 / 0
    catch outer_err:
        write(stdout, f"outer: {outer_err}")
"#);
    assert!(out.contains("inner: division by zero"), "got: {}", out);
    assert!(out.contains("outer: division by zero"), "got: {}", out);
}

#[test]
fn test_try_catch_in_loop_with_break() {
    let out = expect_run_ok(r#"flow main():
    i = 0
    loop max=5:
        try:
            if i == 2:
                break
            write(stdout, f"{i}")
        catch:
            pass
        i = i + 1
"#);
    assert_eq!(out.trim(), "0\n1");
}

// ─── Type validation edge cases ───

#[test]
fn test_format_validation_extra_fields_ok() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Simple:
    name: String

flow main():
    result = think("test", format="Simple")
    write(stdout, f"name={result[\"name\"]}")
"#).unwrap();
    // Extra field "extra" should be fine
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"name\": \"test\", \"extra\": 42}"]}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert!(output.status.success(), "extra fields should pass, stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name=test"));
}

#[test]
fn test_format_json_no_type_validation() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Unused:
    required_field: String

flow main():
    result = think("test", format="json")
    write(stdout, f"got={result[\"anything\"]}")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"anything\": \"works\"}"]}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert!(output.status.success(), "format=json should not validate types, stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("got=works"));
}

// ─── Mock environment edge cases ───

#[test]
fn test_mock_no_llm_responses() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
flow main():
    result = think("hello")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": []}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no more LLM responses"), "got: {}", stderr);
}

#[test]
fn test_mock_no_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
flow main():
    x = read(stdin)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": []}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("end of input"), "got: {}", stderr);
}

#[test]
fn test_mock_shell_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
flow shell(cmd: String) -> String:
    "Run a shell command"
    return __exec_shell__(cmd)

flow main():
    result = shell("unknown_cmd")
    write(stdout, result)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": [], "shell": {}}"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not configured") || stdout.contains("mock"), "got: {}", stdout);
}

// ─── String/value edge cases ───

#[test]
fn test_unicode_in_fstrings_complex() {
    let out = expect_run_ok("flow main():\n    emoji = \"\u{1f680}\"\n    write(stdout, f\"Launch {emoji} now!\")\n");
    assert_eq!(out.trim(), "Launch \u{1f680} now!");
}

#[test]
fn test_very_long_string() {
    // Build a long string via concatenation
    let out = expect_run_ok(r#"flow main():
    s = "a"
    i = 0
    loop max=10:
        s = s + s
        i = i + 1
    write(stdout, s.length)
"#);
    assert_eq!(out.trim(), "1024");
}

#[test]
fn test_map_duplicate_keys() {
    // Last value wins in source map literal
    let out = expect_run_ok(r#"flow main():
    m = {"a": 1, "b": 2, "a": 3}
    write(stdout, m["a"])
"#);
    // Maps are ordered Vec, so both entries exist; index finds first
    let val = out.trim();
    // Either 1 or 3 is acceptable behavior, just shouldn't crash
    assert!(val == "1" || val == "3", "got: {}", val);
}

#[test]
fn test_nested_map_field_access() {
    let out = expect_run_ok(r#"flow main():
    m = {"a": {"b": "deep"}}
    write(stdout, m["a"]["b"])
"#);
    assert_eq!(out.trim(), "deep");
}

#[test]
fn test_list_of_maps_indexing() {
    let out = expect_run_ok(r#"flow main():
    items = [{"name": "first"}, {"name": "second"}]
    write(stdout, items[0]["name"])
    write(stdout, items[1]["name"])
"#);
    assert_eq!(out.trim(), "first\nsecond");
}

// ─── Session persistence edge cases ───

#[test]
fn test_session_nested_maps_and_lists() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("session.json");
    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"
flow main():
    data = {"items": [1, 2, 3], "meta": {"version": "1.0"}}
"#).unwrap();

    let bin = cognos_bin();
    Command::new(&bin)
        .args(&["run", "--session", session.to_str().unwrap(), cog.to_str().unwrap()])
        .output().unwrap();

    assert!(session.exists());
    let content = std::fs::read_to_string(&session).unwrap();
    assert!(content.contains("items"), "session should contain nested data: {}", content);
    assert!(content.contains("version"), "session should contain nested map: {}", content);
}

#[test]
fn test_session_skips_handles_and_modules() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("session.json");
    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"
flow main():
    x = 42
"#).unwrap();

    let bin = cognos_bin();
    Command::new(&bin)
        .args(&["run", "--session", session.to_str().unwrap(), cog.to_str().unwrap()])
        .output().unwrap();

    let content = std::fs::read_to_string(&session).unwrap();
    // stdin/stdout/http should be filtered out
    assert!(!content.contains("\"stdin\""), "session should not save stdin handle: {}", content);
    assert!(!content.contains("\"stdout\""), "session should not save stdout handle: {}", content);
}

// ─── For key/value on list ───

#[test]
fn test_for_index_value_list() {
    let out = expect_run_ok(r#"flow main():
    for i, v in ["a", "b", "c"]:
        write(stdout, f"{i}:{v}")
"#);
    assert_eq!(out.trim(), "0:a\n1:b\n2:c");
}

// ═══════════════════════════════════════════════════════
// Feature: Kwargs in flow calls
// ═══════════════════════════════════════════════════════

#[test]
fn test_flow_kwargs_only() {
    let out = expect_run_ok(concat!(
        "flow greet(name: String, greeting: String) -> String:\n",
        "    return f\"{greeting}, {name}!\"\n",
        "\n",
        "flow main():\n",
        "    result = greet(name=\"World\", greeting=\"Hello\")\n",
        "    write(stdout, result)\n",
    ));
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn test_flow_kwargs_mixed() {
    let out = expect_run_ok(concat!(
        "flow greet(name: String, greeting: String) -> String:\n",
        "    return f\"{greeting}, {name}!\"\n",
        "\n",
        "flow main():\n",
        "    result = greet(\"World\", greeting=\"Hello\")\n",
        "    write(stdout, result)\n",
    ));
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn test_flow_kwargs_unknown() {
    let err = expect_error(concat!(
        "flow greet(name: String) -> String:\n",
        "    return name\n",
        "\n",
        "flow main():\n",
        "    greet(name=\"World\", unknown=\"bad\")\n",
    ));
    assert!(err.contains("unknown keyword argument"), "got: {}", err);
}

#[test]
fn test_flow_kwargs_duplicate() {
    let err = expect_error(concat!(
        "flow greet(name: String, greeting: String) -> String:\n",
        "    return name\n",
        "\n",
        "flow main():\n",
        "    greet(\"World\", name=\"duplicate\")\n",
    ));
    assert!(err.contains("duplicate argument"), "got: {}", err);
}

#[test]
fn test_flow_kwargs_missing_required() {
    let err = expect_error(concat!(
        "flow greet(name: String, greeting: String) -> String:\n",
        "    return name\n",
        "\n",
        "flow main():\n",
        "    greet(name=\"World\")\n",
    ));
    assert!(err.contains("missing required argument"), "got: {}", err);
}

// ═══════════════════════════════════════════════════════
// Feature: Multi-line expressions
// ═══════════════════════════════════════════════════════

#[test]
fn test_multiline_function_call() {
    let out = expect_run_ok("flow add(a: Int, b: Int) -> Int:\n    return a + b\n\nflow main():\n    result = add(\n        10,\n        20\n    )\n    write(stdout, result)\n");
    assert_eq!(out.trim(), "30");
}

#[test]
fn test_multiline_list_literal() {
    let out = expect_run_ok("flow main():\n    items = [\n        1,\n        2,\n        3\n    ]\n    write(stdout, items)\n");
    assert_eq!(out.trim(), "[1, 2, 3]");
}

#[test]
fn test_multiline_map_literal() {
    let out = expect_run_ok("flow main():\n    m = {\n        \"a\": 1,\n        \"b\": 2\n    }\n    write(stdout, m[\"a\"])\n    write(stdout, m[\"b\"])\n");
    assert_eq!(out.trim(), "1\n2");
}

#[test]
fn test_multiline_flow_call_with_kwargs() {
    let out = expect_run_ok(concat!(
        "flow greet(name: String, greeting: String) -> String:\n",
        "    return f\"{greeting}, {name}!\"\n",
        "\n",
        "flow main():\n",
        "    result = greet(\n",
        "        \"World\",\n",
        "        greeting=\"Hello\"\n",
        "    )\n",
        "    write(stdout, result)\n",
    ));
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn test_multiline_nested_brackets() {
    let out = expect_run_ok("flow main():\n    items = [\n        [1, 2],\n        [3, 4]\n    ]\n    write(stdout, items[0][1])\n    write(stdout, items[1][0])\n");
    assert_eq!(out.trim(), "2\n3");
}

#[test]
fn test_multiline_method_chain() {
    let out = expect_run_ok("flow main():\n    result = [\n        \"hello\",\n        \"world\"\n    ].join(\" \")\n    write(stdout, result)\n");
    assert_eq!(out.trim(), "hello world");
}

// ─── Feature 1: String * repeat ───

#[test]
fn test_string_repeat_right() {
    let out = expect_run_ok("flow main():\n    write(stdout, \"=\" * 5)\n");
    assert_eq!(out.trim(), "=====");
}

#[test]
fn test_string_repeat_left() {
    let out = expect_run_ok("flow main():\n    write(stdout, 3 * \"ab\")\n");
    assert_eq!(out.trim(), "ababab");
}

#[test]
fn test_string_repeat_zero() {
    let out = expect_run_ok("flow main():\n    write(stdout, \"x\" * 0)\n");
    assert_eq!(out.trim(), "");
}

#[test]
fn test_string_repeat_one() {
    let out = expect_run_ok("flow main():\n    write(stdout, \"hello\" * 1)\n");
    assert_eq!(out.trim(), "hello");
}

#[test]
fn test_string_repeat_variable() {
    let out = expect_run_ok("flow main():\n    n = 4\n    s = \"-\" * n\n    write(stdout, s)\n");
    assert_eq!(out.trim(), "----");
}

// ─── Feature 2: Default parameter values ───

#[test]
fn test_default_param_basic() {
    let out = expect_run_ok(r#"flow greet(name: String, greeting: String = "Hello"):
    write(stdout, f"{greeting}, {name}!")

flow main():
    greet("World")
"#);
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn test_default_param_override() {
    let out = expect_run_ok(r#"flow greet(name: String, greeting: String = "Hello"):
    write(stdout, f"{greeting}, {name}!")

flow main():
    greet("World", "Hey")
"#);
    assert_eq!(out.trim(), "Hey, World!");
}

#[test]
fn test_default_param_int() {
    let out = expect_run_ok(r#"flow repeat_str(s: String, n: Int = 3):
    write(stdout, s * n)

flow main():
    repeat_str("x")
"#);
    assert_eq!(out.trim(), "xxx");
}

#[test]
fn test_default_param_multiple() {
    let out = expect_run_ok(r#"flow fmt(text: String, prefix: String = "[", suffix: String = "]"):
    write(stdout, f"{prefix}{text}{suffix}")

flow main():
    fmt("hi")
"#);
    assert_eq!(out.trim(), "[hi]");
}

#[test]
fn test_default_param_kwarg_override() {
    let out = expect_run_ok(r#"flow fmt(text: String, prefix: String = "[", suffix: String = "]"):
    write(stdout, f"{prefix}{text}{suffix}")

flow main():
    fmt("hi", suffix="!")
"#);
    assert_eq!(out.trim(), "[hi!");
}

// ─── Feature 3: exec via lib/exec.cog and agent_think via lib/agent.cog (tested via mock) ───

#[test]
fn test_exec_lib_mock() {
    let dir = tempfile::tempdir().unwrap();
    let lib_dir = dir.path().join("lib");
    std::fs::create_dir_all(&lib_dir).unwrap();

    // Write lib/exec.cog
    std::fs::write(lib_dir.join("exec.cog"), r#"flow exec(response: Map, tools: List = []) -> Map:
    "Execute tool calls from a think() response by invoking named flows"
    if response["has_tool_calls"] == false:
        return response
    results = []
    for call in response["tool_calls"]:
        name = call["name"]
        args = call["arguments"]
        result = invoke(name, args)
        results = results + [f"[Tool result from {name}]: {result}"]
    content = response["content"]
    tool_output = results.join("\n")
    return {"content": f"{content}\n{tool_output}", "has_tool_calls": false}
"#).unwrap();

    // Write lib/agent.cog
    std::fs::write(lib_dir.join("agent.cog"), r#"import "exec.cog"

flow agent_think(input: String, model: String = "qwen2.5:1.5b", system: String = "", tools: List = []) -> String:
    "Think with automatic tool execution. Loops until LLM gives final answer."
    context = input
    loop max=10:
        response = think(context, model=model, system=system, tools=tools)
        if response["has_tool_calls"] == false:
            if tools.length > 0:
                return response["content"]
            else:
                return response
        executed = exec(response, tools=tools)
        context = f"{input}\n{executed[\"content\"]}"
    return context
"#).unwrap();

    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");

    std::fs::write(&cog, r#"import "lib/agent.cog"

flow shell(command: String) -> String:
    "Execute a shell command"
    return __exec_shell__(command)

flow main():
    result = agent_think("list files", model="test", tools=["shell"])
    write(stdout, result)
"#).unwrap();

    // First response has tool_calls, second is final answer
    std::fs::write(&mock, r#"{
        "stdin": [],
        "llm_responses": [
            {
                "content": "Let me list files",
                "tool_calls": [{"name": "shell", "arguments": {"command": "ls"}}]
            },
            "Here are the files: a.txt b.txt"
        ],
        "shell": {"ls": "a.txt\nb.txt"},
        "files": {},
        "allow_shell": true
    }"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(["test", "--env"])
        .arg(&mock)
        .arg("--allow-shell")
        .arg(&cog)
        .output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code().unwrap(), 0,
        "agent_think test failed:\nstdout: {}\nstderr: {}", stdout, stderr);
    assert!(stdout.contains("Here are the files"), "Expected final text, got: {}", stdout);
}

#[test]
fn test_exec_lib_passthrough() {
    // Test that exec() passes through when no tool calls
    let dir = tempfile::tempdir().unwrap();
    let lib_dir = dir.path().join("lib");
    std::fs::create_dir_all(&lib_dir).unwrap();

    std::fs::write(lib_dir.join("exec.cog"), r#"flow exec(response: Map, tools: List = []) -> Map:
    "Execute tool calls from a think() response by invoking named flows"
    if response["has_tool_calls"] == false:
        return response
    results = []
    for call in response["tool_calls"]:
        name = call["name"]
        args = call["arguments"]
        result = invoke(name, args)
        results = results + [f"[Tool result from {name}]: {result}"]
    content = response["content"]
    tool_output = results.join("\n")
    return {"content": f"{content}\n{tool_output}", "has_tool_calls": false}
"#).unwrap();

    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"import "lib/exec.cog"

flow main():
    response = {"content": "just text", "has_tool_calls": false}
    result = exec(response)
    write(stdout, result["content"])
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("run")
        .arg(&cog)
        .output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "just text");
}

#[test]
fn test_exec_lib_tool_execution() {
    // Test that exec() actually invokes tool flows
    let dir = tempfile::tempdir().unwrap();
    let lib_dir = dir.path().join("lib");
    std::fs::create_dir_all(&lib_dir).unwrap();

    std::fs::write(lib_dir.join("exec.cog"), r#"flow exec(response: Map, tools: List = []) -> Map:
    "Execute tool calls from a think() response by invoking named flows"
    if response["has_tool_calls"] == false:
        return response
    results = []
    for call in response["tool_calls"]:
        name = call["name"]
        args = call["arguments"]
        result = invoke(name, args)
        results = results + [f"[Tool result from {name}]: {result}"]
    content = response["content"]
    tool_output = results.join("\n")
    return {"content": f"{content}\n{tool_output}", "has_tool_calls": false}
"#).unwrap();

    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"import "lib/exec.cog"

flow greet(name: String) -> String:
    return f"Hello, {name}!"

flow main():
    tc = [{"name": "greet", "arguments": {"name": "World"}}]
    response = {"content": "", "tool_calls": tc, "has_tool_calls": true}
    result = exec(response, tools=["greet"])
    write(stdout, result["content"])
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .arg("run")
        .arg(&cog)
        .output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, World!"), "Expected greeting in output, got: {}", stdout);
}

// ─── Feature 4: trace-to-mock ───

#[test]
fn test_trace_to_mock_basic() {
    let dir = tempfile::tempdir().unwrap();
    let trace_file = dir.path().join("trace.jsonl");

    std::fs::write(&trace_file, r#"{"event":"io","op":"read","handle":"stdin","content":"hello","bytes":5}
{"event":"llm_call","model":"test","response":"world","has_tool_calls":false}
{"event":"shell_exec","command":"ls","output":"a.txt","exit_code":0}
{"event":"io","op":"read","handle":"file","path":"input.txt","content":"data"}
"#).unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(["trace-to-mock"])
        .arg(&trace_file)
        .output().unwrap();

    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mock: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(mock["stdin"][0], "hello");
    assert_eq!(mock["llm_responses"][0], "world");
    assert_eq!(mock["shell"]["ls"], "a.txt");
    assert_eq!(mock["files"]["input.txt"], "data");
}

#[test]
fn test_trace_to_mock_empty() {
    let dir = tempfile::tempdir().unwrap();
    let trace_file = dir.path().join("empty.jsonl");
    std::fs::write(&trace_file, "").unwrap();

    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(["trace-to-mock"])
        .arg(&trace_file)
        .output().unwrap();

    assert_eq!(output.status.code().unwrap(), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mock: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(mock["stdin"].as_array().unwrap().is_empty());
}

#[test]
fn test_mock_devops_agent() {
    let (out, _, code) = run_test("devops-agent.cog", "devops-agent-sonnet.json");
    assert_eq!(code, 0);
    assert!(out.contains("Phase 1: System Health"));
    assert!(out.contains("Phase 2: Code Quality"));
    assert!(out.contains("Phase 3: Analysis"));
    assert!(out.contains("Pass ✓"));
}

// ═══════════════════════════════════════════════════════
// Type system improvements — Feature 1: Generic validation
// ═══════════════════════════════════════════════════════

#[test]
fn test_list_inner_type_validation_valid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Insight:
    text: String
    score: Int

type Review:
    insights: List[Insight]

flow main():
    result = think("review", format="Review")
    emit(result.insights.length)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"insights\": [{\"text\": \"good\", \"score\": 9}, {\"text\": \"bad\", \"score\": 2}]}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.starts_with("2"), "got: {}", stdout);
}

#[test]
fn test_list_inner_type_validation_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Insight:
    text: String
    score: Int

type Review:
    insights: List[Insight]

flow main():
    result = think("review", format="Review")
"#).unwrap();
    // Second element is just a string, not an Insight object
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"insights\": [{\"text\": \"good\", \"score\": 9}, \"just a string\"]}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("element [1]"), "got: {}", stderr);
}

#[test]
fn test_map_inner_type_validation_valid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Config:
    settings: Map[String, Int]

flow main():
    result = think("config", format="Config")
    emit("ok")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"settings\": {\"timeout\": 30, \"retries\": 3}}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_map_inner_type_validation_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Config:
    settings: Map[String, Int]

flow main():
    result = think("config", format="Config")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"settings\": {\"timeout\": 30, \"retries\": \"three\"}}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("key 'retries'"), "got: {}", stderr);
}

#[test]
fn test_untyped_list_no_validation() {
    // List without inner type should accept anything
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Review:
    tags: List

flow main():
    result = think("review", format="Review")
    emit(result.tags.length)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"tags\": [\"a\", 1, true]}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("3"), "got: {}", String::from_utf8_lossy(&output.stdout));
}

// ═══════════════════════════════════════════════════════
// Type system improvements — Feature 2: Optional fields
// ═══════════════════════════════════════════════════════

#[test]
fn test_optional_field_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Config:
    name: String
    description?: String
    timeout?: Int

flow main():
    result = think("config", format="Config")
    emit(result.name)
"#).unwrap();
    // Only name is provided, optional fields missing
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"name\": \"test\"}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("test"), "got: {}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_optional_field_present() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Config:
    name: String
    description?: String

flow main():
    result = think("config", format="Config")
    emit(result.description)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"name\": \"test\", \"description\": \"a config\"}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("a config"), "got: {}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_required_field_still_enforced_with_optionals() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Config:
    name: String
    description?: String

flow main():
    result = think("config", format="Config")
"#).unwrap();
    // Missing required field 'name'
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"description\": \"oops\"}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("missing field 'name'"), "got: {}", stderr);
}

// ═══════════════════════════════════════════════════════
// Type system improvements — Feature 3: Enum types
// ═══════════════════════════════════════════════════════

#[test]
fn test_enum_type_valid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Severity: "low" | "medium" | "high" | "critical"

type Issue:
    title: String
    severity: Severity

flow main():
    result = think("issue", format="Issue")
    emit(result.severity)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"title\": \"bug\", \"severity\": \"high\"}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("high"), "got: {}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_enum_type_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Severity: "low" | "medium" | "high" | "critical"

type Issue:
    title: String
    severity: Severity

flow main():
    result = think("issue", format="Issue")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"title\": \"bug\", \"severity\": \"extreme\"}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("not one of"), "got: {}", stderr);
}

#[test]
fn test_enum_parses_inline() {
    let out = expect_run_ok(r#"
type Status: "active" | "inactive" | "pending"

flow main():
    emit("enum parsed")
"#);
    assert_eq!(out.trim(), "enum parsed");
}

// ═══════════════════════════════════════════════════════
// Nested: List[Issue] where Issue has enum field
// ═══════════════════════════════════════════════════════

#[test]
fn test_nested_list_with_enum_valid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Severity: "low" | "medium" | "high"

type Issue:
    title: String
    severity: Severity

type Report:
    issues: List[Issue]

flow main():
    result = think("report", format="Report")
    emit(result.issues.length)
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"issues\": [{\"title\": \"bug1\", \"severity\": \"low\"}, {\"title\": \"bug2\", \"severity\": \"high\"}]}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("2"), "got: {}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_nested_list_with_enum_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Severity: "low" | "medium" | "high"

type Issue:
    title: String
    severity: Severity

type Report:
    issues: List[Issue]

flow main():
    result = think("report", format="Report")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"issues\": [{\"title\": \"bug1\", \"severity\": \"extreme\"}]}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(stderr.contains("not one of"), "got: {}", stderr);
}

// ═══════════════════════════════════════════════════════
// Schema generation tests
// ═══════════════════════════════════════════════════════

#[test]
fn test_optional_field_schema_generation() {
    // Optional fields should have ? in schema hint
    let out = expect_run_ok(r#"
type Config:
    name: String
    desc?: String

flow main():
    emit("schema ok")
"#);
    assert_eq!(out.trim(), "schema ok");
}

#[test]
fn test_generic_list_parses() {
    let out = expect_run_ok(r#"
type Item:
    name: String

type Container:
    items: List[Item]
    tags: List
    meta: Map[String, Int]

flow main():
    emit("generics parse")
"#);
    assert_eq!(out.trim(), "generics parse");
}

// ═══════════════════════════════════════════════════════
// Mock agent test with all new features
// ═══════════════════════════════════════════════════════

#[test]
fn test_full_type_system_agent() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    let mock = dir.path().join("mock.json");
    std::fs::write(&cog, r#"
type Priority: "low" | "medium" | "high"

type Task:
    title: String
    priority: Priority
    notes?: String

type Sprint:
    name: String
    tasks: List[Task]
    config: Map[String, Int]

flow main():
    result = think("plan sprint", format="Sprint")
    emit(result.name)
    for task in result.tasks:
        emit(f"{task.title} [{task.priority}]")
"#).unwrap();
    std::fs::write(&mock, r#"{"stdin": [], "llm_responses": ["{\"name\": \"Sprint 1\", \"tasks\": [{\"title\": \"Fix bug\", \"priority\": \"high\"}, {\"title\": \"Add feature\", \"priority\": \"medium\", \"notes\": \"needs design\"}], \"config\": {\"velocity\": 10, \"days\": 14}}"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", mock.to_str().unwrap()])
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Sprint 1"));
    assert!(stdout.contains("Fix bug [high]"));
    assert!(stdout.contains("Add feature [medium]"));
}

// ─── History & Clear History tests ───

#[test]
fn test_history_empty() {
    let src = r#"
flow main():
    h = history()
    print(h.length)
"#;
    let (out, _, code) = run_inline(src, "");
    assert_eq!(code, 0);
    assert!(out.trim().contains("0"));
}

#[test]
fn test_history_after_think() {
    // Use mock env via cognos test command
    // Instead, test history() builtin directly with inline + mock
    // We'll create a .cog + mock json for this
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"
flow main():
    result = think("hello", model="mock-model")
    h = history()
    print(h.length)
    print(h[0]["role"])
    print(h[0]["content"])
    print(h[1]["role"])
"#).unwrap();
    let env_json = dir.path().join("env.json");
    std::fs::write(&env_json, r#"{"llm_responses": ["Hi there!"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", env_json.to_str().unwrap()])
        .output().unwrap();
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(out.contains("2"), "expected 2 history entries, got: {}", out);
    assert!(out.contains("user"));
    assert!(out.contains("hello"));
    assert!(out.contains("assistant"));
}

#[test]
fn test_clear_history() {
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    std::fs::write(&cog, r#"
flow main():
    result = think("hello", model="mock-model")
    h1 = history()
    print(h1.length)
    clear_history()
    h2 = history()
    print(h2.length)
"#).unwrap();
    let env_json = dir.path().join("env.json");
    std::fs::write(&env_json, r#"{"llm_responses": ["Hi there!"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", env_json.to_str().unwrap()])
        .output().unwrap();
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let lines: Vec<&str> = out.trim().lines().collect();
    // First print should show 2, second should show 0
    assert!(lines.iter().any(|l| l.trim() == "2"), "expected '2' in output: {:?}", lines);
    assert!(lines.iter().any(|l| l.trim() == "0"), "expected '0' in output: {:?}", lines);
}

#[test]
fn test_compact_history_no_compaction_needed() {
    // Test compact_history when history is small — should return "No compaction needed"
    let dir = tempfile::tempdir().unwrap();
    let cog = dir.path().join("test.cog");
    // Copy lib/compact.cog to temp dir
    let lib_src = std::fs::read_to_string("lib/compact.cog").unwrap();
    let lib_path = dir.path().join("compact.cog");
    std::fs::write(&lib_path, &lib_src).unwrap();
    std::fs::write(&cog, format!(r#"
import "{}"

flow main():
    result = think("hello", model="mock-model")
    summary = compact_history(max_turns=10, model="mock-model")
    print(summary)
"#, lib_path.to_str().unwrap())).unwrap();
    let env_json = dir.path().join("env.json");
    std::fs::write(&env_json, r#"{"llm_responses": ["Hi there!"]}"#).unwrap();
    let bin = cognos_bin();
    let output = Command::new(&bin)
        .args(&["test", cog.to_str().unwrap(), "--env", env_json.to_str().unwrap()])
        .output().unwrap();
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(output.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(out.contains("No compaction needed"), "expected 'No compaction needed', got: {}", out);
}

#[test]
fn test_openai_provider_routing() {
    // Test that gpt-*, o1-*, o3-* models are recognized as OpenAI
    // We can't call the real API, but we test that the model name check works
    // by verifying the error message mentions OpenAI
    let src = r#"
flow main():
    result = think("hello", model="gpt-4o")
"#;
    let (_, err, code) = run_inline(src, "");
    // Should fail because no OPENAI_API_KEY is set (or no network)
    // But the error should mention OpenAI, proving routing worked
    assert_ne!(code, 0);
    assert!(err.contains("OPENAI_API_KEY") || err.contains("OpenAI"),
        "expected OpenAI-related error, got: {}", err);
}

#[test]
fn test_openai_provider_routing_o1() {
    let src = r#"
flow main():
    result = think("hello", model="o1-preview")
"#;
    let (_, err, code) = run_inline(src, "");
    assert_ne!(code, 0);
    assert!(err.contains("OPENAI_API_KEY") || err.contains("OpenAI"),
        "expected OpenAI-related error for o1 model, got: {}", err);
}

#[test]
fn test_openai_provider_routing_o3() {
    let src = r#"
flow main():
    result = think("hello", model="o3-mini")
"#;
    let (_, err, code) = run_inline(src, "");
    assert_ne!(code, 0);
    assert!(err.contains("OPENAI_API_KEY") || err.contains("OpenAI"),
        "expected OpenAI-related error for o3 model, got: {}", err);
}

// ─── Parallel & Async Tests ───

#[test]
fn test_parallel_basic() {
    let src = r#"
flow main:
    parallel:
        branch:
            a = 1 + 2
        branch:
            b = 3 + 4
        branch:
            c = 5 + 6
    emit(a)
    emit(b)
    emit(c)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["3", "7", "11"]);
}

#[test]
fn test_parallel_multi_stmt_branch() {
    let src = r#"
flow main:
    parallel:
        branch:
            x = 10
            y = x + 5
        branch:
            z = 3 * 7
    emit(y)
    emit(z)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["15", "21"]);
}

#[test]
fn test_parallel_string_ops() {
    let src = r#"
flow main:
    parallel:
        branch:
            x = "hello" + " world"
        branch:
            y = "foo" + "bar"
    emit(x)
    emit(y)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["hello world", "foobar"]);
}

#[test]
fn test_parallel_error_propagation() {
    let src = r#"
flow main:
    parallel:
        branch:
            a = 1 + 2
        branch:
            b = 10 / 0
    emit(a)
"#;
    let (_, _, code) = run_inline(src, "");
    assert_ne!(code, 0);
}

#[test]
fn test_parallel_uses_outer_vars() {
    let src = r#"
flow main:
    x = 10
    parallel:
        branch:
            a = x + 1
        branch:
            b = x + 2
    emit(a)
    emit(b)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["11", "12"]);
}

#[test]
fn test_async_await_basic() {
    let src = r#"
flow compute(x: Int) -> Int:
    return x * 2

flow main:
    handle = async compute(21)
    other = 100
    result = await(handle)
    emit(result)
    emit(other)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["42", "100"]);
}

#[test]
fn test_async_await_multiple() {
    let src = r#"
flow double(x: Int) -> Int:
    return x * 2

flow main:
    h1 = async double(5)
    h2 = async double(10)
    h3 = async double(15)
    r1 = await(h1)
    r2 = await(h2)
    r3 = await(h3)
    emit(r1)
    emit(r2)
    emit(r3)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["10", "20", "30"]);
}

#[test]
fn test_async_work_between() {
    let src = r#"
flow slow_add(a: Int, b: Int) -> Int:
    return a + b

flow main:
    handle = async slow_add(10, 20)
    x = 5 * 5
    result = await(handle)
    emit(x)
    emit(result)
"#;
    let (out, err, code) = run_inline(src, "");
    assert_eq!(code, 0, "stderr: {}", err);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines, vec!["25", "30"]);
}

#[test]
fn test_parallel_parse() {
    let src = r#"
flow main:
    parallel:
        branch:
            a = 1
        branch:
            b = 2
"#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.cog");
    std::fs::write(&file, src).unwrap();
    let bin = cognos_bin();
    let output = std::process::Command::new(&bin)
        .arg("parse")
        .arg(&file)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("parallel:"), "parse output should contain parallel: got {}", stdout);
    assert!(stdout.contains("branch:"), "parse output should contain branch: got {}", stdout);
}

#[test]
fn test_await_invalid_handle() {
    let src = r#"
flow main:
    result = await(42)
"#;
    let (_, _, code) = run_inline(src, "");
    assert_ne!(code, 0);
}

#[test]
fn test_parallel_no_branch_error() {
    // parallel: without branch: should fail to parse
    let src = r#"
flow main:
    parallel:
        a = 1
"#;
    let (_, _, code) = run_inline(src, "");
    assert_ne!(code, 0);
}
