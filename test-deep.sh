#!/bin/bash
# Cognos Deep/Adversarial Test Suite
# Tests edge cases, stress, and failure modes — no LLM calls
set -o pipefail
cd /home/reza/clawd/neocognos/cognos-lang
COGNOS="./target/release/cognos"
LOG="/tmp/cognos-test-deep-$(date +%Y%m%d-%H%M%S).log"
ISSUES="/tmp/cognos-deep-issues-$(date +%Y%m%d-%H%M%S).txt"
PASS=0
FAIL=0
TOTAL=0
TMPDIR_DEEP=$(mktemp -d /tmp/cognos-deep-XXXXXX)

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

run_inline_fail() {
    local name="$1" src="$2" input="$3" expect_in_err="$4" timeout_s="${5:-10}"
    local tmpfile=$(mktemp /tmp/cognos-test-XXXXXX.cog)
    echo "$src" > "$tmpfile"
    log "TEST: $name"
    output=$(echo -e "$input" | timeout "$timeout_s" $COGNOS run "$tmpfile" 2>&1)
    code=$?
    rm -f "$tmpfile"
    if [ $code -eq 124 ]; then
        fail "$name" "TIMEOUT after ${timeout_s}s"
    elif [ $code -ne 0 ]; then
        if [ -z "$expect_in_err" ] || echo "$output" | grep -qiF -- "$expect_in_err"; then
            pass "$name"
        else
            fail "$name" "Non-zero exit but expected '$expect_in_err' not found. Exit=$code Output=$(echo "$output" | head -3)"
        fi
    else
        fail "$name" "Expected failure but got exit=0. Output=$(echo "$output" | head -3)"
    fi
}

# Run inline, accept either success or specific error (for "should not crash" tests)
run_inline_nocrash() {
    local name="$1" src="$2" input="$3" timeout_s="${4:-10}"
    local tmpfile=$(mktemp /tmp/cognos-test-XXXXXX.cog)
    echo "$src" > "$tmpfile"
    log "TEST: $name"
    output=$(echo -e "$input" | timeout "$timeout_s" $COGNOS run "$tmpfile" 2>&1)
    code=$?
    rm -f "$tmpfile"
    if [ $code -eq 124 ]; then
        fail "$name" "TIMEOUT after ${timeout_s}s"
    elif [ $code -ge 128 ]; then
        fail "$name" "CRASHED with signal $((code-128)). Output=$(echo "$output" | head -3)"
    else
        pass "$name"
    fi
}

log "=========================================="
log "Cognos Deep/Adversarial Test Suite"
log "=========================================="

log "Building release..."
PATH="$HOME/.cargo/bin:$PATH" cargo build --release 2>&1 | tail -1 | tee -a "$LOG"
if [ $? -ne 0 ]; then
    log "BUILD FAILED — aborting"
    exit 1
fi

log ""
log "=== SECTION 1: Concurrency Edge Cases ==="

run_inline "conc-parallel-overlap" 'flow main():
    x = 0
    parallel:
        branch:
            x = x + 1
        branch:
            x = x + 10
    write(stdout, f"x={x}")' "" "x="

run_inline "conc-select-fast-slow" 'flow fast() -> String:
    return "fast"
flow slow() -> String:
    x = 0
    loop max=100:
        x = x + 1
    return "slow"
flow main():
    select:
        branch:
            r = fast()
        branch:
            r = slow()
    write(stdout, f"r={r}")' "" "r="

run_inline "conc-parallel-in-loop" 'flow main():
    total = 0
    loop max=3:
        parallel:
            branch:
                a = 1
            branch:
                b = 2
        total = total + a + b
    write(stdout, f"total={total}")' "" "total=9"

run_inline "conc-select-in-parallel" 'flow quick() -> String:
    return "q"
flow main():
    parallel:
        branch:
            select:
                branch:
                    a = quick()
                branch:
                    a = quick()
        branch:
            b = "done"
    write(stdout, f"{a},{b}")' "" "q,done"

# 20 async handles awaited in reverse
run_inline "conc-20-async-reverse" 'flow task(n: Int) -> Int:
    return n * 2
