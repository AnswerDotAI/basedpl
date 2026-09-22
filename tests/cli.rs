use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn native_expression_and_diagnostic() {
    for (code, expected) in [
        ("2×3+4", "14\n"),
        ("(2×3)+4", "10\n"),
        ("10-3-2", "9\n"),
        ("¯2+5", "3\n"),
        ("1x 0ₓ ¯2x 9223372036854775808ₓ", "1ₓ 0ₓ ¯2ₓ 9223372036854775808ₓ\n"),
        ("⊂4ₓ ⋄ ⊂⊂4ₓ ⋄ ⊂1 2", "⊂4ₓ\n⊂⊂4ₓ\n⊂(1 2)\n"),
        ("f←{⍵=0:0 ⋄ 1+∇⍵-1} ⋄ f 500", "500\n"),
        ("1ₓ÷3ₓ ⋄ 6ₓ÷3ₓ ⋄ 1ₓ÷3", "1r3\n2ₓ\n0.3333333333333333\n"),
        ("(1j2)+(3j4) ⋄ (1j2)×(1j¯2) ⋄ +1j2", "4j6\n5\n1j¯2\n"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_bapl")).args(["-e", code]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_bapl")).args(["-e", "¯2+)"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("SYNTAX ERROR") && error.contains("<expression>:1:4\n¯2+)\n   ^"));
}

#[test]
fn repl_continuation_recovery_and_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bapl")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all("(2×3\n)+4\n1÷0\n2+2\n)\n¯2+5\n".as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n4\n3\n");
    let errors = String::from_utf8(output.stderr).unwrap();
    assert_eq!(errors.matches("DOMAIN ERROR").count(), 1);
    assert_eq!(errors.matches("SYNTAX ERROR").count(), 1);
    let mut child = Command::new(env!("CARGO_BIN_EXE_bapl")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"(2+\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr).unwrap().contains("unclosed delimiter"));
}

#[test]
fn json_session_flushes_before_eof_and_recovers() {
    use serde_json::{json, Value};
    use std::{
        io::{BufRead, BufReader},
        sync::mpsc,
        thread,
        time::Duration,
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_bapl")).arg("--json").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (send, recv) = mpsc::channel();
    let reader = thread::spawn(move || { for line in BufReader::new(output).lines() { if send.send(line.unwrap()).is_err() { break; } } });
    for (request, expected, error_kind, printed) in [
        (json!("v←⍳10").to_string(), Some(json!({"shape":[10], "data":(1..=10).map(f64::from).collect::<Vec<_>>(), "prototype":0.0})), None, vec![]),
        (json!("+/v").to_string(), Some(json!(55.0)), None, vec!["55"]),
        ("{".into(), None, Some("REQUEST ERROR"), vec![]),
        (json!(3).to_string(), None, Some("REQUEST ERROR"), vec![]),
        (json!(["1+2"]).to_string(), None, Some("REQUEST ERROR"), vec![]),
        (json!({"code":"1+2"}).to_string(), None, Some("REQUEST ERROR"), vec![]),
        (json!("⎕←7 ⋄ 1÷0").to_string(), None, Some("DOMAIN ERROR"), vec!["7"]),
        (json!("(2+").to_string(), None, Some("SYNTAX ERROR"), vec![]),
        (json!("⍝ \"quoted\"\n+/v").to_string(), Some(json!(55.0)), None, vec!["55"]),
        (json!("f←+").to_string(), None, None, vec![]),
        (json!("fs←+˘×").to_string(), None, Some("DOMAIN ERROR"), vec![]),
        (json!("f←2⊃fs ⋄ 2 f 3").to_string(), Some(json!(6.0)), None, vec!["6"]),
        (json!("").to_string(), None, None, vec![]),
        (json!("⍳0").to_string(), Some(json!({"shape":[0], "data":[], "prototype":0.0})), None, vec!["⍬"]),
    ] {
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        let line = match recv.recv_timeout(Duration::from_secs(5)) {
            Ok(line) => line,
            Err(e) => {
                child.kill().unwrap();
                panic!("reply was not flushed before EOF: {e}");
            }
        };
        let reply: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["output"], json!(printed));
        assert_eq!(reply["error"]["kind"].as_str(), error_kind);
        assert_eq!(reply["value"], expected.unwrap_or(Value::Null));
    }
    drop(input);
    let result = child.wait_with_output().unwrap();
    reader.join().unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
}

#[test]
fn batch_stdin_and_persistent_repl() {
    for args in [vec!["-"], vec![]] {
        let mut child =
            Command::new(env!("CARGO_BIN_EXE_bapl")).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all("v←⍳10\n+/v\n".as_bytes()).unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(String::from_utf8(result.stdout).unwrap(), "55\n");
        assert!(result.stderr.is_empty());
    }
}
