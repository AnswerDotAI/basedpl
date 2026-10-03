use basedpl::{parse, Error, ErrorKind, ErrorKind::*, EvalOptions, ParseStatus, Session, Source, Value as AplValue, Value::Character};
use std::sync::Arc;

fn run(code: &str) -> Result<Option<AplValue>, Error> {
    let result = Session::new().eval_source(Source::new("test", code), EvalOptions::default());
    match result.error { Some(e) => Err(e), None => Ok(result.value) }
}

fn number(n: impl TryInto<basedpl::Number>) -> AplValue { AplValue::number(n).unwrap() }
fn vector(values: &[f64]) -> AplValue { AplValue::from_parts(vec![values.len()], values.iter().copied().map(number).collect(), number(0.0)).unwrap() }

#[track_caller]
fn check_in(session: &mut Session, code: &str, expected: AplValue) {
    let result = session.eval(code);
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    assert_same(result.value, &expected);
}

#[track_caller]
fn check(code: &str, expected: AplValue) { check_in(&mut Session::new(), code, expected); }

/// Asserts that `actual` is exactly `expected`, as `Value::same` compares them.
#[track_caller]
fn assert_same(actual: impl Into<Option<AplValue>>, expected: &AplValue) {
    let actual = actual.into();
    assert!(actual.as_ref().is_some_and(|a| a.same(expected)), "{actual:?} is not {expected:?}");
}

#[track_caller]
fn equiv_in(session: &mut Session, code: &str, expected: &str) { check_in(session, code, run(expected).unwrap().expect("expected an array")); }

#[track_caller]
fn equiv(code: &str, expected: &str) { equiv_in(&mut Session::new(), code, expected); }

#[test]
fn language_examples() {
    let mut failures = Vec::new();
    let page = std::env::var("BASEDPL_PAGE").ok();
    for dir in ["nbs", "nbs/glyphs"] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|ext| ext != "qmd") { continue; }
            if page.as_deref().is_some_and(|p| !path.to_string_lossy().contains(p)) { continue; }
            let text = std::fs::read_to_string(&path).unwrap();
            let mut fenced = false;
            for line in text.lines() {
                if line.starts_with("```") {
                    fenced = !fenced;
                    continue;
                }
                if fenced { continue; }
                for link in line.split("](").skip(1) {
                    let target = link.split(')').next().unwrap().split('#').next().unwrap();
                    if !target.is_empty() && !target.contains("://") {
                        assert!(path.parent().unwrap().join(target).exists(), "{}: missing {target}", path.display());
                    }
                }
            }
            let mut session = None;
            let mut code = String::new();
            for (line, text) in text.lines().enumerate() {
                if text == "```bpl" {
                    session = Some(Session::new());
                    continue;
                }
                let Some(bpl) = session.as_mut() else { continue; };
                let closing = text.starts_with("```");
                let (source, expected) = text.split_once(" ⍝ ").map_or((text, None), |(c, e)| (c, Some(e)));
                if !closing {
                    code.push_str(source);
                    code.push('\n');
                }
                if expected.is_none() && !closing { continue; }
                let result = bpl.eval(&code);
                let actual = match result.error { Some(e) => Err(e), None => Ok(result.value) };
                if let Some(expected) = expected {
                    match (actual, run(expected)) {
                        (Ok(Some(actual)), Ok(Some(expected))) if basedpl::reference::difference(&actual, &expected, 1e-13, 1e-13, true).is_none() => (),
                        pair => failures.push(format!("{}:{}: {code}\n{pair:?}", path.display(), line + 1)),
                    }
                }
                else if let Err(error) = actual { failures.push(format!("{}:{}: {error}", path.display(), line + 1)); }
                code.clear();
                if closing { session = None; }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

macro_rules! equiv_in {
    ($session:expr; $($code:expr => $expected:expr),* $(,)?) => {{
        let session = $session;
        $(equiv_in(session, $code, $expected);)*
    }};
}

#[track_caller]
fn fails_in(session: &mut Session, kind: ErrorKind, codes: &[&str]) {
    for code in codes {
        let result = session.eval(code);
        match result.error {
            Some(error) => assert_eq!(error.kind, kind, "{code}: {error}"),
            None => panic!("{code}: expected {kind:?}, got {:?}", result.value),
        }
    }
}

#[test]
fn explicit_output_without_echo() {
    let mut s = Session::new();
    let quiet = || basedpl::EvalOptions { echo: false, ..basedpl::EvalOptions::default() };
    let code = r#"1 ⋄ ⎕←2 ⋄ ⍎"3 ⋄ ⎕←4 ⋄ 5" ⋄ 6"#;
    let r = s.eval_with(code, quiet());
    assert!(r.error.is_none());
    assert_eq!(r.output_text(), ["2", "4"]);
    assert_same(r.value, &number(6.0));
    assert_eq!(s.eval(code).output_text(), ["1", "2", "3", "4", "5", "6"]);
    for code in ["x←7", "+", "f←{⎕←⍵ ⋄ ⍵+1} ⋄ f 8"] {
        let r = s.eval_with(code, quiet());
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.output_text(), if code.starts_with("f←") { vec!["8"] } else { vec![] });
    }
    let r = s.eval_with("⎕←9 ⋄ 1÷'a'", quiet());
    assert_eq!(r.error.as_ref().unwrap().kind, Domain);
    assert_eq!(r.output_text(), ["9"]);
    for code in ["]Display 1 2", "]help +", "]help f -source"] { assert!(!s.eval_with(code, quiet()).output_text().is_empty()); }
    let streamed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events = streamed.clone();
    let output =
        Arc::new(move |output: &basedpl::Output| events.lock().unwrap().push((matches!(output.kind, basedpl::OutputKind::Explicit), output.text().to_owned())));
    let r = s.eval_with("1 ⋄ ⎕←2 ⋄ 1÷'a'", basedpl::EvalOptions { output: Some(output), ..basedpl::EvalOptions::default() });
    assert_eq!(r.error.as_ref().unwrap().kind, Domain);
    assert!(r.output_text().is_empty());
    assert_eq!(*streamed.lock().unwrap(), [(false, "1".into()), (true, "2".into())]);
    assert_eq!(s.eval("3").output_text(), ["3"]);
    equiv_in(&mut s, "x", "7");
}

