use miniapl::{reference, EvalOptions};
use serde_json::Value;

const SOURCES: [&str; 4] = [
    include_str!("reference/ngn.jsonl"),
    include_str!("reference/april.jsonl"),
    include_str!("reference/aplcart.jsonl"),
    include_str!("reference/dyalog.jsonl"),
];

fn check(case: &Value) {
    let result = reference::check(case, EvalOptions { timeout: Some(std::time::Duration::from_secs(2)), ..EvalOptions::default() });
    assert_eq!(result["status"], "pass", "{}: {}: {} {}", case["id"], case["code"], result["kind"], result["message"]);
}

#[test]
fn numerical_reference_tolerance() {
    let mut case = serde_json::json!({"code":"1E¯16", "expected":{"shape":[], "data":[0], "prototype":0}, "relative_tolerance":1e-14});
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "mismatch");
    case["absolute_tolerance"] = serde_json::json!(1e-15);
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "pass");
    case["code"] = serde_json::json!("1E¯10");
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "mismatch");
}

#[test]
fn enabled_reference_cases() {
    let mut ids = std::collections::HashSet::new();
    let mut active = 0;
    for source in SOURCES {
        for line in source.lines() {
            let case: Value = serde_json::from_str(line).unwrap();
            assert!(ids.insert(case["id"].as_str().unwrap().to_owned()), "duplicate source id: {}", case["id"]);
            match case["status"].as_str().unwrap() {
                "active" => {
                    check(&case);
                    active += 1;
                }
                "pending" | "question" | "excluded" => assert!(!case["reason"].as_str().unwrap().is_empty()),
                status => panic!("unknown reference-case status: {status}"),
            }
        }
    }
    assert!(active > 0, "enable reference cases before claiming coverage");
    eprintln!("{active} active reference cases; {} retained source entries", ids.len());
}

#[test]
#[ignore = "select a pending case with MINIAPL_CASE=source:line; review its expectation before enabling"]
fn pending_reference_case() {
    let id = std::env::var("MINIAPL_CASE").expect("set MINIAPL_CASE to the exact source:line identifier");
    let case = SOURCES
        .iter()
        .flat_map(|s| s.lines())
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .find(|case| case["id"] == id)
        .expect("known reference case id");
    assert_ne!(case["status"], "excluded", "explicitly out of scope: {}", case["reason"]);
    check(&case);
}
