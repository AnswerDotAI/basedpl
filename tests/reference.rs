use miniapl::{Array, Element, Session};
use serde_json::Value;

const SOURCES: [&str; 4] = [
    include_str!("reference/ngn.jsonl"),
    include_str!("reference/april.jsonl"),
    include_str!("reference/aplcart.jsonl"),
    include_str!("reference/dyalog.jsonl"),
];

fn element(value: &Value) -> Element {
    if let Some(n) = value.as_f64() { Element::Number(n.try_into().unwrap()) } else if let Some(s) = value.as_str() {
        let mut chars = s.chars();
        let c = chars.next().expect("character expectation");
        assert!(chars.next().is_none(), "one character per element");
        Element::Character(c)
    } else if let Some(z) = value.get("complex") {
        Element::Number(num_complex::Complex64::new(z[0].as_f64().unwrap(), z[1].as_f64().unwrap()).try_into().unwrap())
    } else { Element::Nested(expected_array(value)) }
}

fn expected_array(value: &Value) -> Array {
    let shape = value["shape"].as_array().unwrap().iter().map(|n| n.as_u64().unwrap() as usize).collect();
    let data = value["data"].as_array().unwrap().iter().map(element).collect();
    let prototype = element(&value["prototype"]);
    let result = Array::from_parts(shape, data, prototype.clone()).expect("valid independently specified array");
    assert_eq!(result.prototype(), &prototype, "independently specified prototype");
    result
}

fn same_element(x: &Element, y: &Element, tolerance: f64) -> bool {
    match (x, y) {
        (Element::Nested(x), Element::Nested(y)) => same_array(x, y, tolerance),
        (Element::Number(x), Element::Number(y)) if x.is_exact() && y.as_float().is_some() => {
            x.as_exact() == num_rational::BigRational::from_float(y.as_float().unwrap())
        }
        (Element::Number(x), Element::Number(y)) if tolerance != 0.0 => {
            let complex = |n: &miniapl::Number| n.as_complex().or_else(|| n.as_float().map(|x| num_complex::Complex64::new(x, 0.)));
            match (complex(x), complex(y)) { (Some(a), Some(b)) => a == b || (a - b).norm() <= tolerance * a.norm().max(b.norm()), _ => x == y }
        }
        _ => x == y,
    }
}

fn same_array(x: &Array, y: &Array, tolerance: f64) -> bool {
    x.shape() == y.shape()
        && same_element(x.prototype(), y.prototype(), tolerance)
        && x.elements().zip(y.elements()).all(|(a, b)| same_element(&a, &b, tolerance))
}

fn check(case: &Value) {
    let id = case["id"].as_str().unwrap();
    let code = case["code"].as_str().unwrap();
    assert!(!code.is_empty(), "{id}: needs a concrete program");
    let error_kind = case["expected_error"].as_str().filter(|s| !s.is_empty());
    assert!(case.get("expected").is_some() || error_kind.is_some(), "{id}: needs an independent expectation");
    let result = Session::new().eval(code);
    if let Some(kind) = error_kind {
        let error = result.error.unwrap_or_else(|| panic!("{id}: {code}: expected {kind}, got {:?}", result.value));
        assert_eq!(error.kind.to_string(), kind, "{id}: {code}");
    } else {
        assert!(result.error.is_none(), "{id}: {code}: {:?}", result.error);
        let actual = result.value.unwrap_or_else(|| panic!("{id}: {code}: no result"));
        let expected = expected_array(&case["expected"]);
        let tolerance = case["relative_tolerance"].as_f64().unwrap_or(0.0);
        assert!(same_array(&actual, &expected, tolerance), "{id}: {code}: actual {actual:?}, expected {expected:?}");
    }
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