#[test]
fn calls_with_array_arguments() {
    let mut s = Session::new();
    s.eval("x←42 ⋄ mean←+/÷≢ ⋄ bad←{⎕←⍵ ⋄ 1÷⍵}");
    for (function, codes, expected) in [
        ("mean", vec!["1 2 3"], "2"),
        ("-", vec!["10ₓ", "[1 2]ₓ"], "[9 8]ₓ"),
        ("#⍠0", vec!["1 0", "2 2⍴⍳4"], "[0 1 ⋄]"),
        ("⊢", vec![r#"[1r3 2ₓ;"ab";0 3⍴0ₓ]"#], r#"[1r3 2ₓ;"ab";0 3⍴0ₓ]"#),
        ("{k←⍵ ⋄ {k+⍵}⍵}", vec!["3ₓ"], "6ₓ"),
        ("{x←⍵}", vec!["7"], "7"),
    ] {
        let args: Vec<_> = codes.iter().map(|c| run(c).unwrap().unwrap()).collect();
        let r = s.call_with(function, &args, basedpl::EvalOptions { echo: false, ..basedpl::EvalOptions::default() });
        assert!(r.error.is_none(), "{function}: {:?}", r.error);
        assert!(r.output_text().is_empty());
        assert_same(r.value, &run(expected).unwrap().unwrap());
    }
    assert_eq!(s.call("+", &[number(3.0)]).output_text(), ["3"]);
    let r = s.call("{}", &[number(3.0)]);
    assert!(r.value.is_none() && r.error.is_none() && r.output_text().is_empty());
    for (function, args, kind) in [
        ("+", vec![], Length),
        ("+", vec![number(1.0); 3], Length),
        ("", vec![number(1.0)], Syntax),
        ("1", vec![number(1.0)], Syntax),
        ("¨", vec![number(1.0)], Syntax),
        ("+ ⋄ -", vec![number(1.0)], Syntax),
    ] { assert_eq!(s.call(function, &args).error.unwrap().kind, kind); }
    let r = s.call("bad", &[AplValue::Character('a')]);
    assert_eq!(r.output_text(), ["'a'"]);
    let e = r.error.as_ref().unwrap();
    assert_eq!(e.kind, Domain);
    assert_eq!(e.calls.last().unwrap().source.text, "bad");
    assert!(e.span.source.text.contains("bad←"));
    let r = s.call_with("{∇⍵}", &[number(0.0)], basedpl::EvalOptions { timeout: Some(std::time::Duration::ZERO), ..basedpl::EvalOptions::default() });
    assert_eq!(r.error.as_ref().unwrap().kind, Timeout);
    equiv_in(&mut s, "x", "42");
}

#[test]
fn cancellation_preserves_session_and_unwinds_calls() {
    use std::time::Duration;
    let mut s = Session::new();
    let r = s.eval_timeout("keep←42 ⋄ ⎕←7 ⋄ {0::99 ⋄ (+⍣{0})⍵}0", Duration::from_millis(10));
    assert_eq!(r.error.as_ref().unwrap().kind, Timeout);
    assert_eq!(r.output_text(), ["7"]);
    equiv_in(&mut s, "keep+1", "43");
    s.set("u", AplValue::floats(vec![1_000_000], (0..1_000_000).map(f64::from).collect()).unwrap()).unwrap();
    assert_eq!(s.eval_timeout("∪u", Duration::from_millis(2)).error.unwrap().kind, Timeout);
    assert_eq!(s.eval_timeout("⍭1000000000000ₓ", Duration::from_millis(2)).error.unwrap().kind, Timeout);
    let interrupt = basedpl::InterruptHandle::default();
    let handle = interrupt.clone();
    let cancel = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        handle.interrupt();
    });
    let r = s.eval_with("{∇⍵}0", basedpl::EvalOptions { interrupt, timeout: Some(Duration::from_secs(2)), ..basedpl::EvalOptions::default() });
    cancel.join().unwrap();
    assert_eq!(r.error.as_ref().unwrap().kind, Interrupt);
    equiv_in(&mut s, "keep", "42");
}

