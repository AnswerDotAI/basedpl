use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn native_expression_and_diagnostic() {
    for (code, expected) in [("2×3+4", "14\n"), ("⊂4ₓ ⋄ ⊂⊂4ₓ ⋄ ⊂1 2", "⊂4ₓ\n⊂⊂4ₓ\n⊂[1 2]\n"), ("f←{⍵=0?0;1+∇⍵-1} ⋄ f 500", "500\n")]
    {
        let output = Command::new(env!("CARGO_BIN_EXE_bpl")).args(["-e", code]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_bpl")).args(["-e", "¯2+)"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("SYNTAX ERROR") && error.contains("<expression>:1:4\n¯2+)\n   ^"));
}

#[test]
fn repl_continuation_recovery_and_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bpl")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all("(2×3\n)+4\n1÷'a'\n2+2\n)\n¯2+5\n".as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n4\n3\n");
    let errors = String::from_utf8(output.stderr).unwrap();
    assert_eq!(errors.matches("DOMAIN ERROR").count(), 1);
    assert_eq!(errors.matches("SYNTAX ERROR").count(), 1);
    let mut child = Command::new(env!("CARGO_BIN_EXE_bpl")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"(2+\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr).unwrap().contains("unclosed delimiter"));
}

#[test]
fn worker_flushes_before_eof_and_recovers() {
    use serde_json::{json, Value};
    use std::{
        io::{BufRead, BufReader},
        sync::mpsc,
        thread,
        time::Duration,
    };
    let mut child =
        Command::new(env!("CARGO_BIN_EXE_bpl")).arg("--worker").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (send, recv) = mpsc::channel();
    let reader = thread::spawn(move || { for line in BufReader::new(output).lines() { if send.send(line.unwrap()).is_err() { break; } } });
    let code = |source: &str| json!({"code": source}).to_string();
    for (request, expected, error_kind, printed) in [
        (code("v←⍳10"), Some(json!({"shape":[10], "data":(0..10).map(f64::from).collect::<Vec<_>>(), "prototype":0.0})), None, vec![]),
        (json!({"id":2, "code":"+/v"}).to_string(), Some(json!(45.0)), None, vec!["45"]),
        ("{".into(), None, Some("REQUEST"), vec![]),
        (json!(3).to_string(), None, Some("REQUEST"), vec![]),
        (json!("1+2").to_string(), None, Some("REQUEST"), vec![]),
        (json!({"id":"x", "code":"1+2"}).to_string(), None, Some("REQUEST"), vec![]),
        (code("⎕←7 ⋄ 1÷'a'"), None, Some("DOMAIN"), vec!["7"]),
        (code("(2+"), None, Some("SYNTAX"), vec![]),
        (code("⍝ \"quoted\"\n+/v"), Some(json!(45.0)), None, vec!["45"]),
        (code("f←+"), None, None, vec![]),
        (code("fs←[+ ×]"), None, Some("DOMAIN"), vec![]),
        (code("f←1⊃fs ⋄ 2 f 3"), Some(json!(6.0)), None, vec!["6"]),
        (code(""), None, None, vec![]),
        (code("⍳0"), Some(json!({"shape":[0], "data":[], "prototype":0.0})), None, vec!["⍬"]),
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
        let id = serde_json::from_str::<Value>(&request).ok().and_then(|r| r.get("id").filter(|id| id.is_u64()).cloned());
        assert_eq!(reply["id"], id.unwrap_or(Value::Null));
        let result = &reply["result"];
        assert_eq!(result["output"].as_array().unwrap().iter().map(|e| e["data"]["text/plain"].as_str().unwrap()).collect::<Vec<_>>(), printed);
        assert_eq!(result["error"]["kind"].as_str(), error_kind);
        assert_eq!(result["value"], expected.unwrap_or(Value::Null));
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_bpl")).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all("v←⍳10\n+/v\n".as_bytes()).unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(String::from_utf8(result.stdout).unwrap(), "45\n");
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn programs_read_standard_input() {
    let run = |args: &[&str], input: &str| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bpl")).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        (String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap())
    };
    assert_eq!(run(&["-e", "⎕ ⋄ ⍴⎕ ⋄ ⎕ ⋄ ⍬≡⎕"], "first\n\nlast"), ("first\n[0]ₓ\nlast\n$t\n".into(), String::new()));
    assert_eq!(run(&["-e", "⎕ ⋄ ≢•nget \"-\""], "head\nab\ncd\n"), ("head\n6ₓ\n".into(), String::new()));
    // Standard input holds the program itself here.
    for args in [&["-"][..], &[]] { assert!(run(args, "⎕\n").1.contains("IO ERROR: standard input is unavailable here")); }
}
