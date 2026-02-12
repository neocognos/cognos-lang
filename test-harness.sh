#!/bin/bash
# Cognos Language Robustness Test Harness
# Runs varied agent sessions and logs issues
set -o pipefail
cd /home/reza/clawd/neocognos/cognos-lang
COGNOS="./target/release/cognos"
LOG="/tmp/cognos-test-harness-$(date +%Y%m%d-%H%M%S).log"
ISSUES="/tmp/cognos-issues-$(date +%Y%m%d-%H%M%S).txt"
PASS=0
FAIL=0
TOTAL=0

log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }
pass() { PASS=$((PASS+1)); TOTAL=$((TOTAL+1)); log "  ✅ PASS: $1"; }
fail() { FAIL=$((FAIL+1)); TOTAL=$((TOTAL+1)); log "  ❌ FAIL: $1"; echo "$1: $2" >> "$ISSUES"; }

run_test() {
    local name="$1" input="$2" file="$3" flags="$4" expect="$5" timeout_s="${6:-30}"
    log "TEST: $name"
    output=$(echo -e "$input" | timeout "$timeout_s" $COGNOS run "$file" $flags 2>&1)
    code=$?
    if [ $code -eq 124 ]; then
        fail "$name" "TIMEOUT after ${timeout_s}s"
        return
    fi
    if [ -n "$expect" ]; then
        if echo "$output" | grep -qF -- "$expect"; then
            pass "$name"
        else
            fail "$name" "Expected '$expect' not found. Exit=$code Output=$(echo "$output" | head -5)"
        fi
    elif [ $code -eq 0 ]; then
        pass "$name"
    else
        fail "$name" "Exit code $code. Output=$(echo "$output" | head -5)"
    fi
}

run_inline() {
    local name="$1" src="$2" input="$3" expect="$4" flags="$5" timeout_s="${6:-10}"
    local tmpfile=$(mktemp /tmp/cognos-test-XXXXXX.cog)
    echo "$src" > "$tmpfile"
    run_test "$name" "$input" "$tmpfile" "$flags" "$expect" "$timeout_s"
    rm -f "$tmpfile"
}

log "=========================================="
log "Cognos Robustness Test Harness"
log "=========================================="

# Build first
log "Building release..."
PATH="$HOME/.cargo/bin:$PATH" cargo build --release 2>&1 | tail -1 | tee -a "$LOG"
if [ $? -ne 0 ]; then
    log "BUILD FAILED — aborting"
    exit 1
fi

log "Running unit tests..."
result=$(PATH="$HOME/.cargo/bin:$PATH" cargo test 2>&1 | grep "test result" | tail -1)
log "  $result"

log ""
log "=== SECTION 1: Basic Programs ==="

run_test "empty" "" examples/empty.cog "" "" 5
run_test "hello" "" examples/hello.cog "" "Hello" 5
run_test "echo" "testing echo" examples/echo.cog "" "testing echo" 5
run_test "for-map" "" examples/for-map.cog "" "cognos" 5
run_test "slice-test" "" examples/slice-test.cog "" "Hello" 5
run_test "map-test" "" examples/map-test.cog "" "cognos" 5
run_test "try-catch" "" examples/try-catch.cog "" "Caught" 5
run_test "import-test" "World" examples/import-test.cog "" "Hello, World" 5

log ""
log "=== SECTION 2: Language Features ==="

run_inline "none-literal" 'flow main():
    x = none
    if x == none:
        write(stdout, "PASS")' "" "PASS"

run_inline "none-falsy" 'flow main():
    x = none
    if not x:
        write(stdout, "PASS")' "" "PASS"

run_inline "none-display" 'flow main():
    write(stdout, f"val={none}")' "" "val=none"

# eof-returns-none tested in integration tests (needs /dev/null stdin)

run_inline "comment-first-line" 'flow main():
    # comment
    write(stdout, "PASS")' "" "PASS"

run_inline "comment-nested" 'flow main():
    if true:
        # nested
        write(stdout, "PASS")' "" "PASS"

run_inline "comment-between-blocks" 'flow main():
    x = 1
    # between
    y = 2
    write(stdout, f"{x+y}")' "" "3"

run_inline "await-no-parens" 'flow task() -> String:
    return "done"
flow main():
    h = async task()
    r = await h
    write(stdout, r)' "" "done"

run_inline "await-with-parens" 'flow task() -> String:
    return "done"
flow main():
    h = async task()
    r = await(h)
    write(stdout, r)' "" "done"

run_inline "parallel-basic" 'flow main():
    parallel:
        branch:
            a = 1
        branch:
            b = 2
    write(stdout, f"{a+b}")' "" "3"

run_inline "select-break-propagates" 'flow fast() -> String:
    return "x"
flow main():
    count = 0
    loop:
        count = count + 1
        select:
            branch:
                r = fast()
                break
            branch:
                r = fast()
                break
    write(stdout, f"c={count}")' "" "c=1"

run_inline "loop-max" 'flow main():
    c = 0
    loop max=5:
        c = c + 1
    write(stdout, f"{c}")' "" "5"