#[test]
fn boxed_display_and_function_trees() {
    let mut s = Session::new();
    assert!(s.eval(r#"•prefs ["box":$t "trees":$t "fns":$t]"#).error.is_none());
    assert_eq!(s.eval("⊂4ₓ ⋄ ⊂⊂4ₓ ⋄ ⊂¨[0 1]ₓ").output_text(), ["⊂4ₓ", "⊂⊂4ₓ", "┌→────────┐\n│ ⊂0ₓ ⊂1ₓ │\n└∊────────┘"]);
    let r = s.eval(r#"A←2 3 4⍴"DUCKSWANBIRDWORMCAKESEED" ⋄ ⊂⍠2 A"#);
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output_text(), ["┌→─────────────────────┐\n↓ ┌→───┐ ┌→───┐ ┌→───┐ │\n│ │DUCK│ │SWAN│ │BIRD│ │\n│ └────┘ └────┘ └────┘ │\n│ ┌→───┐ ┌→───┐ ┌→───┐ │\n│ │WORM│ │CAKE│ │SEED│ │\n│ └────┘ └────┘ └────┘ │\n└∊─────────────────────┘"]);
    assert_eq!(s.eval("⍬").output_text(), ["┌⊖┐\n│0│\n└~┘"]);
    assert_eq!(s.eval("0 3⍴0").output_text(), ["┌→────┐\n⌽0 0 0│\n└~────┘"]);
    assert_eq!(s.eval("1↓'a' 1 2").output_text(), ["┌→──┐\n│1 2│\n└+──┘"]);
    assert_eq!(s.eval("2 2⍴⍳4ₓ").output_text(), ["┌→──┐\n↓0 1│\n│2 3│\n└ₓ──┘"]);
    let f = s.eval("avg←+/÷≢ ⋄ avg");
    assert!(f.error.is_none() && f.value.is_none());
    assert_eq!(f.output_text(), ["fork\n├─ /\n│  └─ +\n├─ ÷\n└─ ≢"]);
    equiv_in(&mut s, "avg [1 2 4]ₓ", "7r3");
    let r = s.eval("{⎕←⍵ ⋄ ⍵}1 2");
    assert_eq!(r.output_text().len(), 2);
    assert_eq!(r.output_text()[0], r.output_text()[1]);
    assert!(s.eval(r#"•prefs ["fns":$f]"#).error.is_none());
    assert_eq!(s.eval("{⎕←⍵ ⋄ ⍵}1 2").output_text()[0], "1 2");
    equiv_in(&mut s, "⍕1 2", r#""1 2""#);
    assert!(s.eval(r#"•prefs ["box":$f]"#).error.is_none());
    assert_eq!(s.eval("1 2").output_text(), ["1 2"]);
    assert_eq!(s.eval("]Display ⎕←1 2").output_text(), ["1 2", "┌→──┐\n│1 2│\n└~──┘"]);
    assert_eq!(s.eval("1 2").output_text(), ["1 2"]);
    assert!(s.eval(r#"•prefs ["boks":$t]"#).error.is_some());
    assert!(s.eval(r#"p←•prefs ["limit":4 "edges":1]"#).error.is_none());
    assert_eq!(s.eval(r#"3 3⍴⍳9 ⋄ ⍳9 ⋄ ⎕←⍳9 ⋄ "abcdefghi" ⋄ ≢⍕⍳9"#).output_text(), ["0 … 2\n⋮ ⋱ ⋮\n6 … 8", "0 … 8", "0 1 2 3 4 5 6 7 8", "a…i", "17ₓ"]);
    assert_eq!(s.eval("1 2").output_text(), ["1 2"]);
    let wide = s.eval("]Display '界' 'a'").output_text().join("\n");
    assert!(wide.contains("界"));
}

#[test]
fn execute_source_and_session() {
    let mut s = Session::new();
    let r = s.eval(r#"a←⍎"1+1 ⋄ 2+2""#);
    assert_eq!(r.output_text(), ["2"]);
    assert_same(r.value, &number(4.));
    equiv_in(&mut s, "a", "4");
    let failed = s.eval(r#"⍎"⎕←7 ⋄ 1÷'a'""#);
    assert!(failed.value.is_none());
    assert_eq!(failed.output_text(), ["7"]);
    assert_eq!(failed.error.unwrap().span.source.text, "⎕←7 ⋄ 1÷'a'");
}

#[test]
fn numeric_constructors() {
    assert_eq!(AplValue::floats(vec![2], vec![1.]).err(), Some(Length));
    assert_eq!(AplValue::floats(vec![1], vec![-0.]).unwrap().as_floats().unwrap()[0].to_bits(), (-0f64).to_bits());
    assert_eq!(AplValue::number(num_rational::BigRational::new_raw(1.into(), 0.into())).err(), Some(Domain));
    assert_same(number(num_rational::BigRational::new_raw(2.into(), (-4).into())), &exact(-1, 2));
}

#[test]
fn exact_results_and_readback() {
    for code in ["0.1|0.3", "3|6.000000000000001"] { assert!(run(code).unwrap().unwrap().same(&number(0.0)), "{code}"); }
    for code in ["1j2÷3j4", "×3j4", "÷1j2", "¯.5j2E¯1", "1E2j¯4E¯1", "1.7E308÷0.5j0.5", "5E¯324 ¯1.2345678901234567E200 1E¯100j2E100"] {
        let a = run(code).unwrap().unwrap();
        assert!(run(&a.to_string()).unwrap().unwrap().same(&a), "{code}");
    }
    let huge = format!("1{}", "0".repeat(400));
    assert_same(run(&format!("{huge}ₓ÷{huge}ₓ")).unwrap().unwrap(), &exact(1, 1));
    assert_same(run(&format!("{huge}1r{huge}0+0.5")).unwrap().unwrap(), &number(1.5));
    assert_same(run(&format!("{huge}ₓ+0")).unwrap().unwrap(), &number(f64::INFINITY));
    assert_same(run(&format!("{huge}ₓ+0j1")).unwrap().unwrap(), &number(num_complex::Complex64::new(f64::INFINITY, 1.0)));
}
#[test]
fn array_literal_completeness() {
    for code in ["[1 2 ⋄", "(1 +", "[[{⍵}1;2] ⋄"] { assert!(matches!(parse(Source::new("partial", code)), ParseStatus::Incomplete(_))); }
    for code in ["[1 ⋄ 2)", "(1 ⋄ 2]", "[⋄]", "(1 ⋄ ⋄ 2)", "[1;2 ⋄ 3]"] {
        assert!(matches!(parse(Source::new("invalid", code)), ParseStatus::Invalid(_)));
    }
}

#[test]
fn shy_results_and_signal_messages() {
    let mut s = Session::new();
    for (code, output) in [
        ("f←{a←1} ⋄ f 0", vec![]),
        ("(f 0)", vec!["1"]),
        ("{2 ⋄ f ⍵}0", vec![]),
        ("{11::a←7 ⋄ 1÷⍵}'a'", vec![]),
        ("{f←{a←1} ⋄ (+f+)3}0", vec![]),
        ("{⎕←7}0", vec!["7"]),
    ] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.output_text(), output, "{code}");
    }
    for kind in [Syntax, Index, Rank, Length, Value, Limit, Domain] {
        let error = run(&format!(r#"•signal "{kind}""#)).unwrap_err();
        assert_eq!((error.kind, error.message.as_str()), (kind, "explicitly signalled"));
    }
}

fn exact(n: i64, d: i64) -> AplValue { AplValue::number(num_rational::BigRational::new(n.into(), d.into())).unwrap() }

#[test]
fn huge_shapes_count_exactly() {
    let mut session = Session::new();
    session.set("a", AplValue::empty(vec![usize::MAX, 0], number(0.)).unwrap()).unwrap();
    equiv_in(&mut session, "≢a", &format!("{}ₓ", usize::MAX));
    equiv_in(&mut session, "⍴a", &format!("[{} 0]ₓ", usize::MAX));
}

#[test]
fn errors_and_evaluation_order() {
    // The right argument fails before the parenthesized left argument is evaluated.
    let e = run("(÷'a')+(×'b')").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "×");
    let e = run("¯2+1÷'a'").unwrap_err();
    assert_eq!(e.span.range, 5..7); // UTF-8 bytes, not glyph indices.
    assert_eq!(e.to_string(), "DOMAIN ERROR: expected numeric elements\n --> test:1:5\n¯2+1÷'a'\n    ^");
    let e = run("(2\n 1÷'a')").unwrap_err();
    assert!(e.to_string().contains("test:2:3\n 1÷'a')\n  ^"));
    let e = run("¯2+1r0").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "1r0");
}

#[test]
fn structural_completeness_and_source_lifetime() {
    assert!(matches!(parse(Source::new("test", "(2+⍝ )\n")), ParseStatus::Incomplete(_)));
    assert!(matches!(parse(Source::new("test", "(2+))")), ParseStatus::Invalid(_)));
    assert!(matches!(parse(Source::new("test", "('")), ParseStatus::Invalid(_)));
    let source = Source::new("old input", "(1÷'a')");
    let weak = Arc::downgrade(&source);
    // Parsing a domain error is non-executing; the same retained syntax can be evaluated later.
    let ParseStatus::Complete(parsed) = parse(source.clone()) else { panic!("expected complete input") };
    drop(source);
    let e = Session::new().eval_parsed(&parsed, EvalOptions::default()).error.unwrap();
    drop(parsed);
    assert!(e.to_string().contains("old input:1:3"));
    assert!(weak.upgrade().is_some());
    drop(e);
    assert!(weak.upgrade().is_none());
    // Flat evaluation is iterative, not one Rust stack frame per function application.
    equiv(&format!("{}1", "1+".repeat(10_000)), "10001");
}

#[test]
fn based_values() {
    let n = number(3.0);
    let unit = n.enclose().unwrap();
    check("3", n.clone());
    check("⊂3", unit.clone());
    check("⊂⊂3", unit.enclose().unwrap());
    check(",3", vector(&[3.]));
    assert!(!n.same(&unit));
    assert!(!unit.same(&vector(&[3.])));
}

#[test]
fn array_invariants() {
    let seven = number(7.0);
    let singleton = vector(&[7.0]);
    assert!(!seven.same(&singleton));
    assert!(seven.shape().is_empty());
    assert_eq!(singleton.shape(), &[1]);
    assert_eq!(AplValue::new(vec![2, 2], vec![number(1.0); 3]).err(), Some(Length));
    assert_eq!(AplValue::new(vec![], vec![]).err(), Some(Length));
    assert_eq!(AplValue::new(vec![usize::MAX, 2], vec![number(1.0)]).err(), Some(Limit));
    let rows = AplValue::empty(vec![0, 3], number(0.0)).unwrap();
    let cols = AplValue::empty(vec![3, 0], number(0.0)).unwrap();
    assert!(!rows.same(&cols));
    assert!(rows.is_empty());
    assert_eq!(rows.shape(), &[0, 3]);
    assert!(AplValue::empty(vec![usize::MAX, 2, 0], number(0.0)).is_ok());
    assert_eq!(AplValue::empty(vec![], number(0.0)).err(), Some(Length));
    let text = AplValue::empty(vec![0, 3], Character('x')).unwrap();
    assert!(!rows.same(&text));
    assert_same(text.prototype(), &Character(' '));
    let normalized = AplValue::new(vec![1], vec![seven]).unwrap();
    assert_same(normalized, &singleton);
}

#[test]
fn nested_prototypes_and_value_semantics() {
    // Dyalog 20 prototype examples, documentation-derived:
    // https://docs.dyalog.com/20.0/programming-reference-guide/introduction/arrays/prototypes-and-fill-items/
    let nested = AplValue::new(vec![2], vec![vector(&[1.0, 2.0]), vector(&[3.0, 4.0, 5.0])]).unwrap();
    assert_same(nested.prototype(), &vector(&[0.0, 0.0]));
    let empty = AplValue::empty(vec![0], nested.prototype().clone()).unwrap();
    assert_same(empty.prototype(), &nested.prototype());
    let mixed = AplValue::new(vec![2], vec![number(88.0), Character('X')]).unwrap();
    let a = AplValue::new(vec![], vec![mixed]).unwrap();
    let expected = AplValue::new(vec![2], vec![number(0.0), Character(' ')]).unwrap();
    assert_same(a.prototype(), &expected);
    let saved = nested.clone();
    let mut detached: Vec<AplValue> = nested.elements().collect();
    detached[0] = number(9.0);
    let changed = AplValue::new(vec![2], detached).unwrap();
    assert!(!changed.same(&saved));
    drop(nested);
    assert_same(saved.prototype(), &vector(&[0.0, 0.0]));
}

#[test]
fn persistent_arrays_and_functions() {
    let mut s = Session::new();
    let r = s.eval("v←⍳10");
    assert!(r.error.is_none());
    assert!(r.output_text().is_empty());
    assert_eq!(r.value.unwrap().shape(), &[10]);
    equiv_in! { &mut s;
        "+/v" => "45",
        "f←+ ⋄ 2 f 3" => "5",
        "sum←+/ ⋄ sum 1 2 3" => "6",
        "f/1 2 3" => "6",
        "⍴v" => ",10ₓ",
        "a←v ⋄ v←0 ⋄ +/a" => "45",
    }
}

#[test]
fn result_output_and_nonexecuting_parse() {
    let mut s = Session::new();
    equiv_in(&mut s, "x←2", "2");
    let r = s.eval("3 ⋄ f←+");
    assert!(r.error.is_none());
    assert!(r.value.is_none());
    assert_eq!(r.output_text(), ["3"]);
    for _ in 0..2 { assert!(matches!(parse(Source::new("check", "x←99 ⋄ ⎕←8")), ParseStatus::Complete(_))); }
    equiv_in(&mut s, "x", "2");
    assert!(s.eval("").value.is_none());
    assert!(s.eval("2+2").error.is_none());
    let r = s.eval("x←5 ⋄ 1÷'a'");
    assert!(r.error.is_some());
    equiv_in(&mut s, "x", "5");
    // No whole-input transaction.
}

#[test]
fn binding_error_recovery() {
    let mut s = Session::new();
    s.eval("x←10");
    let r = s.eval("bad←{local←99 ⋄ 1÷'a'} ⋄ bad 0");
    assert_eq!(r.error.as_ref().unwrap().kind, Domain);
    fails_in(&mut s, Value, &["local"]);
    equiv_in(&mut s, "x", "10");
    fails_in(&mut s, Limit, &["loop←{1+∇⍵} ⋄ loop 0"]);
    equiv_in(&mut s, "2+2", "4");
    fails_in(&mut s, Syntax, &["⍵←1"]);
}

#[test]
fn definitions_retain_only_needed_sources() {
    let source = Source::new("definition.bpl", "bad←{1÷⍵}");
    let weak = Arc::downgrade(&source);
    let mut s = Session::new();
    assert!(s.eval_source(source, EvalOptions::default()).error.is_none());
    assert!(weak.upgrade().is_some());
    let error = s.eval("bad 'a'").error.unwrap();
    assert_eq!(error.span.source.name, "definition.bpl");
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
fn operator_categories_and_singleton_replicate() {
    // Dyalog 20: operators acquire their function operand before binding the next operator.

    // Dyalog 20 binding-strength and replicate documentation; not reference executions.
    let mut s = Session::new();
    for _ in 0..2 { equiv_in(&mut s, "op←{⍶⍵} ⋄ +op 3", "3"); }
    for code in ["r←/ ⋄ +r 1 2 3", "r←/ ⋄ sum←+r ⋄ sum 1 2 3", "r←/ ⋄ alias←r ⋄ +(alias)1 2 3"] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_same(r.value, &number(6.0));
    }
    for (code, expected) in [("(,2)#3 4", vec![3.0, 3.0, 4.0, 4.0]), ("1 0 1#,3", vec![3.0, 3.0])] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_same(r.value, &vector(&expected));
    }
}

#[test]
fn long_assignment_chains() { assert_same(Session::new().eval(&format!("{}7", "a←".repeat(10_000))).value, &number(7.0)); }

#[test]
fn diagnostic_width_and_call_context() {
    let mut s = Session::new();
    let error = s.eval("界←1 ⋄\t界÷'a'").error.unwrap();
    assert_eq!(error.to_string(), "DOMAIN ERROR: expected numeric elements\n --> <input>:1:11\n界←1 ⋄  界÷'a'\n          ^");
    assert!(s.eval_source(Source::new("old.bpl", "bad←{1÷⍵} ⋄ outer←{bad ⍵}"), EvalOptions::default()).error.is_none());
    let e = s.eval("outer 'a'").error.unwrap();
    assert_eq!(&e.span.source.text[e.span.range.clone()], "÷");
    assert_eq!(e.calls.iter().map(|s| &s.source.text[s.range.clone()]).collect::<Vec<_>>(), ["bad", "outer"]);
    assert!(e.to_string().contains("called from old.bpl:"));
    let deep = s.eval("down←{⍵=0?1÷'a';1+down ⍵-1} ⋄ down 100").error.unwrap();
    assert_eq!(deep.calls.len(), 101);
    assert_eq!(deep.to_string().matches("called from").count(), 6);
    assert!(deep.to_string().contains("95 more calls"));
    let empty = Source::new("empty.bpl", "f←{}");
    let weak = Arc::downgrade(&empty);
    s.eval_source(empty, EvalOptions::default());
    assert!(weak.upgrade().is_some());
    let result = s.eval("f 0");
    assert!(result.error.is_none() && result.value.is_none());
    s.eval("f←+");
    assert!(weak.upgrade().is_none());
}
