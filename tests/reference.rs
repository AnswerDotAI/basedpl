use basedpl::{reference, InterruptHandle};
use serde_json::{json, Value};

const SOURCES: &[(&str, &str)] = &[
    ("core", include_str!("reference/core.bpl")), ("graphics", include_str!("reference/graphics.bpl")), ("ngn", include_str!("reference/ngn.bpl")),
    ("april", include_str!("reference/april.bpl")), ("aplcart", include_str!("reference/aplcart.bpl")), ("dyalog", include_str!("reference/dyalog.bpl")),
    ("regex", include_str!("reference/regex.bpl")), ("distributions", include_str!("reference/distributions.bpl")), ("lib", include_str!("reference/lib.bpl")),
    ("bqn", include_str!("reference/bqn.bpl")), ("static", include_str!("reference/static.bpl")), ("static-int32", include_str!("reference/static-int32.bpl")),
];

fn cases(text: &str) -> Vec<Value> { reference::cases(text).unwrap() }

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
    let kinds = reference::check(&json!({"code":r#"{⍵?1;$t}"batch"⊃•prefs ⍬"#, "expected":{"shape":[], "data":[1], "prototype":0}, "serial":true}), &InterruptHandle::default());
    assert_eq!((kinds["status"].as_str(), kinds["message"].as_str()), (Some("mismatch"), Some("storage kind: float64 with the batch pref on, bool with the batch pref off")));
    let captured = json!({"code":"1ₓ", "expected":{"shape":[], "data":[1], "prototype":0}});
    assert_eq!(reference::check(&captured, &InterruptHandle::default())["status"], "pass");
    let parsed = cases("⍝  — [rtol=1e-14 atol=1e-15]\nf←{\n\n⍵+1\n}\nf 2\n⍝ =>\n3\n\n⍝  —\n\"unfinished\n⍝ error: SYNTAX ERROR\n\n");
    assert_eq!(parsed[0]["code"], "f←{\n\n⍵+1\n}\nf 2");
    assert_eq!(parsed[1]["line"], 10);
    for case in parsed { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    for (code, expect, status) in [
        ("1E¯10", "0", "mismatch"), ("1E¯16", "0", "pass"), ("1+1E¯15", "1", "pass"), ("a←3", "a", "invalid"), ("1", "{}0", "mismatch"),
        ("{}0", "{}0", "pass"), ("+", "{}0", "mismatch"), ("⍬", r#""""#, "mismatch"), (",1", "1", "mismatch"), ("0⍴⊂1 2", "0⍴⊂1", "mismatch"),
        (r#""aa":1"#, r#""bb":1"#, "mismatch"), (r#""aa":1"#, ",1", "mismatch"), (r#"["row":1]⍴1"#, r#"["col":1]⍴1"#, "mismatch"),
        ("3", "÷'a'", "invalid"),
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
}

/// Every case, run once for each of `reference::check`'s runs: with the `batch` pref off, then on, under which maps first try one call of their operand on batches of their cells, then on with fusion's small sizes, under which every fusible call waits and a graph splits even small arrays into blocks. Each run must meet the case's expectation, with values held in the same kinds of storage.
#[test]
fn reference_cases() {
    // Shared, so that no timing runs while the cases load the machine.
    let _lock = rustymath::timing::shared().unwrap();
    let mut ids = std::collections::HashSet::new();
    let mut chosen = Vec::new();
    let selected = std::env::var("BASEDPL_CASE").ok();
    let file = std::env::var("BASEDPL_SOURCE").ok();
    for &(name, source) in SOURCES {
        if file.as_deref().is_some_and(|f| f != name) { continue; }
        let all = reference::cases(source).unwrap_or_else(|e| panic!("{name}.bpl: {e}"));
        for id in all.iter().filter_map(|case| case["id"].as_str()).filter(|id| !id.is_empty()) {
            assert!(ids.insert(id.to_owned()), "duplicate source id: {id}");
        }
        let wanted = |case: &Value| selected.as_deref().is_none_or(|s| case["id"].as_str().unwrap().starts_with(s));
        chosen.extend(all.into_iter().filter(wanted).map(|mut case| { case["file"] = json!(name); case }));
    }
    let count = chosen.len();
    let failures: Vec<String> = reference::failures(&chosen, &InterruptHandle::default()).iter().map(|f| {
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
    assert!(count > 0, "no reference cases selected");
    assert!(failures.is_empty(), "{} of {count} reference cases failed:\n{}", failures.len(), failures.join("\n"));
    eprintln!("{count} reference cases passed with the batch pref off, on, and on with fusion at any size");
}
