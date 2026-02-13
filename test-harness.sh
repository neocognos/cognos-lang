#!/bin/bash
# Cognos Language Robustness Test Harness
# Runs varied agent sessions with randomized inputs and logs issues
set -o pipefail
cd /home/reza/clawd/neocognos/cognos-lang
COGNOS="./target/release/cognos"
LOG="/tmp/cognos-test-harness-$(date +%Y%m%d-%H%M%S).log"
ISSUES="/tmp/cognos-issues-$(date +%Y%m%d-%H%M%S).txt"
PASS=0
FAIL=0
TOTAL=0
SEED=$RANDOM

log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }
pass() { PASS=$((PASS+1)); TOTAL=$((TOTAL+1)); log "  ✅ PASS: $1"; }
fail() { FAIL=$((FAIL+1)); TOTAL=$((TOTAL+1)); log "  ❌ FAIL: $1"; echo "$1: $2" >> "$ISSUES"; }

# Pick random element from array
pick() { local arr=("$@"); echo "${arr[$((RANDOM % ${#arr[@]}))]}" ; }

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

# Run inline test, expect non-zero exit (error case)
run_inline_fail() {
    local name="$1" src="$2" input="$3" expect_in_err="$4" timeout_s="${5:-10}"
    local tmpfile=$(mktemp /tmp/cognos-test-XXXXXX.cog)
    echo "$src" > "$tmpfile"
    log "TEST: $name"
    output=$(echo -e "$input" | timeout "$timeout_s" $COGNOS run "$tmpfile" 2>&1)
    code=$?
    rm -f "$tmpfile"
    if [ $code -ne 0 ] && echo "$output" | grep -qF -- "$expect_in_err"; then
        pass "$name"
    else
        fail "$name" "Expected error with '$expect_in_err'. Exit=$code Output=$(echo "$output" | head -3)"
    fi
}

log "=========================================="
log "Cognos Robustness Test Harness (seed=$SEED)"
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
run_test "import-test" "World" examples/import-test.cog "" "Hello," 5

log ""
log "=== SECTION 2: Language Features (Core) ==="

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

run_inline "comment-first-line" 'flow main():
    # comment
    write(stdout, "PASS")' "" "PASS"

run_inline "comment-nested" 'flow main():
    if true:
        # nested
        write(stdout, "PASS")' "" "PASS"

run_inline "comment-between" 'flow main():
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

run_inline "parallel-basic" 'flow main():
    parallel:
        branch:
            a = 1
        branch:
            b = 2
    write(stdout, f"{a+b}")' "" "3"

run_inline "select-break" 'flow fast() -> String:
    return "x"
flow main():
    c = 0
    loop:
        c = c + 1
        select:
            branch:
                r = fast()
                break
            branch:
                r = fast()
                break
    write(stdout, f"c={c}")' "" "c=1"

run_inline "loop-max" 'flow main():
    c = 0
    loop max=5:
        c = c + 1
    write(stdout, f"{c}")' "" "5"

run_inline "default-params" 'flow greet(name: String, g: String = "Hello"):
    write(stdout, f"{g} {name}")
flow main():
    greet("World")
    greet("World", g="Hi")' "" "Hello World"

run_inline "string-methods" 'flow main():
    s = "  Hello World  "
    write(stdout, s.strip())
    write(stdout, s.strip().lower())
    write(stdout, s.strip().upper())
    write(stdout, "hello".replace("l", "L"))' "" "heLLo"

run_inline "list-ops" 'flow main():
    l = [1, 2, 3] + [4, 5]
    write(stdout, f"len={l.length}")
    write(stdout, f"has3={l.contains(3)}")
    write(stdout, l.join(","))' "" "1,2,3,4,5"

run_inline "map-ops" 'flow main():
    m = {"a": 1, "b": 2}
    m["c"] = 3
    write(stdout, f"len={m.length}")
    m2 = remove(m, "b")
    write(stdout, f"len2={m2.length}")' "" "len2=2"