flow main():
    h0 = async task(0)
    h1 = async task(1)
    h2 = async task(2)
    h3 = async task(3)
    h4 = async task(4)
    h5 = async task(5)
    h6 = async task(6)
    h7 = async task(7)
    h8 = async task(8)
    h9 = async task(9)
    h10 = async task(10)
    h11 = async task(11)
    h12 = async task(12)
    h13 = async task(13)
    h14 = async task(14)
    h15 = async task(15)
    h16 = async task(16)
    h17 = async task(17)
    h18 = async task(18)
    h19 = async task(19)
    r19 = await h19
    r18 = await h18
    r17 = await h17
    r16 = await h16
    r15 = await h15
    r14 = await h14
    r13 = await h13
    r12 = await h12
    r11 = await h11
    r10 = await h10
    r9 = await h9
    r8 = await h8
    r7 = await h7
    r6 = await h6
    r5 = await h5
    r4 = await h4
    r3 = await h3
    r2 = await h2
    r1 = await h1
    r0 = await h0
    write(stdout, f"{r0},{r19}")' "" "0,38" "" 15

run_inline_nocrash "conc-cancel-then-await" 'flow task() -> String:
    return "done"
flow main():
    h = async task()
    cancel(h)
    try:
        r = await h
    catch err:
        write(stdout, f"caught: {err}")
    write(stdout, "survived")'

run_inline_nocrash "conc-double-await" 'flow task() -> Int:
    return 42
flow main():
    h = async task()
    r1 = await h
    try:
        r2 = await h
    catch err:
        write(stdout, f"caught: {err}")
    write(stdout, f"r1={r1}")'

log ""
log "=== SECTION 2: Memory & Resource Stress ==="

run_inline "stress-deep-recursion-200" 'flow countdown(n: Int) -> Int:
    if n <= 0:
        return 0
    return countdown(n - 1)
flow main():
    r = countdown(200)
    write(stdout, f"done={r}")' "" "done=0" "" 15

run_inline "stress-deep-recursion-500" 'flow countdown(n: Int) -> Int:
    if n <= 0:
        return 0
    return countdown(n - 1)
flow main():
    r = countdown(500)
    write(stdout, f"done={r}")' "" "done=0" "" 15

run_inline "stress-large-map-500" 'flow main():
    m = {}
    loop max=500:
        k = f"key_{m.length}"
        m[k] = m.length
    write(stdout, f"len={m.length}")' "" "len=500" "" 15

run_inline "stress-large-list-1000" 'flow main():
    l = []
    loop max=1000:
        l = l + [l.length]
    write(stdout, f"len={l.length}")' "" "len=1000" "" 15

# Huge f-string with 50+ interpolations
FSTR_SRC='flow main():
    x = 1
    s = f"{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}{x}"
    write(stdout, f"len={s.length}")'
run_inline "stress-fstring-50-interp" "$FSTR_SRC" "" "len=50"

run_inline "stress-list-grow-100" 'flow main():
    l = []
    loop max=100:
        l = l + [l.length]
    write(stdout, f"len={l.length}")' "" "len=100"

run_inline "stress-string-concat-100" 'flow main():
    s = ""
    loop max=100:
        s = s + "a"
    write(stdout, f"len={s.length}")' "" "len=100"

log ""
log "=== SECTION 3: Parser Adversarial ==="

run_inline "parse-deep-parens" 'flow main():
    x = ((((((1+2))))))
    write(stdout, f"{x}")' "" "3"

run_inline "parse-deep-indent" 'flow main():
    if true:
        if true:
            if true:
                if true:
                    if true:
                        if true:
                            if true:
                                if true:
                                    write(stdout, "DEEP8")' "" "DEEP8"

run_inline "parse-chained-pass" 'flow a():
    pass
flow b():
    pass
flow c():
    pass
flow main():
    a()
    b()
    c()
    write(stdout, "PASS")' "" "PASS"

run_inline "parse-blank-lines" 'flow main():

    x = 1


    y = 2



    write(stdout, f"{x+y}")' "" "3"

run_inline "parse-comments-everywhere" '# top comment
flow helper(
    # comment in params area
    x: Int
) -> Int:
    # body comment
    return x + 1 # inline comment
# between flows
flow main():
    # before code
    r = helper(5)
    # after call
    write(stdout, f"{r}")
# trailing comment' "" "6"

# Trailing whitespace
TRAIL_SRC=$(printf 'flow main():   \n    x = 1   \n    write(stdout, f"{x}")   \n')
run_inline "parse-trailing-whitespace" "$TRAIL_SRC" "" "1"

