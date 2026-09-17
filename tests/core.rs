use miniapl::{
    parse, Array,
    Element::{self, *},
    Error,
    ErrorKind::*,
    ParseStatus, Session, Source,
};
use std::rc::Rc;

fn run(code: &str) -> Result<Option<Array>, Error> {
    let result = Session::new().eval_source(Source::new("test", code));
    match result.error { Some(e) => Err(e), None => Ok(result.value) }
}

fn number(n: f64) -> Element { Element::Number(n.try_into().unwrap()) }
fn vector(values: &[f64]) -> Array { Array::from_parts(vec![values.len()], values.iter().copied().map(number).collect(), number(0.0)).unwrap() }

fn exact(n: i64, d: i64) -> Array { Array::scalar(num_rational::BigRational::new(n.into(), d.into())).unwrap() }

#[test]
fn complex_arithmetic_and_roundtrips() {
    // Basic arithmetic expectations calculated independently. APL notation/conjugation:
    // https://docs.dyalog.com/20.0/programming-reference-guide/introduction/complex-numbers/
    for (code, re, im) in [
        ("1J2", 1.0, 2.0),
        ("¯.5j2E¯1", -0.5, 0.2),
        ("1E2J¯4E¯1", 100.0, -0.4),
        ("1J¯0", 1.0, 0.0),
        ("¯0J2", 0.0, 2.0),
        ("1J2+3J4", 4.0, 6.0),
        ("1J2-3J4", -2.0, -2.0),
        ("1J2×3J4", -5.0, 10.0),
        ("1J2×1J¯2", 5.0, 0.0),
        ("1J2÷3J4", 0.44, 0.08),
        ("+1J2", 1.0, -2.0),
        ("-1J2", -1.0, -2.0),
        ("×3J4", 0.6, 0.8),
        ("÷1J2", 0.2, -0.4),
        ("1r2+1J2", 1.5, 2.0),
        ("1J2-1r2", 0.5, 2.0),
        ("2x÷0J1", 0.0, -2.0),
        ("1J2÷2x", 0.5, 1.0),
        ("sum←+/ ⋄ sum 1J2 3J4", 4.0, 6.0),
        ("-/1J2 3J4 5J6", 3.0, 4.0),
        ("f←{⍵×+⍵} ⋄ f 3J4", 25.0, 0.0),
        ("0J0÷0J0", 1.0, 0.0),
        ("1E300J1E300÷1E300J1E300", 1.0, 0.0),
        ("1E¯320J1E¯320÷1E¯320J1E¯320", 1.0, 0.0),
        ("1E308J1E308÷1J1", 1e308, 0.0),
        ("1.7E308÷0.5J0.5", 1.7e308, -1.7e308),
    ] {
        let a = run(code).unwrap().unwrap();
        let expected = if im == 0.0 { Array::scalar(re).unwrap() } else { Array::scalar(num_complex::Complex64::new(re, im)).unwrap() };
        assert_eq!(a, expected, "{code}");
        assert_eq!(run(&a.to_string()).unwrap().unwrap(), a, "display round-trip: {code}");
    }
    assert_eq!(run("1J2×1J¯2").unwrap().unwrap().to_string(), "5");
    assert_eq!(run("¯0J2").unwrap().unwrap().to_string(), "0J2");
    let a = run("1x 0.5 1J2").unwrap().unwrap();
    assert_eq!(a.data(), &[exact(1, 1).data()[0].clone(), number(0.5), Element::Number(num_complex::Complex64::new(1.0, 2.0).try_into().unwrap())]);
    for code in ["0/1J2", "1J2+0/1x", "+/0/1J2"] { assert_eq!(run(code).unwrap().unwrap().prototype(), &number(0.0)); }
    assert_eq!(run("×/0/1J2").unwrap().unwrap(), Array::scalar(1.0).unwrap());
    assert_eq!(run("⍳3J0").unwrap().unwrap(), vector(&[1.0, 2.0, 3.0]));
}

