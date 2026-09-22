use basedpl::{reference, EvalOptions};
use serde_json::{json, Value};

const SOURCES: [(&str, &str); 5] = [
    ("core", include_str!("reference/core.apl")),
    ("ngn", include_str!("reference/ngn.apl")),
    ("april", include_str!("reference/april.apl")),
    ("aplcart", include_str!("reference/aplcart.apl")),
    ("dyalog", include_str!("reference/dyalog.apl")),
];

fn header(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix("⍝ ")?;
    let (id, comment) = match line.strip_prefix('—') { Some(comment) => ("", comment), None => line.split_once(" —")? };
    if id.chars().any(char::is_whitespace) { return None; }
    Some((id, if comment.is_empty() { "" } else { comment.strip_prefix(' ')? }))
}

fn inline(line: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        if c == '\'' { quoted = !quoted; } else if c == '⍝' && !quoted { return Some((line[..i].trim_end(), line[i + c.len_utf8()..].trim_start())); }
    }
    None
}

fn cases(text: &str) -> Vec<Value> {
    if text.is_empty() { return Vec::new(); }
    let lines: Vec<_> = text.split_terminator('\n').collect();
    let mut starts: Vec<_> = lines.iter().enumerate().filter_map(|(i, s)| (header(s).is_some() || s.starts_with("⍝⍝ ")).then_some(i)).collect();
    assert_eq!(starts.first(), Some(&0), "expected a case or section header on line 1");
    starts.push(lines.len());
    let mut section = "";
    starts
        .windows(2)
        .filter_map(|range| {
            let (start, end) = (range[0], range[1]);
            if let Some(title) = lines[start].strip_prefix("⍝⍝ ") {
                assert!(lines[start + 1..end].iter().all(|s| s.is_empty()), "expected a case after section heading");
                section = title;
                return None;
            }
            let (id, comment) = header(lines[start]).unwrap();
            let location = format!("line {} ({id})", start + 1);
            let mut case = json!({"id":id, "line":start + 1, "section":section});
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
            let mut body = &lines[start + 1..end];
            if body.last() == Some(&"") { body = &body[..body.len() - 1]; }
            else { assert_eq!(end, lines.len(), "{location}: missing blank record separator"); }
            if let Some(text) = body.last().and_then(|s| s.strip_prefix("⍝ ⎕:")) {
                let mut output = String::new();
                let mut chars = text.strip_prefix(' ').unwrap_or(text).chars();
                while let Some(c) = chars.next() {
                    output.push(if c == '\\' {
                        match chars.next() { Some('n') => '\n', Some('\\') => '\\', _ => panic!("{location}: output escapes are \\n and \\\\") }
                    } else { c });
                }
                case["expected_output"] = json!(output);
                body = &body[..body.len() - 1];
            }
            assert!(!body.iter().any(|s| s.starts_with("⍝ ⎕:")), "{location}: output expectation must be last");
            let splits: Vec<_> = body.iter().enumerate().filter_map(|(i, s)| (*s == "⍝ =>").then_some(i)).collect();
            let (code, expect) = match splits.as_slice() {
                [] if body.len() == 2 => (body[0].to_owned(), body[1].to_owned()),
                [] if body.len() == 1 => {
                    let (code, expect) = inline(body[0]).expect("inline case needs ⍝ before its expectation");
                    (code.to_owned(), expect.to_owned())
                }
                [i] => (body[..*i].join("\n"), body[i + 1..].join("\n")),
                _ => panic!("{location}: use one ⍝ => between multiline expressions"),
            };
            assert!(!expect.is_empty(), "{location}: missing expectation");
            case["code"] = json!(code);
            if let Some(kind) = expect.strip_prefix("⍝ error: ") { case["expected_error"] = json!(kind); }
            else { case["expected_code"] = json!(expect); }
            Some(case)
        })
        .collect()
}