# Windows line endings
WIN_SRC=$(printf 'flow main():\r\n    x = 42\r\n    write(stdout, f"{x}")\r\n')
run_inline_nocrash "parse-windows-crlf" "$WIN_SRC"

# Tab characters
TAB_SRC=$(printf 'flow main():\n\tx = 1\n\twrite(stdout, f"{x}")\n')
run_inline_nocrash "parse-tab-indent" "$TAB_SRC"

log ""
log "=== SECTION 4: LLM Failure Modes (no actual LLM) ==="

run_inline "llm-unreachable-model" 'flow main():
    try:
        r = think("hello", model="nonexistent-model-xyz")
    catch err:
        write(stdout, f"caught: {err}")
    write(stdout, "survived")' "" "survived" "" 15

run_inline "llm-type-mismatch-format" 'type Review:
    score: Int
    summary: String
flow main():
    m = {"score": "not_an_int", "summary": 123}
    write(stdout, f"score={m.score}")' "" "score="

log ""
log "=== SECTION 5: Import Edge Cases ==="

# Circular import: A imports B, B imports A
cat > "$TMPDIR_DEEP/circ_a.cog" << 'EOF'
import "circ_b.cog"
flow hello_a() -> String:
    return "a"
flow main():
    write(stdout, hello_a())
EOF
cat > "$TMPDIR_DEEP/circ_b.cog" << 'EOF'
import "circ_a.cog"
flow hello_b() -> String:
    return "b"
EOF
log "TEST: import-circular"
output=$(timeout 10 $COGNOS run "$TMPDIR_DEEP/circ_a.cog" 2>&1)
code=$?
if [ $code -eq 124 ]; then
    fail "import-circular" "TIMEOUT — infinite loop on circular import"
elif [ $code -ge 128 ]; then
    fail "import-circular" "CRASHED with signal $((code-128))"
elif [ $code -ne 0 ]; then
    pass "import-circular (detected error)"
else
    # If it works, that's also fine (means it deduplicates)
    pass "import-circular (handled gracefully)"
fi

# Diamond import: A→B, A→C, B→D, C→D
cat > "$TMPDIR_DEEP/diamond_d.cog" << 'EOF'
flow shared() -> String:
    return "shared"
EOF
cat > "$TMPDIR_DEEP/diamond_b.cog" << 'EOF'
import "diamond_d.cog"
flow from_b() -> String:
    return f"b+{shared()}"
EOF
cat > "$TMPDIR_DEEP/diamond_c.cog" << 'EOF'
import "diamond_d.cog"
flow from_c() -> String:
    return f"c+{shared()}"
EOF
cat > "$TMPDIR_DEEP/diamond_a.cog" << 'EOF'
import "diamond_b.cog"
import "diamond_c.cog"
flow main():
    write(stdout, f"{from_b()},{from_c()}")
EOF
run_test "import-diamond" "" "$TMPDIR_DEEP/diamond_a.cog" "" "b+shared,c+shared"