run_inline "default-params" 'flow greet(name: String, greeting: String = "Hello"):
    write(stdout, f"{greeting} {name}")
flow main():
    greet("World")
    greet("World", greeting="Hi")' "" "Hello World"

run_inline "string-methods" 'flow main():
    s = "  Hello World  "
    write(stdout, s.strip())
    write(stdout, s.strip().lower())
    write(stdout, s.strip().upper())
    write(stdout, "hello".replace("l", "L"))' "" "heLLo"

run_inline "list-operations" 'flow main():
    l = [1, 2, 3] + [4, 5]
    write(stdout, f"len={l.length}")
    write(stdout, f"has3={l.contains(3)}")
    write(stdout, f"has9={l.contains(9)}")
    write(stdout, l.join(","))' "" "1,2,3,4,5"

run_inline "map-operations" 'flow main():
    m = {"a": 1, "b": 2}
    m["c"] = 3
    write(stdout, f"len={m.length}")
    m2 = remove(m, "b")
    write(stdout, f"len2={m2.length}")' "" "len2=2"

run_inline "string-repeat" 'flow main():
    s = "ab" * 3
    write(stdout, s)' "" "ababab"

run_inline "negative-slice" 'flow main():
    s = "hello"
    write(stdout, s[-2:])' "" "lo"

run_inline "mixed-arithmetic" 'flow main():
    write(stdout, f"{1.5 + 2}")
    write(stdout, f"{3 * 1.5}")
    write(stdout, f"{0 - 5}")' "" "-5"

run_inline "for-map-kv" 'flow main():
    m = {"x": 1, "y": 2}
    for k, v in m:
        write(stdout, f"{k}={v}")' "" "x=1"

run_inline "type-definition" 'type Point:
    x: Int
    y: Int
flow main():
    p = {"x": 10, "y": 20}
    write(stdout, f"{p.x},{p.y}")' "" "10,20"

run_inline "optional-field" 'type Config:
    name: String
    debug?: Bool
flow main():
    c = {"name": "test"}
    write(stdout, c.name)' "" "test"

run_inline "enum-type" 'type Level: "low" | "medium" | "high"
flow main():
    write(stdout, "PASS")' "" "PASS"

run_inline "try-catch-variable" 'flow main():
    try:
        x = read(file("nonexistent.txt"))
    catch err:
        write(stdout, f"caught: {err}")' "" "caught:"

run_inline "save-load" 'flow main():
    save("/tmp/cognos-harness-test.json", {"key": "value", "num": 42})
    data = load("/tmp/cognos-harness-test.json")
    write(stdout, f"{data.key},{data.num}")' "" "value,42"

run_inline "fstring-expressions" 'flow main():
    x = 5
    write(stdout, f"calc={x * 2 + 1}")
    write(stdout, f"bool={x > 3}")
    write(stdout, f"list={[1,2,3]}")' "" "calc=11"

run_inline "multi-flow" 'flow add(a: Int, b: Int) -> Int:
    return a + b
flow mul(a: Int, b: Int) -> Int:
    return a * b
flow main():
    write(stdout, f"{add(3, 4)}")
    write(stdout, f"{mul(3, 4)}")' "" "12"

run_inline "kwargs-in-call" 'flow greet(name: String, loud: Bool = false) -> String:
    if loud:
        return name.upper()
    return name
flow main():
    write(stdout, greet("hello", loud=true))' "" "HELLO"

run_inline "docstring" 'flow helper():
    "This is a docstring"
    write(stdout, "PASS")
flow main():
    helper()' "" "PASS"

run_inline "pass-statement" 'flow empty():
    pass
flow main():
    empty()
    write(stdout, "PASS")' "" "PASS"

run_inline "cancel-async" 'flow slow() -> String:
    return "slow"
flow main():
    h = async slow()
    cancel(h)
    write(stdout, "PASS")' "" "PASS"

run_inline "nested-if" 'flow main():
    x = 5
    if x > 3:
        if x < 10:
            write(stdout, "PASS")' "" "PASS"

run_inline "elif" 'flow main():
    x = 2
    if x == 1:
        write(stdout, "one")
    elif x == 2:
        write(stdout, "two")
    else:
        write(stdout, "other")' "" "two"

run_inline "loop-break-continue" 'flow main():
    result = []
    loop:
        i = result.length
        if i >= 5:
            break
        if i == 2:
            result = result + [0]
            continue
        result = result + [i]
    write(stdout, result.join(","))' "" "0,1,0,3,4"

run_inline "for-list" 'flow main():
    total = 0
    for x in [10, 20, 30]:
        total = total + x
    write(stdout, f"{total}")' "" "60"

run_test "import-chain" "Test" examples/import-test.cog "" "Hello," 5

log ""
log "=== SECTION 3: LLM Integration (DeepSeek) ==="

run_inline "deepseek-simple" 'flow main():
    r = think("Say exactly: HELLO123", model="deepseek-chat", system="Output only what is asked, nothing else.")
    write(stdout, r.content)' "" "HELLO123" "" 15

run_inline "deepseek-tools" 'flow get_time() -> String:
    "Get current time"
    return "12:00"