#[test]
fn complex_comparison_errors_and_recovery() {
    // Documentation-derived magnitude tolerance, not separate component comparisons:
    // https://docs.dyalog.com/20.0/language-reference-guide/primitive-functions/equal-to/
    for (x, y, equal) in [
        ("1J1", "1J1.000000000000012", true),
        ("1J1", "1J1.00000000000002", false),
        ("1x", "1J5E¯15", true),
        ("1", "1J5E¯14", false),
        ("0", "0J1E¯100", false),
        ("1.7E308J1.7E308", "1.7E308J1.7E308", true),
        ("1.7E308J1.7E308", "¯1.7E308J¯1.7E308", false),
    ] {
        for (op, expected) in [("=", equal), ("≠", !equal)] {
            for code in [format!("{x}{op}{y}"), format!("{y}{op}{x}")] {
                assert_eq!(run(&code).unwrap().unwrap(), Array::scalar(u8::from(expected) as f64).unwrap(), "{code}");
            }
        }
    }
    for code in ["1J", "1J¯", "1J2E¯", "1J2x", "1xJ2", "1r2J3", "1J2J3"] { assert_eq!(run(code).unwrap_err().kind, Syntax, "{code}"); }
    for code in ["1J1E309", "1E309J1", "1E308J1E308+1E308J1E308", "1J2÷0", "÷0J0", "⍳1J1E¯15", "1J2/3", "1J2<1J2", "1J2≤2", "2>1J2", "2≥1J2"] {
        assert_eq!(run(code).unwrap_err().kind, Domain, "{code}");
    }
    assert_eq!(run(&format!("1{}x+0J1", "0".repeat(400))).unwrap_err().kind, Domain);
    assert_eq!(Array::scalar(num_complex::Complex64::new(0.0, f64::INFINITY)), Err(Domain));
    let mut s = Session::new();
    let failed = s.eval("⎕←1J2 ⋄ 1J2÷0");
    assert_eq!(failed.output, ["1J2"]);
    assert_eq!(failed.error.unwrap().kind, Domain);
    assert_eq!(s.eval("1J2+3J4").output, ["4J6"]);
}

#[test]
fn exact_literals_arithmetic_and_roundtrips() {
    // miniapl's explicit exact-number extension, not Dyalog reference cases.
    for (code, n, d) in [
        ("42x", 42, 1),
        ("42r1", 42, 1),
        ("2r4", 1, 2),
        ("1r¯2", -1, 2),
        ("¯1r¯2", 1, 2),
        ("¯0x", 0, 1),
        ("1x÷3x", 1, 3),
        ("1r3+1r6", 1, 2),
        ("6x÷3x", 2, 1),
        ("1r3-1r6", 1, 6),
        ("2r3×3r4", 1, 2),
        ("+2r3", 2, 3),
        ("-2r3", -2, 3),
        ("×¯2r3", -1, 1),
        ("×0x", 0, 1),
        ("÷2r3", 3, 2),
        ("÷¯2r3", -3, 2),
        ("2r3÷¯1r3", -2, 1),
        ("0x÷0x", 1, 1),
        ("+/1r3 1r6", 1, 2),
        ("sum←+/ ⋄ sum 1r3 1r6", 1, 2),
        ("add←{⍺+⍵} ⋄ add/1r3 1r6", 1, 2),
        ("-/1x 2x 3x", 2, 1),
        ("9007199254740993x-9007199254740992x", 1, 1),
    ] {
        let a = run(code).unwrap().unwrap();
        assert_eq!(a, exact(n, d), "{code}");
        assert_eq!(run(&a.to_string()).unwrap().unwrap(), a, "display round-trip: {code}");
    }
    assert_eq!(run("42r1").unwrap().unwrap().to_string(), "42x");
    for code in ["(1÷3)+(1÷6)", "1r3+1÷6", "1÷6+0", "1x÷2", "1÷2x"] {
        let a = run(code).unwrap().unwrap();
        assert!(a.as_number().unwrap().as_float().is_some(), "{code}");
    }
    assert_eq!(run("1x÷2").unwrap().unwrap(), Array::scalar(0.5).unwrap());
    assert_eq!(run("(1÷3)+(1÷6)").unwrap().unwrap(), Array::scalar(0.5).unwrap());
    assert_eq!(run("1r4+0.5").unwrap().unwrap(), Array::scalar(0.75).unwrap());
    let huge = format!("1{}", "0".repeat(400));
    assert_eq!(run(&format!("{huge}x÷{huge}x")).unwrap().unwrap(), exact(1, 1));
    assert_eq!(run(&format!("{huge}x+0")).unwrap_err().kind, Domain);
    assert_eq!(run(&format!("{huge}1r{huge}0+0.5")).unwrap().unwrap(), Array::scalar(1.5).unwrap());
    for (code, kind) in [
        ("1r0", Domain),
        ("0r0", Domain),
        ("1x÷0x", Domain),
        ("÷0x", Domain),
        ("1r", Syntax),
        ("1r¯", Syntax),
        ("1.5x", Syntax),
        ("1E2x", Syntax),
        ("1r2.5", Syntax),
        ("1r2r3", Syntax),
        ("1x2", Syntax),
    ] { assert_eq!(run(code).unwrap_err().kind, kind, "{code}"); }
    let e = run("¯2+1r0").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "1r0");
}