# Import file with syntax error
cat > "$TMPDIR_DEEP/bad_syntax.cog" << 'EOF'
flow broken(
    this is not valid syntax !!!
EOF
cat > "$TMPDIR_DEEP/import_bad.cog" << 'EOF'
import "bad_syntax.cog"
flow main():
    write(stdout, "should not reach")
EOF
log "TEST: import-syntax-error"
output=$(timeout 10 $COGNOS run "$TMPDIR_DEEP/import_bad.cog" 2>&1)
code=$?
if [ $code -ne 0 ]; then
    pass "import-syntax-error"
else
    fail "import-syntax-error" "Expected error for bad import, got exit=0"
fi

# Flow name collision
cat > "$TMPDIR_DEEP/collision_lib1.cog" << 'EOF'
flow helper() -> String:
    return "lib1"
EOF
cat > "$TMPDIR_DEEP/collision_lib2.cog" << 'EOF'
flow helper() -> String:
    return "lib2"
EOF
cat > "$TMPDIR_DEEP/collision_main.cog" << 'EOF'
import "collision_lib1.cog"
import "collision_lib2.cog"
flow main():
    write(stdout, helper())
EOF
run_inline_nocrash "import-name-collision" "$(cat $TMPDIR_DEEP/collision_main.cog)"
# Also test via file directly
log "TEST: import-name-collision-file"
output=$(timeout 10 $COGNOS run "$TMPDIR_DEEP/collision_main.cog" 2>&1)
code=$?
if [ $code -ge 128 ]; then
    fail "import-name-collision-file" "CRASHED"
else
    pass "import-name-collision-file"
fi

# Import nonexistent
cat > "$TMPDIR_DEEP/import_missing.cog" << 'EOF'
import "this_file_does_not_exist.cog"
flow main():
    write(stdout, "should not reach")
EOF
log "TEST: import-nonexistent"
output=$(timeout 10 $COGNOS run "$TMPDIR_DEEP/import_missing.cog" 2>&1)
code=$?
if [ $code -ne 0 ]; then
    pass "import-nonexistent"
else
    fail "import-nonexistent" "Expected error, got exit=0"
fi

log ""
log "=== SECTION 6: State & Session ==="

run_inline "state-save-all-types" 'flow main():
    data = {
        "str": "hello",
        "int": 42,
        "float": 3.14,
        "bool_t": true,
        "bool_f": false,
        "none_v": none,
        "list": [1, "two", 3.0],
        "map": {"nested": "value"},
        "deep": {"a": {"b": {"c": 1}}}
    }
    save("/tmp/cognos-deep-all-types.json", data)
    d = load("/tmp/cognos-deep-all-types.json")
    write(stdout, f"str={d.str}")
    write(stdout, f"int={d.int}")
    write(stdout, f"bool={d.bool_t}")
    write(stdout, f"list_len={d.list.length}")
    write(stdout, f"nested={d.map.nested}")
    write(stdout, f"deep={d.deep.a.b.c}")' "" "deep=" "" 10

run_inline "state-load-nonexistent" 'flow main():
    try:
        d = load("/tmp/cognos-this-file-absolutely-does-not-exist.json")
    catch err:
        write(stdout, f"caught: {err}")
    write(stdout, "survived")' "" "survived"

run_inline "state-save-readonly" 'flow main():
    try:
        save("/proc/cognos-impossible.json", {"x": 1})
    catch err:
        write(stdout, f"caught: {err}")
    write(stdout, "survived")' "" "survived"

log ""
log "=== SECTION 7: Type System ==="

run_inline "type-many-fields" 'type BigType:
    f1: String
    f2: String
    f3: String
    f4: Int
    f5: Int
    f6: Int
    f7: Bool
    f8: Bool
    f9: Float
    f10: Float
    f11: String
flow main():
    obj = {"f1": "a", "f2": "b", "f3": "c", "f4": 1, "f5": 2, "f6": 3, "f7": true, "f8": false, "f9": 1.1, "f10": 2.2, "f11": "k"}
    write(stdout, f"{obj.f1},{obj.f11}")' "" "a,k"

run_inline "type-nested-types" 'type Address:
    city: String
    zip: String
type Person:
    name: String
    addr: Address
flow main():
    p = {"name": "Alice", "addr": {"city": "Amsterdam", "zip": "1000"}}
    write(stdout, f"{p.name} in {p.addr.city}")' "" "Alice in Amsterdam"

run_inline "type-optional-present" 'type Config:
    name: String
    debug?: Bool
flow main():
    c = {"name": "test", "debug": true}
    write(stdout, f"{c.name},{c.debug}")' "" "test,true"

run_inline "type-optional-absent" 'type Config:
    name: String
    debug?: Bool
flow main():
    c = {"name": "test"}
    write(stdout, c.name)' "" "test"

run_inline "type-enum-valid" 'type Level: "low" | "medium" | "high"
flow check(l: Level):
    write(stdout, f"level={l}")
flow main():
    check("medium")' "" "level=medium"

run_inline_nocrash "type-enum-invalid" 'type Level: "low" | "medium" | "high"
flow check(l: Level):
    write(stdout, f"level={l}")
flow main():
    check("invalid")'

run_inline "type-generic-list" 'flow sum_list(items: List[Int]) -> Int:
    total = 0
    for x in items:
        total = total + x
    return total
flow main():
    r = sum_list([1, 2, 3])
    write(stdout, f"{r}")' "" "6"

log ""
log "=========================================="
RESULT_LINE="RESULTS: $PASS passed, $FAIL failed, $TOTAL total"
log "$RESULT_LINE"
log "=========================================="
log "Log: $LOG"
if [ $FAIL -gt 0 ]; then
    log "Issues: $ISSUES"
    cat "$ISSUES" | tee -a "$LOG"
fi

# Cleanup
rm -rf "$TMPDIR_DEEP"
rm -f /tmp/cognos-deep-all-types.json