#[test]
fn reference_format_and_comparison() {
    let compact = cases("⍝ —\n'a''⍝b'   ⍝ 'a''⍝b'");
    assert_eq!(compact[0]["code"], "'a''⍝b'");
    assert_eq!(reference::check(&compact[0], EvalOptions::default())["status"], "pass");
    for source in ["⍝ —\n1\n1", "⍝ —\n{\n⍵\n}1\n⍝ =>\n1", "⍝ —\n⎕←1\n1\n⍝ ⎕: 1"] {
        for ending in ["", "\n", "\n\n"] { assert_eq!(cases(&format!("{source}{ending}")), cases(&format!("{source}\n\n"))); }
    }
    let parsed_output = cases("⍝⍝ Output\n\n⍝ —\n⎕←9 ⋄ ⎕←2 ⋄ 7\n7\n⍝ ⎕: 9\\n2\n\n⍝ —\n3\n3\n⍝ ⎕:\n\n");
    assert_eq!(parsed_output[0]["section"], "Output");
    assert_eq!(parsed_output[0]["expected_output"], "9\n2");
    for case in parsed_output { assert_eq!(reference::check(&case, EvalOptions::default())["status"], "pass"); }
    let mut output_error = json!({"code":"⎕←9 ⋄ 1÷0", "expected_error":"DOMAIN ERROR", "expected_output":"9"});
    assert_eq!(reference::check(&output_error, EvalOptions::default())["status"], "pass");
    output_error["expected_output"] = json!("2");
    assert_eq!(reference::check(&output_error, EvalOptions::default())["status"], "mismatch");
    let representation = json!({"code":"1", "expected_code":"1x", "exact_representation":true});
    assert_eq!(reference::check(&representation, EvalOptions::default())["status"], "mismatch");
    let parsed = cases("⍝  — [rtol=1e-14 atol=1e-15]\nf←{\n\n⍵+1\n}\nf 2\n⍝ =>\n3\n\n⍝  —\n'unfinished\n⍝ error: SYNTAX ERROR\n\n");
    assert_eq!(parsed[0]["code"], "f←{\n\n⍵+1\n}\nf 2");
    assert_eq!(parsed[1]["line"], 10);
    for case in parsed { assert_eq!(reference::check(&case, EvalOptions::default())["status"], "pass"); }
    for (code, expect, status) in [
        ("1E¯10", "0", "mismatch"),
        ("1E¯16", "0", "pass"),
        ("1+1E¯15", "1", "pass"),
        ("a←3", "a", "invalid"),
        ("1", "{}0", "mismatch"),
        ("{}0", "{}0", "pass"),
        ("+", "{}0", "mismatch"),
        ("⍬", "''", "mismatch"),
        (",1", "1", "mismatch"),
        ("0⍴⊂1 2", "0⍴⊂1", "mismatch"),
        ("3", "÷0", "invalid"),
    ] {
        let case = json!({"code":code, "expected_code":expect});
        assert_eq!(reference::check(&case, EvalOptions::default())["status"], status, "{code} vs {expect}");
    }
    let mut case = json!({"code":"1E¯16", "expected_code":"0", "relative_tolerance":1e-14, "absolute_tolerance":0.0});
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "mismatch");
    case["absolute_tolerance"] = json!(1e-15);
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "pass");
    case["code"] = json!("1E¯10");
    assert_eq!(reference::check(&case, EvalOptions::default())["status"], "mismatch");
}

fn run_reference_cases(sources: &[(&str, &str)], timeout: u64) {
    let mut ids = std::collections::HashSet::new();
    let mut count = 0;
    let mut failures = Vec::new();
    let selected = std::env::var("BASEDPL_CASE").ok();
    for &(name, source) in sources {
        for mut case in cases(source) {
            if name == "core" { case["exact_representation"] = json!(true); }
            let id = case["id"].as_str().unwrap();
            assert!(id.is_empty() || ids.insert(id.to_owned()), "duplicate source id: {id}");
            if selected.as_deref().is_some_and(|s| !id.starts_with(s)) { continue; }
            let result = reference::check(&case, EvalOptions { timeout: Some(std::time::Duration::from_secs(timeout)), echo: false, ..EvalOptions::default() });
            if result["status"] != "pass" {
                failures.push(format!(
                    "{name}.apl:{} [{}] {id}: {} ({})",
                    case["line"],
                    case["section"].as_str().unwrap(),
                    result["message"],
                    result["actual"]["error"]["kind"]
                ));
            }
            count += 1;
        }
    }
    assert!(count > 0, "no reference cases selected");
    assert!(failures.is_empty(), "{} of {count} reference cases failed:\n{}", failures.len(), failures.join("\n"));
    eprintln!("{count} reference cases passed");
}

#[test]
fn enabled_reference_cases() { run_reference_cases(&SOURCES, 2); }

#[test]
#[ignore = "slow workloads"]
fn slow_reference_cases() { run_reference_cases(&[("slow", include_str!("reference/slow.apl"))], 60); }
