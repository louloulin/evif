#!/usr/bin/env bash
# EVIF Test Assertions
# Assertion functions for test validation

set -euo pipefail

# Assert command exit code
assert_exit_code() {
    local expected="$1"
    local actual="$2"
    local message="${3:-Exit code check}"

    if [ "$expected" -eq "$actual" ]; then
        log_pass "$message (exit code: $actual)"
        return 0
    else
        log_fail "$message" "exit code $expected" "exit code $actual"
        return 1
    fi
}

# Assert HTTP status code
assert_http_status() {
    local expected="$1"
    local actual="$2"
    local message="${3:-HTTP status check}"

    if [ "$expected" -eq "$actual" ]; then
        log_pass "$message (status: $actual)"
        return 0
    else
        log_fail "$message" "status $expected" "status $actual"
        return 1
    fi
}

# Assert output contains string
assert_output_contains() {
    local haystack="$1"
    local needle="$2"
    local message="${3:-Output contains check}"

    if echo "$haystack" | grep -q "$needle"; then
        log_pass "$message (found: '$needle')"
        return 0
    else
        log_fail "$message" "contains '$needle'" "does not contain '$needle'"
        echo "  Output: $haystack"
        return 1
    fi
}

# Assert output does NOT contain string
assert_output_not_contains() {
    local haystack="$1"
    local needle="$2"
    local message="${3:-Output not contains check}"

    if ! echo "$haystack" | grep -q "$needle"; then
        log_pass "$message (not found: '$needle')"
        return 0
    else
        log_fail "$message" "does not contain '$needle'" "contains '$needle'"
        echo "  Output: $haystack"
        return 1
    fi
}

# Assert output equals expected
assert_output_equals() {
    local expected="$1"
    local actual="$2"
    local message="${3:-Output equals check}"

    if [ "$expected" = "$actual" ]; then
        log_pass "$message"
        return 0
    else
        log_fail "$message" "$expected" "$actual"
        return 1
    fi
}

# Assert JSON field exists
assert_json_field() {
    local json="$1"
    local field="$2"
    local message="${3:-JSON field check}"

    if echo "$json" | jq -e ".$field" >/dev/null 2>&1; then
        log_pass "$message (field: $field)"
        return 0
    else
        log_fail "$message" "field '$field' exists" "field '$field' missing"
        echo "  JSON: $json"
        return 1
    fi
}

# Assert JSON field value
assert_json_value() {
    local json="$1"
    local field="$2"
    local expected="$3"
    local message="${4:-JSON value check}"

    local actual
    actual=$(echo "$json" | jq -r ".$field")

    if [ "$expected" = "$actual" ]; then
        log_pass "$message ($field = $actual)"
        return 0
    else
        log_fail "$message" "$field = $expected" "$field = $actual"
        echo "  JSON: $json"
        return 1
    fi
}

# Assert file exists
assert_file_exists() {
    local path="$1"
    local message="${2:-File exists check}"

    if [ -f "$path" ]; then
        log_pass "$message (file: $path)"
        return 0
    else
        log_fail "$message" "file exists" "file not found"
        echo "  Path: $path"
        return 1
    fi
}

# Assert directory exists
assert_dir_exists() {
    local path="$1"
    local message="${2:-Directory exists check}"

    if [ -d "$path" ]; then
        log_pass "$message (dir: $path)"
        return 0
    else
        log_fail "$message" "directory exists" "directory not found"
        echo "  Path: $path"
        return 1
    fi
}

# Assert file content
assert_file_content() {
    local path="$1"
    local expected="$2"
    local message="${3:-File content check}"

    if [ ! -f "$path" ]; then
        log_fail "$message" "file exists" "file not found"
        return 1
    fi

    local actual
    actual=$(cat "$path")

    if [ "$expected" = "$actual" ]; then
        log_pass "$message"
        return 0
    else
        log_fail "$message" "$expected" "$actual"
        return 1
    fi
}

# Assert numeric comparison
assert_greater_than() {
    local actual="$1"
    local threshold="$2"
    local message="${3:-Greater than check}"

    if (( actual > threshold )); then
        log_pass "$message ($actual > $threshold)"
        return 0
    else
        log_fail "$message ""> $threshold" "$actual"
        return 1
    fi
}

assert_less_than() {
    local actual="$1"
    local threshold="$2"
    local message="${3:-Less than check}"

    if (( actual < threshold )); then
        log_pass "$message ($actual < $threshold)"
        return 0
    else
        log_fail "$message" "< $threshold" "$actual"
        return 1
    fi
}