#[test]
fn exact_arrays_prototypes_and_counts() {
    let a = run("9007199254740993x 0.5 1r3").unwrap().unwrap();
    assert_eq!(a.data()[0], exact(9007199254740993, 1).data()[0]);
    assert_eq!(a.data()[1], number(0.5));
    assert_eq!(a.data()[2], exact(1, 3).data()[0]);
    let empty = run("0x/1r3").unwrap().unwrap();
    assert_eq!(empty.shape(), &[0]);
    assert_eq!(empty.prototype(), exact(0, 1).prototype());
    assert_eq!(run("+/0/1r3").unwrap().unwrap(), exact(0, 1));
    assert_eq!(run("×/0/1r3").unwrap().unwrap(), exact(1, 1));
    assert_eq!(run("1x+0/1r3").unwrap().unwrap(), empty);
    assert_eq!(run("1+0/1r3").unwrap().unwrap().prototype(), &number(0.0));
    assert_eq!(run("⍳3x").unwrap().unwrap(), vector(&[1.0, 2.0, 3.0]));
    assert_eq!(run("2x/1r3").unwrap().unwrap().data(), vec![exact(1, 3).data()[0].clone(); 2]);
    for (code, kind) in [("⍳3r2", Domain), ("⍳¯1x", Domain), ("⍳1000001x", Limit), ("⍳999999999999999999999x", Limit), ("1r2/3", Domain)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
    let invalid = num_rational::BigRational::new_raw(1.into(), 0.into());
    assert_eq!(Array::scalar(invalid), Err(Domain));
    let raw = num_rational::BigRational::new_raw(2.into(), (-4).into());
    assert_eq!(Array::scalar(raw).unwrap(), exact(-1, 2));
}

#[test]
fn exact_and_tolerant_comparisons() {
    // Approximate cases follow Dyalog 20 ⎕CT=1E¯14 (documentation-derived):
    // https://docs.dyalog.com/20.0/language-reference-guide/system-functions/ct/
    // Exact/exact and mixed promotion are miniapl's explicit extension.
    for (x, y, equal, less) in [
        ("0.3", "(0.1+0.2)", true, false),
        ("(0.1+0.2)", "0.3", true, false),
        ("¯0.3", "(¯0.1-0.2)", true, false),
        ("1", "(1+5E¯15)", true, false),
        ("1", "(1+5E¯14)", false, true),
        ("1E8", "(1E8+5E¯7)", true, false),
        ("1E8", "(1E8+5E¯5)", false, true),
        ("0", "1E¯100", false, true),
        ("1E¯100", "0", false, false),
        ("1E308", "¯1E308", false, false),
        ("1r3", "2r6", true, false),
        ("1r3", "1r2", false, true),
        ("9007199254740993x", "9007199254740992x", false, false),
        ("1x", "1000000000000001r1000000000000000", false, true),
        ("1", "1000000000000001r1000000000000000", true, false),
        ("1r3", "(1÷3)", true, false),
    ] {
        for (op, expected) in [("=", equal), ("≠", !equal), ("<", less), ("≤", less || equal), (">", !less && !equal), ("≥", !less)] {
            let code = format!("{x}{op}{y}");
            assert_eq!(run(&code).unwrap().unwrap(), Array::scalar(u8::from(expected) as f64).unwrap(), "{code}");
        }
    }
    assert_eq!(run("0.3=0.3 (0.1+0.2) 0.4").unwrap().unwrap(), vector(&[1.0, 1.0, 0.0]));
    assert_eq!(run("1 2≤2 1").unwrap().unwrap(), vector(&[1.0, 0.0]));
    assert_eq!(run("(1=1+8E¯15)((1+8E¯15)=1+16E¯15)(1=1+16E¯15)").unwrap().unwrap(), vector(&[1.0, 1.0, 0.0]));
    assert_eq!(run("1x=0/1x").unwrap().unwrap().prototype(), &number(0.0));
    assert_eq!(run("=⍳0").unwrap_err().kind, Unsupported);
}

#[test]
fn scalar_apl() {
    for (code, expected) in [
        ("2×3+4", 14.0),
        ("(2×3)+4", 10.0),
        ("10-3-2", 9.0),
        ("¯2+5", 3.0),
        ("2×-3+4", -14.0),
        ("2+-3", -1.0),
        ("--2", 2.0),
        ("8÷4÷2", 4.0),
        ("+¯2", -2.0),
        ("×¯9", -1.0),
        ("×0", 0.0),
        ("×3", 1.0),
        ("÷4", 0.25),
        ("1.5+.5", 2.0),
        ("¯.5×2", -1.0),
        ("1E¯2+2e¯2", 0.03),
        ("2.", 2.0),
        ("(+)3", 3.0),
        ("2(-)3", -1.0),
        ("(2×\n3) + 4 ⍝ comment )", 10.0),
        // Dyalog 20, ⎕DIV=0; documentation-derived, not reference-interpreter executions:
        // https://docs.dyalog.com/20.0/language-reference-guide/system-functions/div/
        ("0÷0", 1.0),
        ("0÷¯0", 1.0),
        ("¯0", 0.0),
    ] {
        let a = run(code).unwrap().unwrap();
        assert_eq!(a, Array::scalar(expected).unwrap(), "{code}");
    }
    assert_eq!(run("⍝ only a comment").unwrap(), None);
    assert_eq!(run("").unwrap(), None);
    assert_eq!(run("¯0").unwrap().unwrap().to_string(), "0");
    assert_eq!(run("¯2").unwrap().unwrap().to_string(), "¯2");
}

#[test]
fn errors_and_evaluation_order() {
    for (code, kind) in [
        ("1÷0", Domain),
        ("÷0", Domain),
        ("1e309", Domain),
        ("1e308×2", Domain),
        ("1e", Syntax),
        ("1e-2", Syntax),
        ("¯", Syntax),
        (".", Syntax),
        ("2+", Syntax),
        ("+", Syntax),
        (")", Syntax),
        ("(2+3", Syntax),
        ("1 2+3 4 5", Length),
        ("'a'", Unsupported),
        ("⍳¯1", Domain),
        ("(+ -)", Unsupported),
    ] { assert_eq!(run(code).unwrap_err().kind, kind, "{code}"); }
    // The right argument fails before the parenthesized left argument is evaluated.
    let e = run("(1÷0)+(2×1e308)").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "×");
    let e = run("¯2+1÷0").unwrap_err();
    assert_eq!(e.span.range, 5..7); // UTF-8 bytes, not glyph indices.
    assert_eq!(e.to_string(), "DOMAIN ERROR: division by zero\n --> test:1:5\n¯2+1÷0\n    ^");
    let e = run("(2+\n 1÷0)").unwrap_err();
    assert!(e.to_string().contains("test:2:3\n 1÷0)\n  ^"));
    assert_eq!(run("2+2").unwrap().unwrap().as_number(), Some(&miniapl::Number::try_from(4.0).unwrap()));
}