flow main():
    r = think("What time is it?", model="deepseek-chat", tools=["get_time"], system="Use tools.")
    if r.has_tool_calls:
        write(stdout, "TOOL_CALLED")
    else:
        write(stdout, "NO_TOOL")' "" "" "" 15

run_inline "deepseek-format" 'type Answer:
    value: String
flow main():
    r = think("What is 2+2? Answer with value field only.", format="Answer", model="deepseek-chat", system="Be precise.")
    write(stdout, f"type={r.value}")' "" "type=" "" 15

run_inline "deepseek-system-prompt" 'flow main():
    r = think("Who are you?", model="deepseek-chat", system="You are Bob. Always say your name is Bob.")
    if r.content.contains("Bob"):
        write(stdout, "PASS")
    else:
        write(stdout, f"FAIL: {r.content}")' "" "PASS" "" 15

log ""
log "=== SECTION 4: Shell & File Tools ==="

run_inline "shell-exec" 'flow main():
    r = __exec_shell__("echo hello_from_shell")
    write(stdout, r)' "" "hello_from_shell" "--allow-shell" 5

run_inline "file-write-read" 'flow main():
    write(file("/tmp/cognos-harness-rw.txt"), "test content 123")
    c = read(file("/tmp/cognos-harness-rw.txt"))
    write(stdout, c)' "" "test content 123" "" 5

run_inline "shell-tool-deepseek" 'flow shell(command: String) -> String:
    "Run a shell command"
    return __exec_shell__(command)
flow exec_tools(response: Map) -> Map:
    "Execute tool calls"
    results = []
    for call in response["tool_calls"]:
        result = invoke(call["name"], call["arguments"])
        results = results + [result]
    return {"content": results.join("\n"), "has_tool_calls": false}
flow main():
    r = think("Run: echo MARKER42", model="deepseek-chat", tools=["shell"], system="Use the shell tool. Run the exact command given.")
    if r.has_tool_calls:
        result = exec_tools(r)
        write(stdout, result.content)
    else:
        write(stdout, "no tool call")' "" "MARKER42" "--allow-shell" 20

log ""
log "=== SECTION 5: Agent Sessions ==="

run_test "general-assistant-basic" "hello\nexit" examples/general-assistant.cog "--allow-shell" "Assistant ready" 30
run_test "general-assistant-tool" "What is the current date?\nexit" examples/general-assistant.cog "--allow-shell" "" 30
run_test "tool-agent-weather" "What is the weather in London?\nquit" examples/tool-agent.cog "--allow-shell" "" 30
run_test "chat-single" "Hi there\nquit" examples/chat.cog "" "" 20

log ""
log "=== SECTION 6: Edge Cases & Error Handling ==="

run_inline "div-by-zero" 'flow main():
    try:
        x = 1 / 0
    catch err:
        write(stdout, "caught")' "" "caught"

run_inline "undefined-var" 'flow main():
    try:
        write(stdout, f"{undefined_var}")
    catch err:
        write(stdout, "caught")' "" "caught"

run_inline "empty-list-ops" 'flow main():
    l = []
    write(stdout, f"len={l.length}")
    write(stdout, l.join(","))' "" "len=0"

run_inline "empty-map" 'flow main():
    m = {}
    write(stdout, f"len={m.length}")' "" "len=0"

run_inline "empty-string-methods" 'flow main():
    s = ""
    write(stdout, f"len={s.length}")
    write(stdout, f"strip={s.strip()}")
    write(stdout, f"lower={s.lower()}")' "" "len=0"

run_inline "deep-nesting" 'flow main():
    if true:
        if true:
            if true:
                if true:
                    write(stdout, "DEEP")' "" "DEEP"

run_inline "large-list" 'flow main():
    l = []
    loop max=100:
        l = l + [l.length]
    write(stdout, f"len={l.length}")' "" "len=100"

run_inline "recursive-flow" 'flow fib(n: Int) -> Int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
flow main():
    write(stdout, f"{fib(10)}")' "" "55"

run_inline "multiline-expr" 'flow main():
    result = (
        1 + 2 +
        3 + 4
    )
    write(stdout, f"{result}")' "" "10"

run_inline "map-dot-access" 'flow main():
    m = {"name": "test", "value": 42}
    write(stdout, f"{m.name}:{m.value}")' "" "test:42"

run_inline "bool-comparison" 'flow main():
    write(stdout, f"{true == true}")
    write(stdout, f"{false == false}")
    write(stdout, f"{true != false}")
    write(stdout, f"{none == none}")' "" "true"

# Summary
log ""
log "=========================================="
log "RESULTS: $PASS passed, $FAIL failed, $TOTAL total"
log "=========================================="
log "Log: $LOG"
if [ $FAIL -gt 0 ]; then
    log "Issues: $ISSUES"
    cat "$ISSUES" | tee -a "$LOG"
fi

# Copy latest issues to stable location
cp "$ISSUES" /tmp/cognos-latest-issues.txt 2>/dev/null
echo "$PASS/$TOTAL" > /tmp/cognos-test-result.txt