run_inline "string-repeat" 'flow main():
    s = "ab" * 3
    write(stdout, s)' "" "ababab"

run_inline "negative-slice" 'flow main():
    write(stdout, "hello"[-2:])' "" "lo"

run_inline "mixed-arith" 'flow main():
    write(stdout, f"{1.5 + 2}")
    write(stdout, f"{0 - 5}")' "" "-5"

run_inline "for-kv" 'flow main():
    m = {"x": 1, "y": 2}
    for k, v in m:
        write(stdout, f"{k}={v}")' "" "x=1"

run_inline "type-def" 'type Point:
    x: Int
    y: Int
flow main():
    p = {"x": 10, "y": 20}
    write(stdout, f"{p.x},{p.y}")' "" "10,20"

run_inline "optional-field" 'type Cfg:
    name: String
    debug?: Bool
flow main():
    c = {"name": "test"}
    write(stdout, c.name)' "" "test"

run_inline "enum-type" 'type Level: "low" | "medium" | "high"
flow main():
    write(stdout, "PASS")' "" "PASS"

run_inline "try-catch-var" 'flow main():
    try:
        x = read(file("nonexistent.txt"))
    catch err:
        write(stdout, f"caught: {err}")' "" "caught:"

run_inline "save-load" 'flow main():
    save("/tmp/cognos-harness-test.json", {"key": "value", "num": 42})
    data = load("/tmp/cognos-harness-test.json")
    write(stdout, f"{data.key},{data.num}")' "" "value,42"

run_inline "fstring-expr" 'flow main():
    x = 5
    write(stdout, f"calc={x * 2 + 1}")
    write(stdout, f"bool={x > 3}")' "" "calc=11"

run_inline "multi-flow" 'flow add(a: Int, b: Int) -> Int:
    return a + b
flow mul(a: Int, b: Int) -> Int:
    return a * b
flow main():
    write(stdout, f"{add(3, 4)}")
    write(stdout, f"{mul(3, 4)}")' "" "12"

run_inline "kwargs" 'flow greet(name: String, loud: Bool = false) -> String:
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

run_inline "pass-stmt" 'flow empty():
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

run_inline "elif" 'flow main():
    x = 2
    if x == 1:
        write(stdout, "one")
    elif x == 2:
        write(stdout, "two")
    else:
        write(stdout, "other")' "" "two"

run_inline "break-continue" 'flow main():
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

run_inline "recursive" 'flow fib(n: Int) -> Int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
flow main():
    write(stdout, f"{fib(10)}")' "" "55"

run_inline "multiline" 'flow main():
    result = (
        1 + 2 +
        3 + 4
    )
    write(stdout, f"{result}")' "" "10"

run_inline "dot-access" 'flow main():
    m = {"name": "test", "value": 42}
    write(stdout, f"{m.name}:{m.value}")' "" "test:42"

run_inline "bool-cmp" 'flow main():
    write(stdout, f"{true == true}")
    write(stdout, f"{none == none}")' "" "true"

run_inline "deep-nest" 'flow main():
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

log ""
log "=== SECTION 3: Error Cases ==="

run_inline_fail "err-undefined" 'flow main():
    write(stdout, xyz)' "" "undefined variable"

run_inline_fail "err-type-mismatch" 'flow main():
    x = "hello" + 5' "" "cannot"

run_inline_fail "err-missing-arg" 'flow greet(name: String):
    pass
flow main():
    greet()' "" ""

run_inline "err-div-zero" 'flow main():
    try:
        x = 1 / 0
    catch err:
        write(stdout, "caught")' "" "caught"

run_inline "err-file-missing" 'flow main():
    try:
        x = read(file("/tmp/nonexistent_cognos_xyz.txt"))
    catch err:
        write(stdout, "caught")' "" "caught"

run_inline "err-empty-list" 'flow main():
    l = []
    write(stdout, f"len={l.length}")
    write(stdout, l.join(","))
    write(stdout, "PASS")' "" "PASS"

log ""
log "=== SECTION 4: Randomized LLM Tests (DeepSeek) ==="

