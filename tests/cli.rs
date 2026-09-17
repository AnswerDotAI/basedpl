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
        ("1x÷3x ⋄ 6x÷3x ⋄ 1x÷3", "1r3\n2x\n0.3333333333333333\n"),
        ("(1J2)+(3J4) ⋄ (1J2)×(1J¯2) ⋄ +1J2", "4J6\n5\n1J¯2\n"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_miniapl")).args(["-e", code]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_miniapl")).args(["-e", "¯2+)"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "SYNTAX ERROR: unexpected closing parenthesis\n --> <expression>:1:4\n¯2+)\n   ^\n");
}

#[test]
fn repl_continuation_recovery_and_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_miniapl")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all("(2×\n3)+4\n1÷0\n2+2\n)\n¯2+5\n".as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n4\n3\n");
    let errors = String::from_utf8(output.stderr).unwrap();
    assert_eq!(errors.matches("DOMAIN ERROR").count(), 1);
    assert_eq!(errors.matches("SYNTAX ERROR").count(), 1);
    let mut child = Command::new(env!("CARGO_BIN_EXE_miniapl")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"(2+\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr).unwrap().contains("unclosed parenthesis"));
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
    let mut child =
        Command::new(env!("CARGO_BIN_EXE_miniapl")).arg("--json").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (send, recv) = mpsc::channel();
    let reader = thread::spawn(move || { for line in BufReader::new(output).lines() { if send.send(line.unwrap()).is_err() { break; } } });
    for (request, expected, error_kind, printed) in [
        (json!("v←⍳10").to_string(), Some(json!((1..=10).collect::<Vec<_>>())), None, vec![]),
        (json!("+/v").to_string(), Some(json!([55])), None, vec!["55"]),
        ("{".into(), None, Some("REQUEST ERROR"), vec![]),
        (json!(3).to_string(), None, Some("REQUEST ERROR"), vec![]),
        (json!(["1+2"]).to_string(), None, Some("REQUEST ERROR"), vec![]),
        (json!({"code":"1+2"}).to_string(), None, Some("REQUEST ERROR"), vec![]),
        (json!("⎕←7 ⋄ 1÷0").to_string(), None, Some("DOMAIN ERROR"), vec!["7"]),
        (json!("(2+").to_string(), None, Some("SYNTAX ERROR"), vec![]),
        (json!("⍝ \"quoted\"\n+/v").to_string(), Some(json!([55])), None, vec!["55"]),
        (json!("f←+").to_string(), None, None, vec![]),
        (json!("").to_string(), None, None, vec![]),
        (json!("⍳0").to_string(), Some(json!([])), None, vec!["⍬"]),
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
        match expected {
            Some(data) => {
                let actual: Vec<f64> = reply["value"]["data"].as_array().unwrap().iter().map(|n| n.as_f64().unwrap()).collect();
                let expected: Vec<f64> = data.as_array().unwrap().iter().map(|n| n.as_f64().unwrap()).collect();
                assert_eq!(actual, expected);
                assert_eq!(reply["value"]["prototype"].as_f64(), Some(0.0));
            }
            None => assert!(reply["value"].is_null()),
        }
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
            Command::new(env!("CARGO_BIN_EXE_miniapl")).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all("v←⍳10\n+/v\n".as_bytes()).unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(String::from_utf8(result.stdout).unwrap(), "55\n");
        assert!(result.stderr.is_empty());
    }
}
