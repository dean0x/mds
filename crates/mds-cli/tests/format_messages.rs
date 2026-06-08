//! CLI integration tests for `mds build --format messages`.
//!
//! Coverage:
//! - AC-2.1: `--format messages -o -` → valid pretty-printed JSON array
//! - AC-2.2: `--format messages -o out.json` → file written with valid JSON
//! - AC-2.3: default (no --format, `-o -`) → plain text, unchanged
//! - AC-2.4: `--format markdown -o -` → identical to default
//! - AC-2.5: `--format xml` (invalid) → non-zero exit, error lists valid values
//! - AC-2.6: template with NO @message blocks + `--format messages` → non-zero exit

mod common;
use common::{fixture, mds_bin};

// ── AC-2.1: --format messages → valid JSON array on stdout ───────────────────

#[test]
fn format_messages_to_stdout_is_valid_json_array() {
    let output = mds_bin()
        .args([
            "build",
            fixture("messages.mds").to_str().unwrap(),
            "--format",
            "messages",
            "-o",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "build --format messages should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Must parse as a JSON array.
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON");
    assert!(
        parsed.is_array(),
        "output must be a JSON array; got: {stdout}"
    );
    let arr = parsed.as_array().unwrap();
    assert_eq!(
        arr.len(),
        2,
        "expected 2 messages (system + user); got: {arr:#?}"
    );

    // Verify first message structure.
    assert_eq!(arr[0]["role"].as_str().unwrap(), "system");
    assert!(
        arr[0]["content"].as_str().unwrap().contains("helpful"),
        "system message content should mention 'helpful'; got: {:?}",
        arr[0]["content"]
    );

    // Verify second message structure.
    assert_eq!(arr[1]["role"].as_str().unwrap(), "user");
    assert_eq!(arr[1]["content"].as_str().unwrap(), "Hello!");

    // Must be pretty-printed (contains newlines/indentation).
    assert!(
        stdout.contains('\n'),
        "output should be pretty-printed (contain newlines); got: {stdout:?}"
    );
}

// ── AC-2.2: --format messages -o file → file written with valid JSON ─────────

#[test]
fn format_messages_to_file_is_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("out.json");

    let output = mds_bin()
        .args([
            "build",
            fixture("messages.mds").to_str().unwrap(),
            "--format",
            "messages",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "build --format messages -o file should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(out_path.exists(), "output file should be created");

    let content = std::fs::read_to_string(&out_path).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("file contents must be valid JSON");
    assert!(parsed.is_array(), "file must contain a JSON array");
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2, "expected 2 messages; got: {arr:#?}");
}

// ── AC-2.3: default (no --format) → plain text output ────────────────────────

#[test]
fn default_format_produces_plain_text() {
    let output = mds_bin()
        .args([
            "build",
            fixture("messages.mds").to_str().unwrap(),
            "-o",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "default build should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Must not be a JSON array.
    assert!(
        !stdout.trim_start().starts_with('['),
        "default output must not be a JSON array; got: {stdout:?}"
    );
    // Body content should be present (text mode renders @message body inline).
    assert!(
        stdout.contains("helpful") || stdout.contains("Hello"),
        "text-mode output should contain message bodies; got: {stdout:?}"
    );
}

// ── AC-2.4: --format markdown → identical to default ─────────────────────────

#[test]
fn format_markdown_produces_same_as_default() {
    let default_output = mds_bin()
        .args([
            "build",
            fixture("messages.mds").to_str().unwrap(),
            "-o",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    let markdown_output = mds_bin()
        .args([
            "build",
            fixture("messages.mds").to_str().unwrap(),
            "--format",
            "markdown",
            "-o",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        default_output.status.success(),
        "default build should succeed"
    );
    assert!(
        markdown_output.status.success(),
        "build --format markdown should succeed; stderr: {}",
        String::from_utf8_lossy(&markdown_output.stderr)
    );

    let default_stdout = String::from_utf8(default_output.stdout).unwrap();
    let markdown_stdout = String::from_utf8(markdown_output.stdout).unwrap();

    assert_eq!(
        default_stdout, markdown_stdout,
        "--format markdown must produce identical output to the default"
    );
}

// ── AC-2.5: --format xml (invalid) → non-zero exit ───────────────────────────

#[test]
fn invalid_format_value_exits_nonzero_with_error() {
    let output = mds_bin()
        .args([
            "build",
            fixture("messages.mds").to_str().unwrap(),
            "--format",
            "xml",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "--format xml should exit non-zero"
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    // clap should list valid values in the error.
    assert!(
        stderr.contains("markdown") || stderr.contains("messages") || stderr.contains("invalid"),
        "error should list valid format values; got: {stderr}"
    );
}

// ── AC-2.6: no @message blocks + --format messages → non-zero exit ───────────

#[test]
fn format_messages_without_message_blocks_exits_nonzero() {
    let output = mds_bin()
        .args([
            "build",
            fixture("simple.mds").to_str().unwrap(),
            "--format",
            "messages",
            "-o",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "--format messages on a template with no @message blocks should fail"
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("@message") || stderr.contains("message") || stderr.contains("no "),
        "error should mention missing @message blocks; got: {stderr}"
    );
}

// ── AC-2.1 via stdin: --format messages from stdin → valid JSON ───────────────

#[test]
fn format_messages_from_stdin_produces_valid_json() {
    let mut child = mds_bin()
        .args(["build", "-", "--format", "messages", "-o", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"@message system:\nYou are helpful.\n@end\n@message user:\nHello!\n@end\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "--format messages from stdin should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdin messages output must be valid JSON");
    assert!(parsed.is_array(), "stdin output must be a JSON array");
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2, "expected 2 messages; got: {arr:#?}");
    assert_eq!(arr[0]["role"].as_str().unwrap(), "system");
    assert_eq!(arr[1]["role"].as_str().unwrap(), "user");
}

// ── I11: oversized file → non-zero exit with clear error message ──────────────

#[test]
fn format_messages_rejects_oversized_file() {
    // MAX_FILE_SIZE is 10 MiB. Write a file just over that limit.
    let dir = tempfile::tempdir().unwrap();
    let big_file = dir.path().join("big.mds");

    // Write a valid header plus enough padding to exceed 10 MiB.
    let header = b"@message system:\nYou are helpful.\n@end\n";
    let padding_size = 10 * 1024 * 1024 + 1 - header.len();
    let mut contents = header.to_vec();
    contents.extend(std::iter::repeat_n(b' ', padding_size));

    std::fs::write(&big_file, &contents).unwrap();

    let output = mds_bin()
        .args([
            "build",
            big_file.to_str().unwrap(),
            "--format",
            "messages",
            "-o",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "build --format messages should fail for a file exceeding MAX_FILE_SIZE; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("too large") || stderr.contains("max") || stderr.contains("bytes"),
        "error should mention file-size limit; got: {stderr}"
    );
}