# Randomized simple prompts
SIMPLE_PROMPTS=(
    "Say exactly: MARKER_ABC123"
    "Output only the word: MARKER_ABC123"
    "Reply with just: MARKER_ABC123"
    "Your response must be exactly: MARKER_ABC123"
    "Print: MARKER_ABC123"
)
prompt=$(pick "${SIMPLE_PROMPTS[@]}")
log "  prompt: $prompt"
run_inline "ds-simple-rand" "flow main():
    r = think(\"$prompt\", model=\"deepseek-chat\", system=\"Output exactly what is asked. Nothing else.\")
    write(stdout, r.content)" "" "MARKER_ABC123" "" 15

# Randomized tool use prompts
TOOL_PROMPTS=(
    "What is the current time?"
    "Tell me what time it is right now"
    "Can you check the time?"
    "I need to know the current time"
    "Get the time please"
)
prompt=$(pick "${TOOL_PROMPTS[@]}")
log "  prompt: $prompt"
run_inline "ds-tool-rand" 'flow get_time() -> String:
    "Get the current date and time"
    return "2026-02-12 20:00:00"
flow main():
    r = think("'"$prompt"'", model="deepseek-chat", tools=["get_time"], system="Use tools to answer. Always use get_time for time questions.")
    if r.has_tool_calls:
        write(stdout, "TOOL_USED")
    else:
        write(stdout, "NO_TOOL")' "" "TOOL_USED" "" 15

# Randomized format extraction
FORMAT_PROMPTS=(
    "Rate this code: def add(a,b): return a+b"
    "Review: function multiply(x, y) { return x * y }"
    "Evaluate: const greet = name => Hello + name"
    "Score this code: fn main() { println(hello) }"
    "Assess: SELECT * FROM users WHERE id = 1"
)
prompt=$(pick "${FORMAT_PROMPTS[@]}")
log "  prompt: $prompt"
run_inline "ds-format-rand" 'type Review:
    score: Int
    summary: String
flow main():
    r = think("'"$prompt"'", format="Review", model="deepseek-chat", system="You are a code reviewer. Always give a score 1-10.")
    s = r["score"]
    write(stdout, f"score={s}")' "" "score=" "" 15

# Randomized system personas
PERSONAS=(
    "You are a pirate. Always say arr."
    "You are a robot. Always say BEEP."
    "You are a cat. Always say meow."
    "You are French. Always say bonjour."
    "You are a wizard. Always say abracadabra."
)
persona=$(pick "${PERSONAS[@]}")
MARKERS=("arr" "BEEP" "meow" "bonjour" "abracadabra")
# find which marker matches
for i in "${!PERSONAS[@]}"; do
    if [ "${PERSONAS[$i]}" = "$persona" ]; then
        marker="${MARKERS[$i]}"
        break
    fi
done
log "  persona: $persona (expect: $marker)"
run_inline "ds-persona-rand" "flow main():
    r = think(\"Say hello in character\", model=\"deepseek-chat\", system=\"$persona Respond in character. Include your signature word.\")
    c = r.content.lower()
    if c.contains(\"$(echo $marker | tr '[:upper:]' '[:lower:]')\"):
        write(stdout, \"PASS\")
    else:
        write(stdout, f\"FAIL: {r.content}\")" "" "PASS" "" 15

# Multi-turn with context
TOPICS=("Rust programming" "machine learning" "space exploration" "cooking pasta" "chess strategy")
topic=$(pick "${TOPICS[@]}")
log "  topic: $topic"
run_inline "ds-multi-turn" 'flow main():
    h = []
    h = h + ["User: Tell me one fact about '"$topic"'"]
    r1 = think(h.join("\n"), model="deepseek-chat", system="Be concise. One sentence max.")
    h = h + [f"Assistant: {r1.content}"]
    h = h + ["User: Can you elaborate on that?"]
    r2 = think(h.join("\n"), model="deepseek-chat", system="Be concise. Two sentences max.")
    if r2.content.length > 10:
        write(stdout, "PASS")
    else:
        write(stdout, f"FAIL: too short: {r2.content}")' "" "PASS" "" 20

log ""
log "=== SECTION 5: Randomized Shell Tool Agent ==="

SHELL_TASKS=(
    "How many .rs files are in the src directory?\nexit"
    "What is today's date?\nexit"
    "Show me the first 3 lines of Cargo.toml\nexit"
    "How much disk space is the current directory using?\nexit"
    "What is my username?\nexit"
)
task=$(pick "${SHELL_TASKS[@]}")
log "  task: $(echo $task | head -1)"
run_test "agent-shell-rand" "$task" examples/general-assistant.cog "--allow-shell" "Assistant ready" 30

# Tool agent with randomized queries
TOOL_TASKS=(
    "What is the weather in Tokyo?\nquit"
    "What is the weather in Paris?\nquit"
    "What time is it?\nquit"
    "What is the weather in Berlin?\nquit"
    "Tell me the time and weather in Amsterdam\nquit"
)
task=$(pick "${TOOL_TASKS[@]}")
log "  task: $(echo $task | head -1)"
run_test "agent-tool-rand" "$task" examples/tool-agent.cog "--allow-shell" "" 30

log ""
log "=== SECTION 6: Fuzz & Stress Tests ==="

# Random string inputs — test parser doesn't crash
run_inline "fuzz-unicode" 'flow main():
    s = "héllo wörld 日本語 🎉"
    write(stdout, f"len={s.length}")
    write(stdout, s)' "" "héllo"

run_inline "fuzz-empty-ops" 'flow main():
    s = ""
    l = []
    m = {}
    write(stdout, f"{s.length},{l.length},{m.length}")
    write(stdout, s + "x")
    write(stdout, (l + [1]).join(","))' "" "0,0,0"

run_inline "fuzz-long-string" 'flow main():
    s = "a" * 10000
    write(stdout, f"len={s.length}")' "" "len=10000"

run_inline "fuzz-nested-maps" 'flow main():
    m = {"a": {"b": {"c": {"d": "deep"}}}}
    write(stdout, m["a"]["b"]["c"]["d"])' "" "deep"

run_inline "fuzz-many-args" 'flow sum5(a: Int, b: Int, c: Int, d: Int, e: Int) -> Int:
    return a + b + c + d + e
flow main():
    write(stdout, f"{sum5(1, 2, 3, 4, 5)}")' "" "15"

run_inline "fuzz-rapid-async" 'flow task(n: Int) -> Int:
    return n * 2
flow main():
    handles = []
    handles = handles + [async task(1)]
    handles = handles + [async task(2)]
    handles = handles + [async task(3)]
    r1 = await(handles[0])
    r2 = await(handles[1])
    r3 = await(handles[2])
    write(stdout, f"{r1},{r2},{r3}")' "" "2,4,6"

# Random computation test — verify arithmetic
A=$((RANDOM % 100))
B=$((RANDOM % 100))
EXPECTED=$((A + B))
run_inline "fuzz-arith-$A+$B" "flow main():
    write(stdout, f\"{$A + $B}\")" "" "$EXPECTED"

A=$((RANDOM % 50 + 1))
B=$((RANDOM % 50 + 1))
EXPECTED=$((A * B))
run_inline "fuzz-arith-${A}x${B}" "flow main():
    write(stdout, f\"{$A * $B}\")" "" "$EXPECTED"

# Random list generation and verification
N=$((RANDOM % 20 + 5))
run_inline "fuzz-list-$N" "flow main():
    l = []
    loop max=$N:
        l = l + [l.length]
    write(stdout, f\"len={l.length}\")" "" "len=$N"

# String slice fuzzing
LEN=$((RANDOM % 10 + 5))
START=$((RANDOM % LEN))
END=$((START + RANDOM % (LEN - START) + 1))
STR=$(head -c $LEN /dev/urandom | tr -dc 'a-z' | head -c $LEN)
run_inline "fuzz-slice" "flow main():
    s = \"$STR\"
    r = s[$START:$END]
    write(stdout, f\"len={r.length}\")" "" "len="

log ""
log "=== SECTION 7: Memory System ==="

MEMDB="/tmp/cognos-test-memory-$$.db"
rm -f "$MEMDB"

# Basic remember + recall
run_inline "mem-remember-recall" 'flow main():
    remember("The sky is blue")
    remember("Cognos is written in Rust")
    remember("P11 means lean core runtime")
    results = recall("what color is the sky", limit=3)
    for r in results:
        write(stdout, r)' "" "sky is blue" "--memory --memory-db $MEMDB" 15

# Recall with keyword boost (hybrid search)
run_inline "mem-hybrid-keyword" 'flow main():
    remember("BUG-42: parser fails on empty blocks")
    remember("The weather is nice today")
    remember("Rust is a systems language")
    results = recall("BUG-42", limit=3)
    write(stdout, results[0])' "" "BUG-42" "--memory --memory-db $MEMDB" 15

# Dedup: storing same fact twice should skip
MEMDB_DEDUP="/tmp/cognos-test-dedup-$$.db"
rm -f "$MEMDB_DEDUP"
run_inline "mem-dedup" 'flow main():
    remember("duplicate fact here")
    remember("duplicate fact here")
    results = recall("duplicate", limit=5)
    write(stdout, f"count={results.length}")' "" "count=1" "--memory --memory-db $MEMDB_DEDUP" 15
rm -f "$MEMDB_DEDUP"

# Forget
run_inline "mem-forget" 'flow main():
    remember("temporary note about testing")
    n = forget("temporary note")
    write(stdout, f"forgot={n}")
    results = recall("temporary note", limit=5)
    write(stdout, f"remaining={results.length}")' "" "forgot=1" "--memory --memory-db $MEMDB" 15

# Recall with no matches returns empty list
run_inline "mem-recall-empty" 'flow main():
    results = recall("xyzzy nonexistent query", limit=3)
    write(stdout, f"len={results.length}")' "" "len=" "--memory --memory-db $MEMDB" 15

# Namespace isolation
MEMDB2="/tmp/cognos-test-memory2-$$.db"
rm -f "$MEMDB2"
run_inline "mem-namespace" 'flow main():
    remember("agent-specific fact")
    results = recall("agent-specific", limit=3)
    write(stdout, f"found={results.length}")' "" "found=1" "--memory --memory-db $MEMDB2 --memory-ns test-agent" 15

rm -f "$MEMDB" "$MEMDB2"

log ""
log "=== SECTION 8: Chat Sessions with Varied Inputs ==="

# Randomized multi-turn chat
GREETINGS=("Hello" "Hi there" "Hey" "Greetings" "Good day")
QUESTIONS=("What is 2+2?" "Name a color" "Say a number" "Name a fruit" "What is 1+1?")
FAREWELLS=("quit" "quit" "quit")
g=$(pick "${GREETINGS[@]}")
q=$(pick "${QUESTIONS[@]}")
run_test "chat-rand" "$g\n$q\nquit" examples/chat.cog "" "" 25

# General assistant with varied exit commands
EXITS=("exit" "quit" "exit")
e=$(pick "${EXITS[@]}")
run_test "assist-greet-rand" "Hello, how are you?\n$e" examples/general-assistant.cog "--allow-shell" "Assistant ready" 20

log ""
log "=========================================="
RESULT_LINE="RESULTS: $PASS passed, $FAIL failed, $TOTAL total (seed=$SEED)"
log "$RESULT_LINE"
log "=========================================="
log "Log: $LOG"
if [ $FAIL -gt 0 ]; then
    log "Issues: $ISSUES"
    cat "$ISSUES" | tee -a "$LOG"
fi

# Write stable files for heartbeat
echo "$PASS/$TOTAL" > /tmp/cognos-test-result.txt
cp "$ISSUES" /tmp/cognos-latest-issues.txt 2>/dev/null || echo "" > /tmp/cognos-latest-issues.txt
