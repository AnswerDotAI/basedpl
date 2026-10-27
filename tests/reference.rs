use basedpl::{reference, InterruptHandle};
use serde_json::{json, Value};

const SOURCES: &[(&str, &str)] = &[
    ("core", include_str!("reference/core.bpl")), ("graphics", include_str!("reference/graphics.bpl")), ("ngn", include_str!("reference/ngn.bpl")),
    ("april", include_str!("reference/april.bpl")), ("aplcart", include_str!("reference/aplcart.bpl")), ("dyalog", include_str!("reference/dyalog.bpl")),
    ("regex", include_str!("reference/regex.bpl")), ("distributions", include_str!("reference/distributions.bpl")), ("lib", include_str!("reference/lib.bpl")),
    ("bqn", include_str!("reference/bqn.bpl")), ("static", include_str!("reference/static.bpl")), ("static-int32", include_str!("reference/static-int32.bpl")),
];

fn cases(text: &str) -> Vec<Value> { reference::file("", text).unwrap().cases }

#[test]
fn reference_format_and_comparison() {
    let file = json!({"code":r#"testpath •nput "héllo" ⋄ •nget testpath"#, "expected_code":r#""héllo""#});
    for _ in 0..2 { assert_eq!(reference::check(&file, &InterruptHandle::default())["status"], "pass"); }
    let compact = cases(&format!("⍝ —\n{}", r#""a""⍝b" '⍝'   ⍝ "a""⍝b" '⍝'"#));
    assert_eq!(compact[0]["code"], r#""a""⍝b" '⍝'"#);
    assert_eq!(reference::check(&compact[0], &InterruptHandle::default())["status"], "pass");
    for source in ["⍝ —\n1\n1", "⍝ —\n{\n⍵\n}1\n⍝ =>\n1", "⍝ —\n⎕←1\n1\n⍝ ⎕: 1"] {
        for ending in ["", "\n", "\n\n"] { assert_eq!(cases(&format!("{source}{ending}")), cases(&format!("{source}\n\n"))); }
    }
    assert_eq!(cases("⍝ —\n{\n⍵\n}1   ⍝ 1"), cases("⍝ —\n{\n⍵\n}1\n⍝ =>\n1"));
    let parsed_output = cases("⍝⍝ Output\n\n⍝ —\n⎕←9 ⋄ ⎕←2 ⋄ 7\n7\n⍝ ⎕: 9\\n2\n\n⍝ —\n3\n3\n⍝ ⎕:\n\n⍝ —\n⍎¨[⎕ ⎕]\n3 4\n⍝ input: 3\\n4\n⍝ ⎕:\n\n");
    assert_eq!(parsed_output[0]["section"], "Output");
    assert_eq!(parsed_output[0]["expected_output"], "9\n2");
    assert_eq!(parsed_output[2]["input"], "3\n4");
    for case in parsed_output { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    let mut output_error = json!({"code":"⎕←9 ⋄ 1÷'a'", "expected_error":"DOMAIN ERROR", "expected_output":"9"});
    assert_eq!(reference::check(&output_error, &InterruptHandle::default())["status"], "pass");
    output_error["expected_output"] = json!("2");
    assert_eq!(reference::check(&output_error, &InterruptHandle::default())["status"], "mismatch");
    let exactness = json!({"code":"1", "expected_code":"1ₓ"});
    assert_eq!(reference::check(&exactness, &InterruptHandle::default())["status"], "mismatch");
    let kinds = reference::check(&json!({"code":r#"{⍵?1;$t}"batch"⊃•prefs ⍬"#, "expected":{"shape":[], "data":[1], "prototype":0}}), &InterruptHandle::default());
    assert_eq!((kinds["status"].as_str(), kinds["message"].as_str()), (Some("mismatch"), Some("storage kind: float64 with the batch pref on, bool with the batch pref off")));
    let captured = json!({"code":"1ₓ", "expected":{"shape":[], "data":[1], "prototype":0}});
    assert_eq!(reference::check(&captured, &InterruptHandle::default())["status"], "pass");
    let parsed = cases("⍝  — [rtol=1e-14 atol=1e-15]\nf←{\n\n⍵+1\n}\nf 2\n⍝ =>\n3\n\n⍝  —\n\"unfinished\n⍝ error: SYNTAX ERROR\n\n");
    assert_eq!(parsed[0]["code"], "f←{\n\n⍵+1\n}\nf 2");
    assert_eq!(parsed[1]["line"], 10);
    for case in parsed { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    let text = "⍝⍝ setup\nv←1 2 3\n\n⍝⍝ copies •prefs [\"float\":32]\n⍝ mutate — expectation gets its own setup\nv←v+1 ⋄ v\n⍝ =>\nv+1\n⍝⍝: 2 3 4\n\n-v\n⍝⍝: 1 1 1\n";
    let file = reference::file("sample.bpl", text).unwrap();
    assert_eq!(file.cases.len(), 2);
    assert_eq!(file.cases[0]["prefs"], "•prefs [\"float\":32]");
    let span = file.cases[0]["budget_span"].as_array().unwrap();
    assert_eq!(&text[span[0].as_u64().unwrap() as usize..span[1].as_u64().unwrap() as usize], "⍝⍝: 2 3 4");
    assert!(reference::failures(file.cases.iter().filter(|c| reference::checked(c)), &file.setup, &InterruptHandle::default()).is_empty());
    reference::check_once(&file.cases[0], &file.setup, &InterruptHandle::default()).unwrap();
    for (code, expect, status) in [
        ("1E¯10", "0", "mismatch"), ("1E¯16", "0", "pass"), ("1+1E¯15", "1", "pass"), ("a←3", "a", "invalid"), ("1", "{}0", "mismatch"),
        ("{}0", "{}0", "pass"), ("+", "{}0", "mismatch"), ("⍬", r#""""#, "mismatch"), (",1", "1", "mismatch"), ("0⍴⊂1 2", "0⍴⊂1", "mismatch"),
        (r#""aa":1"#, r#""bb":1"#, "mismatch"), (r#""aa":1"#, ",1", "mismatch"), (r#"["row":1]⍴1"#, r#"["col":1]⍴1"#, "mismatch"),
        ("3", "÷'a'", "invalid"), ("32•float 1+3×2*¯23", "32•float 1", "pass"), ("1+1E¯8", "32•float 1+1E¯8", "mismatch"),
    ] {
        let case = json!({"code":code, "expected_code":expect});
        assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], status, "{code} vs {expect}");
    }
    let mut case = json!({"code":"1E¯16", "expected_code":"0", "relative_tolerance":1e-14, "absolute_tolerance":0.0});
    assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "mismatch");
    case["absolute_tolerance"] = json!(1e-15);
    assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass");
    case["code"] = json!("1E¯10");
    assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "mismatch");
    let exact = json!({"code":"32•float 1+3×2*¯23", "expected_code":"32•float 1", "relative_tolerance":0.0});
    assert_eq!(reference::check(&exact, &InterruptHandle::default())["status"], "mismatch");
}

/// The variable that makes a run of `reference_cases` check one share of the cases: the share's number and the number of shares, as in `3 8`.
const SHARD: &str = "BASEDPL_SHARD";
/// The start of each line on which a share prints one of its failures, as JSON.
const FAILURE: &str = "reference failure: ";

/// The cases that `BASEDPL_SOURCE` and `BASEDPL_CASE` select, in `SOURCES`' order, each with its file's name as `file`. Panics on an id that two cases share.
fn chosen() -> Vec<reference::File> {
    let mut chosen = Vec::new();
    let selected = std::env::var("BASEDPL_CASE").ok();
    let source = std::env::var("BASEDPL_SOURCE").ok();
    let speed = reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap();
    for mut file in SOURCES.iter().map(|&(name, text)| reference::file(name, text).unwrap_or_else(|e| panic!("{name}.bpl: {e}"))).chain(speed) {
        let name = file.name.trim_end_matches(".bpl");
        if source.as_deref().is_some_and(|f| f != name) { continue; }
        let wanted = |case: &Value| selected.as_deref().is_none_or(|s| case["id"].as_str().unwrap().starts_with(s));
        file.cases.retain(|case| reference::checked(case) && wanted(case));
        for case in &mut file.cases { case["file"] = json!(name); }
        if !file.cases.is_empty() { chosen.push(file); }
    }
    chosen
}

/// Every case, run once for each of `reference::check`'s runs: with the `batch` pref off, then on, under which maps first try one call of their operand on batches of their cells, then on with fusion's small sizes, under which every fusible call waits, a block stage splits even small arrays into blocks and every stage compiles its parts, then the same with no compiled parts where that run used one, then on with the code as the body of a dfn called twice. Each run but the last must meet the case's expectation, with values held in the same kinds of storage. In the last, the dfn's second call, which runs the statements its first call recorded, must give the first call's result. A process evaluates one case at a time, so the test runs a child process of itself for each core. Each child checks every n-th case (`SHARD`) and prints its failures for the parent (`FAILURE`).
#[test]
fn reference_cases() {
    let chosen = chosen();
    if let Ok(shard) = std::env::var(SHARD) {
        let (k, n): (usize, usize) = shard.split_once(' ').and_then(|(k, n)| Some((k.parse().ok()?, n.parse().ok()?))).expect("BASEDPL_SHARD holds a share and a count");
        let mut offset = 0;
        for file in &chosen {
            let cases = file.cases.iter().enumerate().filter(|(i, _)| (offset+i)%n == k).map(|(_, c)| c);
            for failure in reference::failures(cases, &file.setup, &InterruptHandle::default()) { println!("{FAILURE}{failure}"); }
            offset += file.cases.len();
        }
        return;
    }
    // Shared, so that no timing runs while the cases load the machine.
    let _lock = rustymath::timing::shared().unwrap();
    let count: usize = chosen.iter().map(|f| f.cases.len()).sum();
    assert!(count > 0, "no reference cases selected");
    let shards = std::thread::available_parallelism().map_or(1, |n| n.get()).min(count);
    // macOS reads the directory that holds a process's executable as its bundle when a framework such as ImageIO first asks for the bundle's information, and `target/debug/deps` holds hundreds of thousands of files. The children run from a copy of this test in a directory of its own.
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("reference");
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let children: Vec<_> = (0..shards).map(|k| std::process::Command::new(&exe)
        .args(["reference_cases", "--exact", "--nocapture"]).env(SHARD, format!("{k} {shards}"))
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap()).collect();
    let mut failures: Vec<Value> = children.into_iter().flat_map(|child| {
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(output.status.success(), "a share of the reference cases stopped:\n{stdout}{}", String::from_utf8_lossy(&output.stderr));
        stdout.lines().filter_map(|line| line.strip_prefix(FAILURE)).map(|line| serde_json::from_str(line).unwrap()).collect::<Vec<Value>>()
    }).collect();
    failures.sort_by(|a, b| (a["file"].as_str(), a["line"].as_u64()).cmp(&(b["file"].as_str(), b["line"].as_u64())));
    let failures: Vec<String> = failures.iter().map(|f| {
        format!(
            "{}.bpl:{} [{}] {}: {} ({})",
            f["file"].as_str().unwrap(),
            f["line"],
            f["section"].as_str().unwrap(),
            f["id"].as_str().unwrap(),
            f["message"],
            f["kind"]
        )
    }).collect();
    assert!(failures.is_empty(), "{} of {count} reference cases failed:\n{}", failures.len(), failures.join("\n"));
    eprintln!("{count} reference cases passed with the batch pref off, on, and on with fusion at any size, with compiled parts and without");
}