#[test]
fn structural_completeness_and_source_lifetime() {
    assert!(matches!(parse(Source::new("test", "(2+⍝ )\n")), ParseStatus::Incomplete(_)));
    assert!(matches!(parse(Source::new("test", "(2+))")), ParseStatus::Invalid(_)));
    assert!(matches!(parse(Source::new("test", "('")), ParseStatus::Invalid(_)));
    let source = Source::new("old input", "(1÷0)");
    let weak = Rc::downgrade(&source);
    // Parsing a domain error is non-executing; the same retained syntax can be evaluated later.
    let ParseStatus::Complete(parsed) = parse(source.clone()) else { panic!("expected complete input") };
    drop(source);
    let e = Session::new().eval_parsed(&parsed).error.unwrap();
    drop(parsed);
    assert!(e.to_string().contains("old input:1:3"));
    assert!(weak.upgrade().is_some());
    drop(e);
    assert!(weak.upgrade().is_none());
    let deep = format!("{}1{}", "(".repeat(129), ")".repeat(129));
    assert_eq!(run(&deep).unwrap_err().kind, Limit);
    // Flat evaluation is iterative, not one Rust stack frame per function application.
    assert_eq!(run(&format!("{}1", "1+".repeat(10_000))).unwrap().unwrap().as_number(), Some(&miniapl::Number::try_from(10_001.0).unwrap()));
}

