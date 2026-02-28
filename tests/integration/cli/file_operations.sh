#!/usr/bin/env bash
# EVIF CLI File Operations Tests (P0)
# Tests for: ls, cat, write, mkdir, rm, mv, cp, stat, touch, head, tail, tree

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"
source "$SCRIPT_DIR/../../lib/server_manager.sh"

# Test configuration
export EVIF_CLI="${EVIF_CLI:-cargo run -p evif-cli --}"

log_section "CLI File Operations Tests"

# Test: ls /
test_ls_root() {
    local output
    output=$($EVIF_CLI ls / 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "ls / should succeed"
    assert_output_contains "$output" "mem" "ls / should show mount points"
}

# Test: ls -l /test
test_ls_long_format() {
    local output
    output=$($EVIF_CLI ls -l / 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "ls -l / should succeed"
    # Long format should show size/date/permissions
    assert_output_contains "$output" "d" "ls -l should show directory marker"
}

# Test: ls -r /test
test_ls_recursive() {
    # Create nested structure first
    $EVIF_CLI mkdir -p /test_recursive/a/b/c 2>&1 >/dev/null || true
    $EVIF_CLI write /test_recursive/a/b/c/file.txt -c "test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI ls -r /test_recursive 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "ls -r should succeed"
    assert_output_contains "$output" "file.txt" "ls -r should show nested files"

    # Cleanup
    $EVIF_CLI rm -r /test_recursive 2>&1 >/dev/null || true
}

# Test: cat /test/file.txt
test_cat_file() {
    # Create test file
    $EVIF_CLI write /test_cat.txt -c "Hello EVIF" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI cat /test_cat.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "cat should succeed"
    assert_output_contains "$output" "Hello EVIF" "cat should show file content"

    # Cleanup
    $EVIF_CLI rm /test_cat.txt 2>&1 >/dev/null || true
}

# Test: write /test/new.txt -c "hi"
test_write_file() {
    local output
    output=$($EVIF_CLI write /test_write.txt -c "hi" 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "write should succeed"

    # Verify file was created
    output=$($EVIF_CLI cat /test_write.txt 2>&1)
    assert_output_contains "$output" "hi" "written content should match"

    # Cleanup
    $EVIF_CLI rm /test_write.txt 2>&1 >/dev/null || true
}

# Test: write /test/append.txt -c "a" -a
test_write_append() {
    # Create initial file
    $EVIF_CLI write /test_append.txt -c "a" 2>&1 >/dev/null || true

    # Append to file
    local output
    output=$($EVIF_CLI write /test_append.txt -c "b" -a 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "write append should succeed"

    # Verify content
    output=$($EVIF_CLI cat /test_append.txt 2>&1)
    assert_output_contains "$output" "ab" "appended content should be 'ab'"

    # Cleanup
    $EVIF_CLI rm /test_append.txt 2>&1 >/dev/null || true
}

# Test: mkdir /test/dir
test_mkdir() {
    local output
    output=$($EVIF_CLI mkdir /test_mkdir 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "mkdir should succeed"

    # Verify directory was created
    output=$($EVIF_CLI ls / 2>&1)
    assert_output_contains "$output" "test_mkdir" "directory should appear in listing"

    # Cleanup
    $EVIF_CLI rm -r /test_mkdir 2>&1 >/dev/null || true
}

# Test: mkdir -p /a/b/c
test_mkdir_recursive() {
    local output
    output=$($EVIF_CLI mkdir -p /test_mkdir_p/a/b/c 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "mkdir -p should succeed"

    # Verify all directories created
    output=$($EVIF_CLI ls /test_mkdir_p/a/b 2>&1)
    assert_output_contains "$output" "c" "nested directories should be created"

    # Cleanup
    $EVIF_CLI rm -r /test_mkdir_p 2>&1 >/dev/null || true
}

# Test: rm /test/file.txt
test_rm_file() {
    # Create test file
    $EVIF_CLI write /test_rm.txt -c "test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI rm /test_rm.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "rm should succeed"

    # Verify file was removed
    output=$($EVIF_CLI cat /test_rm.txt 2>&1) && exit_code=0 || exit_code=$?
    if [ $exit_code -eq 0 ]; then
        log_fail "rm should remove file" "file removed" "file still exists"
        return 1
    fi
}

# Test: rm -r /test/dir
test_rm_recursive() {
    # Create test directory with content
    $EVIF_CLI mkdir -p /test_rm_r/a/b 2>&1 >/dev/null || true
    $EVIF_CLI write /test_rm_r/a/b/file.txt -c "test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI rm -r /test_rm_r 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "rm -r should succeed"

    # Verify directory was removed
    output=$($EVIF_CLI ls /test_rm_r 2>&1) && exit_code=0 || exit_code=$?
    if [ $exit_code -eq 0 ]; then
        log_fail "rm -r should remove directory" "directory removed" "directory still exists"
        return 1
    fi
}

# Test: mv /a /b
test_mv_file() {
    # Create source file
    $EVIF_CLI write /test_mv_src.txt -c "move test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI mv /test_mv_src.txt /test_mv_dst.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "mv should succeed"

    # Verify file moved
    output=$($EVIF_CLI cat /test_mv_dst.txt 2>&1)
    assert_output_contains "$output" "move test" "moved file should have correct content"

    # Cleanup
    $EVIF_CLI rm /test_mv_dst.txt 2>&1 >/dev/null || true
}

# Test: cp /src /dst
test_cp_file() {
    # Create source file
    $EVIF_CLI write /test_cp_src.txt -c "copy test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI cp /test_cp_src.txt /test_cp_dst.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "cp should succeed"

    # Verify both files exist
    output=$($EVIF_CLI cat /test_cp_dst.txt 2>&1)
    assert_output_contains "$output" "copy test" "copied file should have correct content"

    # Cleanup
    $EVIF_CLI rm /test_cp_src.txt /test_cp_dst.txt 2>&1 >/dev/null || true
}

# Test: stat /test/file
test_stat_file() {
    # Create test file
    $EVIF_CLI write /test_stat.txt -c "stat test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI stat /test_stat.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "stat should succeed"
    # Stat should show size, mtime, mode
    assert_output_contains "$output" "size" "stat should show size"
    assert_output_contains "$output" "time" "stat should show time"

    # Cleanup
    $EVIF_CLI rm /test_stat.txt 2>&1 >/dev/null || true
}

# Test: touch /test/empty
test_touch_file() {
    local output
    output=$($EVIF_CLI touch /test_touch.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "touch should succeed"

    # Verify empty file created
    output=$($EVIF_CLI cat /test_touch.txt 2>&1)
    assert_exit_code 0 $? "touched file should exist"

    # Cleanup
    $EVIF_CLI rm /test_touch.txt 2>&1 >/dev/null || true
}

# Test: head -n 5 /test/file
test_head_lines() {
    # Create test file with multiple lines
    local content=""
    for i in {1..10}; do
        content="${content}Line $i\n"
    done
    $EVIF_CLI write /test_head.txt -c "$(echo -e "$content")" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI head -n 5 /test_head.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "head should succeed"
    assert_output_contains "$output" "Line 1" "head should show first lines"
    assert_output_contains "$output" "Line 5" "head should show first 5 lines"

    # Cleanup
    $EVIF_CLI rm /test_head.txt 2>&1 >/dev/null || true
}

# Test: tail -n 5 /test/file
test_tail_lines() {
    # Create test file with multiple lines
    local content=""
    for i in {1..10}; do
        content="${content}Line $i\n"
    done
    $EVIF_CLI write /test_tail.txt -c "$(echo -e "$content")" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI tail -n 5 /test_tail.txt 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "tail should succeed"
    assert_output_contains "$output" "Line 6" "tail should show last lines"
    assert_output_contains "$output" "Line 10" "tail should show last 5 lines"

    # Cleanup
    $EVIF_CLI rm /test_tail.txt 2>&1 >/dev/null || true
}

# Test: tree -d 2 /test
test_tree_view() {
    # Create nested structure
    $EVIF_CLI mkdir -p /test_tree/a/b/c 2>&1 >/dev/null || true
    $EVIF_CLI write /test_tree/a/file.txt -c "test" 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI tree -d 2 /test_tree 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "tree should succeed"
    assert_output_contains "$output" "a" "tree should show first level"
    assert_output_contains "$output" "b" "tree should show second level"

    # Cleanup
    $EVIF_CLI rm -r /test_tree 2>&1 >/dev/null || true
}

# Run all tests
main() {
    log_info "Starting CLI File Operations Tests..."

    run_test "ls /" test_ls_root
    run_test "ls -l /test" test_ls_long_format
    run_test "ls -r /test" test_ls_recursive
    run_test "cat /test/file.txt" test_cat_file
    run_test "write /test/new.txt -c 'hi'" test_write_file
    run_test "write /test/append.txt -c 'a' -a" test_write_append
    run_test "mkdir /test/dir" test_mkdir
    run_test "mkdir -p /a/b/c" test_mkdir_recursive
    run_test "rm /test/file.txt" test_rm_file
    run_test "rm -r /test/dir" test_rm_recursive
    run_test "mv /a /b" test_mv_file
    run_test "cp /src /dst" test_cp_file
    run_test "stat /test/file" test_stat_file
    run_test "touch /test/empty" test_touch_file
    run_test "head -n 5 /test/file" test_head_lines
    run_test "tail -n 5 /test/file" test_tail_lines
    run_test "tree -d 2 /test" test_tree_view

    print_test_summary
}

main "$@"
