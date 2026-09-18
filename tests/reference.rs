use miniapl::{reference, EvalOptions};
use serde_json::{json, Value};

const SOURCES: [(&str, &str); 4] = [
    ("ngn", include_str!("reference/ngn.apl")),
    ("april", include_str!("reference/april.apl")),
    ("aplcart", include_str!("reference/aplcart.apl")),
    ("dyalog", include_str!("reference/dyalog.apl")),
];

fn header(line: &str) -> Option<(&str, &str)> {
    let (id, comment) = line.strip_prefix("⍝ ")?.split_once(" —")?;
    if id.chars().any(char::is_whitespace) { return None; }
    Some((id, if comment.is_empty() { "" } else { comment.strip_prefix(' ')? }))
}

fn cases(text: &str) -> Vec<Value> {
    if text.is_empty() { return Vec::new(); }
    let lines: Vec<_> = text.split_terminator('\n').collect();
    let mut starts: Vec<_> = lines.iter().enumerate().filter_map(|(i, s)| header(s).map(|_| i)).collect();
    assert_eq!(starts.first(), Some(&0), "expected a case header on line 1");
    starts.push(lines.len());
    starts
        .windows(2)
        .map(|range| {
            let (start, end) = (range[0], range[1]);
            let (id, comment) = header(lines[start]).unwrap();
            let location = format!("line {} ({id})", start + 1);
            let mut case = json!({"id":id, "line":start + 1});
            let suffix = comment.strip_prefix('[').or_else(|| comment.rsplit_once(" [").map(|(_, s)| s));
            if let Some(options) = suffix.and_then(|s| s.strip_suffix(']')).filter(|s| s.starts_with("rtol=") || s.starts_with("atol=")) {
                for option in options.split_whitespace() {
                    let (key, value) = option.split_once('=').expect("tolerance key=value");
                    let key = match key { "rtol" => "relative_tolerance", "atol" => "absolute_tolerance", _ => panic!("{location}: unknown tolerance {key}") };
                    let value: f64 = value.parse().expect("numeric tolerance");
                    assert!(value.is_finite() && value >= 0. && case.get(key).is_none(), "{location}: invalid tolerance {option}");
                    case[key] = json!(value);
                }
            }
            assert!(end > start + 1 && lines[end - 1].is_empty(), "{location}: missing blank record separator");
            let body = &lines[start + 1..end - 1];
            let splits: Vec<_> = body.iter().enumerate().filter_map(|(i, s)| (*s == "⍝ =>").then_some(i)).collect();
            let (code, expect) = match splits.as_slice() {
                [] if body.len() == 2 => (body[0].to_owned(), body[1].to_owned()),
                [i] => (body[..*i].join("\n"), body[i + 1..].join("\n")),
                _ => panic!("{location}: use one ⍝ => between multiline expressions"),
            };
            assert!(!expect.is_empty(), "{location}: missing expectation");
            case["code"] = json!(code);
            if let Some(kind) = expect.strip_prefix("⍝ error: ") { case["expected_error"] = json!(kind); }
            else { case["expected_code"] = json!(expect); }
            case
        })
        .collect()
}

#[test]
fn reference_format_and_comparison() {
    let parsed = cases("⍝  — [rtol=1e-14 atol=1e-15]\nf←{\n\n⍵+1\n}\nf 2\n⍝ =>\n3\n\n⍝  —\n'unfinished\n⍝ error: SYNTAX ERROR\n\n");
    assert_eq!(parsed[0]["code"], "f←{\n\n⍵+1\n}\nf 2");
    assert_eq!(parsed[1]["line"], 10);
    for case in parsed { assert_eq!(reference::check(&case, EvalOptions::default())["status"], "pass"); }
    for (code, expect, status) in [
        ("1E¯16", "0", "mismatch"),
        ("a←3", "a", "invalid"),
        ("1", "{}0", "mismatch"),
        ("{}0", "{}0", "pass"),
        ("⍬", "''", "mismatch"),
        (",1", "1", "mismatch"),
        ("0⍴⊂1 2", "0⍴⊂1", "mismatch"),
        ("3", "÷0", "invalid"),
    ] {
        let case = json!({"code":code, "expected_code":expect});
        assert_eq!(reference::check(&case, EvalOptions::default())["status"], status, "{code} vs {expect}");
    }
    let mut case = json!({"code":"1E¯16", "expected_code":"0", "relative_tolerance":1e-14});
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "mismatch");
    case["absolute_tolerance"] = json!(1e-15);
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "pass");
    case["code"] = json!("1E¯10");
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "mismatch");
}

#[test]
fn enabled_reference_cases() {
    let mut ids = std::collections::HashSet::new();
    let mut count = 0;
    let selected = std::env::var("MINIAPL_CASE").ok();
    for (name, source) in SOURCES {
        for case in cases(source) {
            let id = case["id"].as_str().unwrap();
            assert!(id.is_empty() || ids.insert(id.to_owned()), "duplicate source id: {id}");
            if selected.as_deref().is_some_and(|s| s != id) { continue; }
            let result = reference::check(&case, EvalOptions { timeout: Some(std::time::Duration::from_secs(2)), echo: false, ..EvalOptions::default() });
            assert_eq!(result["status"], "pass", "{name}.apl:{} {id}: {}: {}", case["line"], case["code"], result);
            count += 1;
        }
    }
    assert!(count > 0, "no reference cases selected");
    eprintln!("{count} reference cases passed");
}