#[test]
fn array_invariants() {
    let scalar = Array::scalar(7.0).unwrap();
    let singleton = vector(&[7.0]);
    assert_ne!(scalar, singleton);
    assert!(scalar.shape().is_empty());
    assert_eq!(singleton.shape(), &[1]);
    assert_eq!(Array::new(vec![2, 2], vec![number(1.0); 3]), Err(Length));
    assert_eq!(Array::new(vec![], vec![]), Err(Length));
    assert_eq!(Array::new(vec![usize::MAX, 2], vec![number(1.0)]), Err(Limit));
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] { assert_eq!(Array::scalar(n), Err(Domain)); }
    let rows = Array::empty(vec![0, 3], number(0.0)).unwrap();
    let cols = Array::empty(vec![3, 0], number(0.0)).unwrap();
    assert_ne!(rows, cols);
    assert!(rows.data().is_empty());
    assert_eq!(rows.shape(), &[0, 3]);
    assert!(Array::empty(vec![usize::MAX, 2, 0], number(0.0)).is_ok());
    assert_eq!(Array::empty(vec![], number(0.0)), Err(Length));
    let text = Array::empty(vec![0, 3], Character('x')).unwrap();
    assert_ne!(rows, text);
    assert_eq!(text.prototype(), &Character(' '));
    let normalized = Array::new(vec![1], vec![Nested(scalar)]).unwrap();
    assert_eq!(normalized, singleton);
}

#[test]
fn nested_prototypes_and_value_semantics() {
    // Dyalog 20 prototype examples, documentation-derived:
    // https://docs.dyalog.com/20.0/programming-reference-guide/introduction/arrays/prototypes-and-fill-items/
    let nested = Array::new(vec![2], vec![Nested(vector(&[1.0, 2.0])), Nested(vector(&[3.0, 4.0, 5.0]))]).unwrap();
    assert_eq!(nested.prototype(), &Nested(vector(&[0.0, 0.0])));
    let empty = Array::empty(vec![0], nested.prototype().clone()).unwrap();
    assert_eq!(empty.prototype(), nested.prototype());
    let mixed = Array::new(vec![2], vec![number(88.0), Character('X')]).unwrap();
    let a = Array::new(vec![], vec![Nested(mixed)]).unwrap();
    let expected = Array::new(vec![2], vec![number(0.0), Character(' ')]).unwrap();
    assert_eq!(a.prototype(), &Nested(expected));
    let saved = nested.clone();
    let mut detached: Vec<Element> = nested.data().to_vec();
    detached[0] = number(9.0);
    let changed = Array::new(vec![2], detached).unwrap();
    assert_ne!(changed, saved);
    drop(nested);
    assert_eq!(saved.prototype(), &Nested(vector(&[0.0, 0.0])));
}

#[test]
fn persistent_arrays_and_functions() {
    let mut s = Session::new();
    let r = s.eval("v←⍳10");
    assert!(r.error.is_none());
    assert!(r.output.is_empty());
    assert_eq!(r.value.unwrap().shape(), &[10]);
    for (code, expected) in
        [("+/v", 55.0), ("f←+ ⋄ 2 f 3", 5.0), ("sum←+/ ⋄ sum 1 2 3", 6.0), ("f/1 2 3", 6.0), ("-/1 2 3", 2.0), ("+/⍳0", 0.0), ("×/⍳0", 1.0), ("+/7", 7.0)]
    {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap().as_number(), Some(&miniapl::Number::try_from(expected).unwrap()), "{code}");
    }
    for (code, expected) in
        [("1 2 3+10", vec![11.0, 12.0, 13.0]), ("10-1 2 3", vec![9.0, 8.0, 7.0]), ("1 2+3 4", vec![4.0, 6.0]), ("⍴v", vec![10.0]), (",7", vec![7.0])]
    {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&expected), "{code}");
    }
    assert_eq!(s.eval("a←v ⋄ v←0 ⋄ +/a").value.unwrap().as_number(), Some(&miniapl::Number::try_from(55.0).unwrap()));
    assert_eq!(s.eval("≢,7").value.unwrap().as_number(), Some(&miniapl::Number::try_from(1.0).unwrap()));
    assert_eq!(s.eval("⍴7").value.unwrap().shape(), &[0]);
    assert_eq!(s.eval("2+⍳0").value.unwrap().shape(), &[0]);
    for (code, kind) in [("⍳1.5", Domain), ("⍳1000001", Limit), ("missing", Value), ("-/⍳0", Unsupported)] {
        assert_eq!(s.eval(code).error.unwrap().kind, kind);
    }
}

