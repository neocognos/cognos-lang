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

// ─── Tool tests ───

#[test]
fn test_act_executes_tool_flow() {
    // Simulate a think() response with tool_calls, then act() on it
    let out = expect_run_ok("flow greet(name: String) -> String:\n    return f\"Hello, {name}!\"\n\nflow main():\n    tc = [{\"name\": \"greet\", \"arguments\": {\"name\": \"World\"}}]\n    response = {\"content\": \"\", \"tool_calls\": tc, \"has_tool_calls\": true}\n    result = act(response, tools=[\"greet\"])\n    write(stdout, result.tool_results[0].result)\n");
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn test_act_no_tool_calls_passthrough() {
    let out = expect_run_ok(concat!(
        "flow main():\n",
        "    response = {\"content\": \"just text\", \"has_tool_calls\": false}\n",
        "    result = act(response)\n",
        "    write(stdout, result.content)\n",
    ));
    assert_eq!(out.trim(), "just text");
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