#[test]
fn result_output_and_nonexecuting_parse() {
    let mut s = Session::new();
    assert_eq!(s.eval("x←2").value.unwrap().as_number(), Some(&miniapl::Number::try_from(2.0).unwrap()));
    let r = s.eval("3 ⋄ f←+");
    assert!(r.error.is_none());
    assert!(r.value.is_none());
    assert_eq!(r.output, ["3"]);
    let r = s.eval("⎕←7 ⋄ 1÷0");
    assert!(r.value.is_none());
    assert_eq!(r.output, ["7"]);
    assert_eq!(r.error.unwrap().kind, Domain);
    let r = s.eval("(⎕←1)+(⎕←2)");
    assert_eq!(r.output, ["2", "1", "3"]);
    for _ in 0..2 { assert!(matches!(parse(Source::new("check", "x←99 ⋄ ⎕←8")), ParseStatus::Complete(_))); }
    assert_eq!(s.eval("x").value.unwrap().as_number(), Some(&miniapl::Number::try_from(2.0).unwrap()));
    assert!(s.eval("").value.is_none());
    assert!(s.eval("2+2").error.is_none());
    let r = s.eval("x←5 ⋄ 1÷0");
    assert!(r.error.is_some());
    assert_eq!(s.eval("x").value.unwrap().as_number(), Some(&miniapl::Number::try_from(5.0).unwrap()));
    // No whole-input transaction.
}

#[test]
fn executing_binding_gate() {
    let mut s = Session::new();
    for (code, expected) in [
        ("add←{⍺+⍵} ⋄ add/1 2 3", 6.0),
        ("avg←+/÷≢ ⋄ avg 2 4 9", 5.0),
        ("apply←{⍺⍺ ⍵} ⋄ (-apply)3", -3.0),
        ("offset←{⍺⍺+⍵} ⋄ (2 offset)3", 5.0),
        ("inc←{later ⍵} ⋄ later←{1+⍵} ⋄ inc 4", 5.0),
        ("x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller 0", 10.0),
        ("total←add/ ⋄ total 1 2 3", 6.0),
        ("{2×⍵}3", 6.0),
    ] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap().as_number(), Some(&miniapl::Number::try_from(expected).unwrap()), "{code}");
    }
    for code in ["1 0 1/2 4 6", "rep←/ ⋄ 1 0 1 rep 2 4 6"] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&[2.0, 6.0]));
    }
    assert_eq!(s.eval("f←{⎕←1 ⋄ ⍵} ⋄ h←{⎕←2 ⋄ ⍵} ⋄ t←f+h ⋄ t 3").output, ["2", "1", "6"]);
    let r = s.eval("bad←{local←99 ⋄ 1÷0} ⋄ bad 0");
    assert_eq!(r.error.unwrap().kind, Domain);
    assert_eq!(s.eval("local").error.unwrap().kind, Value);
    assert_eq!(s.eval("x").value.unwrap().as_number(), Some(&miniapl::Number::try_from(10.0).unwrap()));
    assert_eq!(s.eval("loop←{∇⍵} ⋄ loop 0").error.unwrap().kind, Limit);
    assert_eq!(s.eval("2+2").value.unwrap().as_number(), Some(&miniapl::Number::try_from(4.0).unwrap()));
    assert_eq!(s.eval("{⍵←1 ⋄ ⍵}2").error.unwrap().kind, Syntax);
    assert_eq!(s.eval("outer←{inner←{⍵} ⋄ inner ⍵} ⋄ outer 1").value.unwrap(), Array::scalar(1.0).unwrap());
}

#[test]
fn definitions_retain_only_needed_sources() {
    let source = Source::new("definition.apl", "bad←{1÷⍵}");
    let weak = Rc::downgrade(&source);
    let mut s = Session::new();
    assert!(s.eval_source(source).error.is_none());
    assert!(weak.upgrade().is_some());
    let error = s.eval("bad 0").error.unwrap();
    assert_eq!(error.span.source.name, "definition.apl");
    assert_eq!(&error.span.source.text[error.span.range.clone()], "÷");
    assert!(s.eval("bad←+").error.is_none());
    assert!(weak.upgrade().is_some());
    drop(error);
    assert!(weak.upgrade().is_none());
    for _ in 0..2 { assert!(matches!(parse(Source::new("check", "f←{⎕←1\n⍵}")), ParseStatus::Complete(_))); }
    assert!(s.eval("f").error.is_some());
    assert!(matches!(parse(Source::new("check", "f←{⍝ }\n")), ParseStatus::Incomplete(_)));
}

#[test]
fn hybrid_categories_and_singleton_replicate() {
    // Dyalog 20 binding-strength and replicate documentation; not reference executions.
    let mut s = Session::new();
    for code in ["r←/ ⋄ +r 1 2 3", "+(/)1 2 3", "r←(/) ⋄ sum←+r ⋄ sum 1 2 3", "r←/ ⋄ alias←r ⋄ +(alias)1 2 3"] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), Array::scalar(6.0).unwrap());
    }
    for (code, expected) in [("1 0 1 r 2 4 6", vec![2.0, 6.0]), ("(,2)/3 4", vec![3.0, 3.0, 4.0, 4.0]), ("1 0 1/,3", vec![3.0, 3.0])] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&expected));
    }
    let e = s.eval("op←{⍵⍵ ⍵}").error.unwrap();
    assert_eq!(e.kind, Unsupported);
    assert!(e.message.contains("dyadic"));
    assert_eq!(&e.span.source.text[e.span.range], "{⍵⍵ ⍵}");
    assert!(s.eval("outer←{inner←{⍵⍵ ⍵} ⋄ 1}").error.is_none());
    assert!(s.eval("outer 0").error.unwrap().message.contains("dyadic"));
}

#[test]
fn singleton_agreement_and_empty_counts() {
    // Dyalog 20 primitive-functions-by-category: singleton scalar extension, not NumPy broadcasting.
    for (code, values) in [
        ("(,2)+3 4", vec![5.0, 6.0]),
        ("3 4+,2", vec![5.0, 6.0]),
        ("2+,3", vec![5.0]),
        ("(,2)/⍳0", vec![]),
        ("(⍳0)/,3", vec![]),
        ("(,2)+⍳0", vec![]),
        ("1000001/⍳0", vec![]),
    ] { assert_eq!(run(code).unwrap().unwrap(), vector(&values), "{code}"); }
    for (code, kind) in [("0.5/⍳0", Domain), ("2 3/1 2 3", Length), ("1000001/1", Limit), ("99999999999999999999x/1", Limit)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
}

#[test]
fn binder_limits_and_single_execution() {
    let mut s = Session::new();
    assert_eq!(s.eval("x←1 ⋄ (⎕←x)+(⎕←(x←2))").output, ["2", "2", "4"]);
    assert_eq!(s.eval("apply←{⍺⍺ ⍵} ⋄ f←{⎕←⍵ ⋄ ⍵} ⋄ (f apply)/1 2 3").output, ["3", "3", "3"]);
    assert_eq!(s.eval("offset←{+/⍺⍺+⍵} ⋄ (1 2 offset)3").value.unwrap(), Array::scalar(9.0).unwrap());
    assert_eq!(s.eval(&format!("{}7", "a←".repeat(10_000))).value.unwrap(), Array::scalar(7.0).unwrap());
    assert_eq!(s.eval(&format!("+{}1", "/".repeat(10_000))).error.unwrap().kind, Limit);
    assert_eq!(s.eval(&format!("f←+ ⋄ {} f 0", "f←f+f ⋄ ".repeat(127))).error.unwrap().kind, Limit);
    assert_eq!(s.eval("2+2").value.unwrap(), Array::scalar(4.0).unwrap());
}

#[test]
fn diagnostic_width_and_call_context() {
    let mut s = Session::new();
    let error = s.eval("界←1 ⋄\t界÷0").error.unwrap();
    assert_eq!(error.to_string(), "DOMAIN ERROR: division by zero\n --> <input>:1:11\n界←1 ⋄  界÷0\n          ^");
    assert!(s.eval_source(Source::new("old.apl", "bad←{1÷⍵} ⋄ outer←{bad ⍵}")).error.is_none());
    let e = s.eval("outer 0").error.unwrap();
    assert_eq!(&e.span.source.text[e.span.range.clone()], "÷");
    assert_eq!(e.calls.iter().map(|s| &s.source.text[s.range.clone()]).collect::<Vec<_>>(), ["bad", "outer"]);
    assert!(e.to_string().contains("called from old.apl:"));
    let empty = Source::new("empty.apl", "f←{}");
    let weak = Rc::downgrade(&empty);
    s.eval_source(empty);
    let e = s.eval("f 0").error.unwrap();
    assert_eq!(&e.span.source.text[e.span.range.clone()], "{}");
    s.eval("f←+");
    drop(e);
    assert!(weak.upgrade().is_none());
}

#[test]
fn lexical_frames_recursion_and_guard_rollback() {
    // Executed in Dyalog 20.0.53963.0, ⎕IO=1, ⎕CT=1E¯14, ⎕DIV=0.
    // Numeric labels avoid depending on the as-yet unimplemented character syntax.
    let mut s = Session::new();
    for (code, expected) in [
        ("outer←{x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller 0} ⋄ outer 0", 10.0),
        ("outer←{x←2 ⋄ f←{x+⍵} ⋄ apply←{⍺⍺ ⍵} ⋄ g←f apply ⋄ x←3 ⋄ g 4} ⋄ outer 0", 7.0),
        ("outer←{offset←{⍺⍺+⍵} ⋄ (2 offset)3} ⋄ outer 0", 5.0),
        ("offset←{⍺⍺+⍵} ⋄ a←2 ⋄ kept←a offset ⋄ a←9 ⋄ kept 3", 5.0),
        ("fact←{⍵=0:1 ⋄ ⍵×∇⍵-1} ⋄ fact 6", 720.0),
        ("outer←{even←{⍵=0:1 ⋄ odd ⍵-1} ⋄ odd←{⍵=0:0 ⋄ even ⍵-1} ⋄ even ⍵} ⋄ outer 8", 1.0),
        ("bad←{1÷⍵} ⋄ guarded←{x←10 ⋄ 0::x ⋄ x←20 ⋄ bad ⍵} ⋄ guarded 0", 10.0),
        ("temp←9 ⋄ guarded←{0::temp ⋄ temp←20 ⋄ 1÷⍵} ⋄ guarded 0", 9.0),
        ("guarded←{x←10 ⋄ (0×(x←20))::x ⋄ x←30 ⋄ 1÷⍵} ⋄ guarded 0", 20.0),
        ("guarded←{0::7 ⋄ 0::1÷0 ⋄ 1÷⍵} ⋄ guarded 0", 7.0),
    ] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), Array::scalar(expected).unwrap(), "{code}");
    }
    let r = s.eval("guarded←{0::7 ⋄ ⎕←2 ⋄ 1÷⍵} ⋄ guarded 0");
    assert_eq!(r.output, ["2", "7"]); // Output is not transactional.
    for (code, kind) in [
        ("{0::1÷0 ⋄ 1÷⍵}0", Domain),
        ("{0::fresh ⋄ fresh←1 ⋄ 1÷⍵}0", Value),
        ("{2:1 ⋄ 0}0", Domain),
        ("{11::1 ⋄ 1÷⍵}0", Unsupported),
        ("{x←2 ⋄ {x+⍵}}0", Syntax),
        ("{1:+ ⋄ 0}0", Syntax),
    ] { assert_eq!(s.eval(code).error.unwrap().kind, kind, "{code}"); }
    assert!(matches!(parse(Source::new("guard", "f←{0::⎕←1}")), ParseStatus::Complete(_)));
    assert_eq!(s.eval("2+2").value.unwrap(), Array::scalar(4.0).unwrap());
}
