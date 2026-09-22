use basedpl::{
    parse, Error, ErrorKind,
    ErrorKind::*,
    ParseStatus, Session, Source, Value as AplValue,
    Value::{Character, Number},
};
use std::sync::Arc;

fn run(code: &str) -> Result<Option<AplValue>, Error> {
    let result = Session::new().eval_source(Source::new("test", code));
    match result.error { Some(e) => Err(e), None => Ok(result.value) }
}

fn number(n: f64) -> AplValue { AplValue::Number(n.try_into().unwrap()) }
fn scalar(n: impl TryInto<basedpl::Number>) -> AplValue { AplValue::scalar(n).unwrap() }
fn vector(values: &[f64]) -> AplValue { AplValue::from_parts(vec![values.len()], values.iter().copied().map(number).collect(), number(0.0)).unwrap() }
fn ints(values: &[i64]) -> AplValue { AplValue::integers(vec![values.len()], values.to_vec()).unwrap() }

#[track_caller]
fn check_in(session: &mut Session, code: &str, expected: AplValue) {
    let result = session.eval(code);
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    assert_eq!(result.value, Some(expected), "{code}");
}

#[track_caller]
fn check(code: &str, expected: AplValue) { check_in(&mut Session::new(), code, expected); }

#[track_caller]
fn equiv_in(session: &mut Session, code: &str, expected: &str) { check_in(session, code, run(expected).unwrap().expect("expected an array")); }

#[track_caller]
fn equiv(code: &str, expected: &str) { equiv_in(&mut Session::new(), code, expected); }

macro_rules! equiv {
    ($($code:expr => $expected:expr),* $(,)?) => {
        $(equiv($code, $expected);)*
    };
}

#[test]
fn language_examples() {
    let mut failures = Vec::new();
    for dir in ["docs", "docs/glyphs"] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|ext| ext != "md") { continue; }
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
                if text == "```apl" {
                    session = Some(Session::new());
                    continue;
                }
                let Some(apl) = session.as_mut() else { continue; };
                let closing = text.starts_with("```");
                let (source, expected) = text.split_once(" ⍝ ").map_or((text, None), |(c, e)| (c, Some(e)));
                if !closing {
                    code.push_str(source);
                    code.push('\n');
                }
                if expected.is_none() && !closing { continue; }
                let result = apl.eval(&code);
                let actual = match result.error { Some(e) => Err(e), None => Ok(result.value) };
                if let Some(expected) = expected {
                    match (actual, run(expected)) {
                        (Ok(Some(actual)), Ok(Some(expected))) if actual == expected => (),
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

#[track_caller]
fn fails(kind: ErrorKind, codes: &[&str]) { for code in codes { fails_in(&mut Session::new(), kind, &[code]); } }

#[test]
fn explicit_output_without_echo() {
    let mut s = Session::new();
    let quiet = || basedpl::EvalOptions { echo: false, ..basedpl::EvalOptions::default() };
    let code = "1 ⋄ ⎕←2 ⋄ ⍎'3 ⋄ ⎕←4 ⋄ 5' ⋄ 6";
    let r = s.eval_with(code, quiet());
    assert!(r.error.is_none());
    assert_eq!(r.value, Some(scalar(6.0)));
    assert_eq!(r.output, ["2", "4"]);
    assert_eq!(s.eval(code).output, ["1", "2", "3", "4", "5", "6"]);
    for code in ["x←7", "+", "/", "f←{⎕←⍵ ⋄ ⍵+1} ⋄ f 8"] {
        let r = s.eval_with(code, quiet());
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.output, if code.starts_with("f←") { vec!["8"] } else { vec![] });
    }
    let r = s.eval_with("⎕←9 ⋄ 1÷0", quiet());
    assert_eq!(r.error.unwrap().kind, Domain);
    assert_eq!(r.output, ["9"]);
    for code in ["]Display 1 2", "]box ?"] { assert!(!s.eval_with(code, quiet()).output.is_empty()); }
    let streamed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events = streamed.clone();
    let output = Arc::new(move |kind, text: &str| events.lock().unwrap().push((matches!(kind, basedpl::OutputKind::Explicit), text.to_owned())));
    let r = s.eval_with("1 ⋄ ⎕←2 ⋄ 1÷0", basedpl::EvalOptions { output: Some(output), ..basedpl::EvalOptions::default() });
    assert_eq!(r.error.unwrap().kind, Domain);
    assert!(r.output.is_empty());
    assert_eq!(*streamed.lock().unwrap(), [(false, "1".into()), (true, "2".into())]);
    assert_eq!(s.eval("3").output, ["3"]);
    equiv_in(&mut s, "x", "7");
}

#[test]
fn calls_with_array_arguments() {
    let mut s = Session::new();
    s.eval("x←42 ⋄ mean←+/÷≢ ⋄ bad←{⎕←⍵ ⋄ 1÷⍵}");
    for (function, codes, expected) in [
        ("mean", vec!["1 2 3"], "2"),
        ("-", vec!["10x", "1x 2x"], "9x 8x"),
        ("/[1]", vec!["1 0", "2 2⍴⍳4"], "[1 2 ⋄]"),
        ("⊢", vec!["(1r3 2x)'ab'(0 3⍴0x)"], "(1r3 2x)'ab'(0 3⍴0x)"),
        ("{k←⍵ ⋄ {k+⍵}⍵}", vec!["3x"], "6x"),
        ("{x←⍵}", vec!["7"], "7"),
    ] {
        let args: Vec<_> = codes.iter().map(|c| run(c).unwrap().unwrap()).collect();
        let r = s.call_with(function, &args, basedpl::EvalOptions { echo: false, ..basedpl::EvalOptions::default() });
        assert!(r.error.is_none(), "{function}: {:?}", r.error);
        assert_eq!(r.value, run(expected).unwrap());
        assert!(r.output.is_empty());
    }
    assert_eq!(s.call("+", &[scalar(3.0)]).output, ["3"]);
    let r = s.call("{}", &[scalar(3.0)]);
    assert!(r.value.is_none() && r.error.is_none() && r.output.is_empty());
    for (function, args, kind) in [
        ("+", vec![], Length),
        ("+", vec![scalar(1.0); 3], Length),
        ("", vec![scalar(1.0)], Syntax),
        ("1", vec![scalar(1.0)], Syntax),
        ("¨", vec![scalar(1.0)], Syntax),
        ("+ ⋄ -", vec![scalar(1.0)], Syntax),
    ] { assert_eq!(s.call(function, &args).error.unwrap().kind, kind); }
    let r = s.call("bad", &[scalar(0.0)]);
    assert_eq!(r.output, ["0"]);
    let e = r.error.unwrap();
    assert_eq!(e.kind, Domain);
    assert_eq!(e.calls.last().unwrap().source.text, "bad");
    assert!(e.span.source.text.contains("bad←"));
    let r = s.call_with("{∇⍵}", &[scalar(0.0)], basedpl::EvalOptions { timeout: Some(std::time::Duration::ZERO), ..basedpl::EvalOptions::default() });
    assert_eq!(r.error.unwrap().kind, Timeout);
    equiv_in(&mut s, "x", "42");
}

#[test]
fn cancellation_preserves_session_and_unwinds_calls() {
    use std::time::Duration;
    let mut s = Session::new();
    let r = s.eval_timeout("keep←42 ⋄ ⎕←7 ⋄ {0::99 ⋄ (+⍣{0})⍵}0", Duration::from_millis(10));
    assert_eq!(r.error.unwrap().kind, Timeout);
    assert_eq!(r.output, ["7"]);
    equiv_in(&mut s, "keep+1", "43");
    s.set("u", AplValue::floats(vec![20000], (0..20000).map(f64::from).collect()).unwrap()).unwrap();
    assert_eq!(s.eval_timeout("∪u", Duration::from_millis(2)).error.unwrap().kind, Timeout);
    assert_eq!(s.eval_timeout("ℙ1000000000000x", Duration::from_millis(2)).error.unwrap().kind, Timeout);
    let interrupt = basedpl::InterruptHandle::default();
    let handle = interrupt.clone();
    let cancel = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        handle.interrupt();
    });
    let r = s.eval_with("{∇⍵}0", basedpl::EvalOptions { interrupt, timeout: Some(Duration::from_secs(2)), ..basedpl::EvalOptions::default() });
    cancel.join().unwrap();
    assert_eq!(r.error.unwrap().kind, Interrupt);
    equiv_in(&mut s, "keep", "42");
}

#[test]
fn leading_unit_axis_broadcasting() {
    for op in ["+", "+¨", "(+⍤0)"] {
        equiv! {
            &format!("(2 3⍴⍳6){op}10 20") => "[11 12 13 ⋄ 24 25 26]",
            &format!("[10 ⋄ 20]{op}[1 2 3 ⋄]") => "[11 12 13 ⋄ 21 22 23]",
            &format!("[10 ⋄]{op}1 2 3") => "[11 ⋄ 12 ⋄ 13]",
            &format!("(1 0⍴0x){op}2 1⍴0x") => "2 0⍴0x",
        }
        fails(Length, &[&format!("(2 3⍴0){op}1 2 3"), &format!("(0 2⍴0){op}3 2⍴0")]);
    }
    let result = run("[10x ⋄ 20x]+[1x 2x 3x ⋄]").unwrap().unwrap();
    assert_eq!(result.shape(), [2, 3]);
    assert_eq!(result.as_integers(), Some([11, 12, 13, 21, 22, 23].as_slice()));

    let mut s = Session::new();
    for op in ["¨", "⍤0"] {
        let r = s.eval(&format!("(1 0⍴0)({{⎕←9 ⋄ ⍺+⍵}}{op})2 1⍴0"));
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.output, ["9", "⍬"]);
    }
}

#[test]
fn selective_assignment() {
    // Dyalog assignment-selective examples, checked in Dyalog 20 (IO=1, ML=1).

    for select in ["↑a", "1⊃a", "first a"] { equiv(&format!("first←↑ ⋄ a←1 2 ⋄ ({select})←3 4 ⋄ a"), "(3 4)2"); }

    for select in [",↑a", "↑¨a"] { fails(Length, &[&format!("a←1 2 ⋄ ({select})←2 2⍴3 4")]); }

    assert!(run("a←1 2 ⋄ (1+a)←0").is_err());
}

#[test]
fn general_axis_forms() {
    // Captured from Dyalog 20.0.53963.0, IO=1, CT=1e-14, ML=1.

    for code in [",[1 3]2 3 4⍴⍳24", "⊂[1 1]2 3⍴⍳6", "1+[1]2 3⍴⍳6", "2↑[1 2]3 4⍴⍳12"] { assert!(run(code).is_err(), "{code}"); }
}

#[test]
fn boxed_display_and_function_trees() {
    let mut s = Session::new();
    assert!(s.eval("]box on -style=max -trains=tree -fns=on").error.is_none());
    assert_eq!(s.eval("⊂4x ⋄ ⊂⊂4x ⋄ ⊂¨0x 1x").output, ["⊂4x", "⊂⊂4x", "┌→────────┐\n│ ⊂0x ⊂1x │\n└∊────────┘"]);
    let r = s.eval("A←2 3 4⍴'DUCKSWANBIRDWORMCAKESEED' ⋄ ⊂[3]A");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["┌→─────────────────────┐\n↓ ┌→───┐ ┌→───┐ ┌→───┐ │\n│ │DUCK│ │SWAN│ │BIRD│ │\n│ └────┘ └────┘ └────┘ │\n│ ┌→───┐ ┌→───┐ ┌→───┐ │\n│ │WORM│ │CAKE│ │SEED│ │\n│ └────┘ └────┘ └────┘ │\n└∊─────────────────────┘"]);
    assert_eq!(s.eval("⍬").output, ["┌⊖┐\n│0│\n└~┘"]);
    assert_eq!(s.eval("0 3⍴0").output, ["┌→────┐\n⌽0 0 0│\n└~────┘"]);
    let f = s.eval("avg←+/÷≢ ⋄ avg");
    assert!(f.error.is_none() && f.value.is_none());
    assert_eq!(f.output, ["fork\n├─ /\n│  └─ +\n├─ ÷\n└─ ≢"]);
    equiv_in(&mut s, "avg 1x 2x 4x", "7r3");
    let r = s.eval("{⎕←⍵ ⋄ ⍵}1 2");
    assert_eq!(r.output.len(), 2);
    assert_eq!(r.output[0], r.output[1]);
    assert!(s.eval("]box -fns=off").error.is_none());
    assert_eq!(s.eval("{⎕←⍵ ⋄ ⍵}1 2").output[0], "1 2");
    equiv_in(&mut s, "⍕1 2", "'1 2'");
    assert!(s.eval("]box off").error.is_none());
    assert_eq!(s.eval("1 2").output, ["1 2"]);
    assert_eq!(s.eval("]Display ⎕←1 2").output, ["1 2", "┌→──┐\n│1 2│\n└~──┘"]);
    assert_eq!(s.eval("1 2").output, ["1 2"]);
    assert!(s.eval("]box on -misspelled").error.is_some());
    assert_eq!(s.eval("1 2").output, ["1 2"]);
    let wide = s.eval("]Display '界' 'a'").output.join("\n");
    assert!(wide.contains("界"));
}

#[test]
fn execute_source_and_session() {
    let mut s = Session::new();
    let r = s.eval("a←⍎'1+1 ⋄ 2+2'");
    assert_eq!(r.value, Some(scalar(4.)));
    assert_eq!(r.output, ["2"]);
    equiv_in(&mut s, "a", "4");
    let failed = s.eval("⍎'⎕←7 ⋄ 1÷0'");
    assert!(failed.value.is_none());
    assert_eq!(failed.output, ["7"]);
    assert_eq!(failed.error.unwrap().span.source.text, "⎕←7 ⋄ 1÷0");
}

#[test]
fn polynomial_representations_and_derivatives() {
    for (code, expected) in [("⊛⊛0 16 ¯12 2", vec![0., 16., -12., 2.]), ("⊛⊛1 0 1", vec![1., 0., 1.]), ("⊛⊛1 ¯2 1", vec![1., -2., 1.])] {
        let value = run(code).unwrap().unwrap();
        assert_eq!(value.shape(), &[expected.len()]);
        for (e, expected) in value.elements().zip(expected) {
            let AplValue::Number(n) = e else { panic!("nonnumeric polynomial coefficient"); };
            let z = n.as_complex().unwrap_or_else(|| num_complex::Complex64::new(n.as_float().unwrap(), 0.));
            assert!((z - expected).norm() < 1e-10, "{code}: {z}");
        }
    }
}

#[test]
fn compact_integers_and_promotion() {
    for (code, values) in [
        ("1x 2r2 6r3", vec![1, 1, 2]),
        ("⍳3r1", vec![1, 2, 3]),
        ("1x+2x 3x", vec![3, 4]),
        ("10x-2x 3x", vec![8, 7]),
        ("2x 3x×4x", vec![8, 12]),
        ("6x 8x÷2x", vec![3, 4]),
        ("-1x ¯2x", vec![-1, 2]),
        ("×¯2x 0x 2x", vec![-1, 0, 1]),
        ("v←¯2x 0x 3x ⋄ (v>0)×v", vec![0, 0, 3]),
        ("'ab'∊'b'", vec![0, 1]),
        ("~0 1", vec![1, 0]),
        ("0 1⍲1 1", vec![1, 0]),
        ("0 1⍱0 0", vec![1, 0]),
        ("⌽⍳3x", vec![3, 2, 1]),
        ("1⌽⍳3x", vec![2, 3, 1]),
        ("5↑⍳3x", vec![1, 2, 3, 0, 0]),
        ("1↓⍳3x", vec![2, 3]),
        ("(⍳3x)[3 1]", vec![3, 1]),
        ("1 0 1/⍳3x", vec![1, 3]),
        ("1 0 1\\1x 2x", vec![1, 0, 2]),
        ("+\\⍳3x", vec![1, 3, 6]),
        ("-¨⍳3x", vec![-1, -2, -3]),
        ("1x 2x∪2x 3x", vec![1, 2, 3]),
        ("1x 2x∩2x", vec![2]),
        ("2x~1x", vec![2]),
        (",⊃(1x 2x⋄ 3x)", vec![1, 2, 3, 0]),
        (",⍉2 2⍴⍳4x", vec![1, 3, 2, 4]),
        (",(⍳2x)×⌝⍳2x", vec![1, 2, 2, 4]),
        ("0x@2⍳3x", vec![1, 0, 3]),
        ("{+/,⍵}⌺3⍳3x", vec![3, 6, 5]),
        ("2x 2x⊤3x", vec![1, 1]),
        ("⍴2 3⍴1x", vec![2, 3]),
        ("⍳0x", vec![]),
        ("⍴1x", vec![]),
        ("↑0⍴⊂1r2 1r3", vec![0, 0]),
    ] {
        let actual = run(code).unwrap().unwrap();
        assert_eq!(actual, ints(&values), "{code}");
        assert_eq!(actual.as_integers(), Some(values.as_slice()), "{code}");
    }

    for code in ["9223372036854775807x+1x", "¯9223372036854775808x÷¯1x", "-¯9223372036854775808x", "|¯9223372036854775808x", "2x*63x"] {
        equiv(code, "9223372036854775808x");
    }

    for code in ["(9223372036854775807x+1x 0x)-1x", "9223372036854775808r1-1x 2x"] {
        assert_eq!(run(code).unwrap().unwrap().as_integers(), Some([i64::MAX, i64::MAX - 1].as_slice()));
    }

    assert!(!run("?0x").unwrap().unwrap().is_exact());
    assert!(!run("?3x 3").unwrap().unwrap().is_exact());
    assert!(run("⍳2x 3x").unwrap().unwrap().is_exact());
    assert!(!run("⍳2x 3").unwrap().unwrap().is_exact());
    for code in ["0∨3x", "3x∨0", "1x×3", "3x+0"] { equiv(code, "3"); }
}

#[test]
fn float_storage_and_kernels() {
    for code in ["⍳3", "⌽⍳3", "2 3⍴⍳6", "⍬", "0 3⍴0"] { assert!(run(code).unwrap().unwrap().as_floats().is_some(), "{code}"); }
    for code in ["1x 2", "1j2 3", "'abc'", "(1 2⋄ 3 4)", "0⍴1x"] { assert!(run(code).unwrap().unwrap().as_floats().is_none(), "{code}"); }
    assert_eq!(AplValue::floats(vec![1], vec![f64::NAN]), Err(Domain));
    assert_eq!(AplValue::floats(vec![2], vec![1.]), Err(Length));
    assert_eq!(AplValue::floats(vec![1], vec![-0.]).unwrap().as_floats().unwrap()[0].to_bits(), 0);
}

#[test]
fn character_parser_and_exact_fill() {
    assert_eq!(run("0↑1x").unwrap().unwrap().prototype(), exact(0, 1).prototype());

    assert!(matches!(parse(Source::new("quoted", "'({⍝⋄})'")), ParseStatus::Complete(_)));
}

#[test]
fn array_literal_completeness() {
    for code in ["[1 2 ⋄", "(1 ⋄", "[({⍵}1 ⋄ 2) ⋄"] { assert!(matches!(parse(Source::new("partial", code)), ParseStatus::Incomplete(_))); }
    for code in ["[1 ⋄ 2)", "(1 ⋄ 2]", "[⋄]"] { assert!(matches!(parse(Source::new("invalid", code)), ParseStatus::Invalid(_))); }
}

#[test]
fn structural_slices_and_brackets() {
    for shape in [vec![0, 3], vec![3, 0], vec![2, 3], vec![2, 2, 3]] {
        let shape = shape.iter().map(usize::to_string).collect::<Vec<_>>().join(" ");
        let source = format!("{shape}⍴⍳12");
        for op in ["⌽⌽", "⊖⊖", "⍉⍉", "⊃↓"] { assert_eq!(run(&format!("{op}{source}")).unwrap(), run(&source).unwrap()); }
    }
}

#[test]
fn dfn_defaults_shy_results_and_numbered_guards() {
    let mut s = Session::new();
    for (code, expected, output) in [
        ("f←{⍺←2 ⋄ ⍺+⍵} ⋄ f 3", Some(5.), vec!["5"]),
        ("f←{⍺←1÷0 ⋄ ⍺+⍵} ⋄ 10 f 3", Some(13.), vec!["13"]),
        ("f←{a←1} ⋄ f 0", Some(1.), vec![]),
        ("(f 0)", Some(1.), vec!["1"]),
        ("1+f 0", Some(2.), vec!["2"]),
        ("{f ⍵ ⋄ 2}0", Some(1.), vec![]),
        ("{}0", None, vec![]),
        ("{a←1 ⋄ 0:2}0", None, vec![]),
        ("{⎕←7}0", Some(7.), vec!["7"]),
        ("{11::7 ⋄ 1÷⍵}0", Some(7.), vec!["7"]),
        ("{6 11::8 ⋄ missingname}0", Some(8.), vec!["8"]),
        ("{11::7 ⋄ 6::8 ⋄ 1÷⍵}0", Some(7.), vec!["7"]),
        ("{11::a←7 ⋄ 1÷⍵}0", Some(7.), vec![]),
        ("{⍺←+ ⋄ ⍺ 4}0", Some(4.), vec!["4"]),
        ("10{g←{⍺←2 ⋄ ⍺+⍵} ⋄ g ⍵}3", Some(5.), vec!["5"]),
        ("{f←{a←1} ⋄ (+f+)3}0", Some(1.), vec![]),
        ("(/ {+⍶ ⍵})1 2 3", Some(6.), vec!["6"]),
        ("{⍵:7 ⋄ 9},1", Some(7.), vec!["7"]),
        ("{⍵:7 ⋄ 9}1 1⍴0", Some(9.), vec!["9"]),
        ("{⍵:7 ⋄ 9}1 1 1⍴1", Some(7.), vec!["7"]),
        ("{⍵:7 ⋄ 9}1.000000000000001", Some(7.), vec!["7"]),
    ] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value, expected.map(scalar), "{code}");
        assert_eq!(r.output, output, "{code}");
    }
    fails_in(&mut s, Value, &["x←{}0", "1+{}0"]);
    fails_in(&mut s, Domain, &["{6::7 ⋄ 1÷⍵}0"]);
    fails_in(&mut s, Syntax, &["{1:1:2}0"]);
    fails_in(&mut s, Length, &["{⍵:7 ⋄ 9}1 1", "{⍵:7 ⋄ 9}⍬"]);
    fails_in(&mut s, Domain, &["{⍵:7 ⋄ 9}⊂,1", "{⍵:7 ⋄ 9}'a'", "{⍵:7 ⋄ 9}2"]);
    fails_in(&mut s, Value, &["10{g←{⍺+⍵} ⋄ g ⍵}3"]);
    for (kind, number) in [(Syntax, 2), (Index, 3), (Rank, 4), (Length, 5), (Value, 6), (Limit, 10), (Domain, 11)] {
        let error = run(&format!("•SIGNAL '{kind}'")).unwrap_err();
        assert_eq!(error.kind, kind);
        assert_eq!(error.message, "explicitly signalled");
        equiv(&format!("{{{number}::7 ⋄ •SIGNAL '{kind}'}}0"), "7");
    }
    equiv("f←{•signal 'LENGTH ERROR'} ⋄ g←{⍵+1} ⋄ {0::g ⍵ ⋄ f ⍵}3", "4");
    fails(Length, &["{11::7 ⋄ •SIGNAL 'LENGTH ERROR'}0", "{0::•SIGNAL 'LENGTH ERROR' ⋄ ÷0}0"]);
    fails(Domain, &["•SIGNAL 11", "•SIGNAL 'unknown'", "•SIGNAL 'INTERRUPT'", "•SIGNAL 'TIMEOUT'", "•SIGNAL 'UNSUPPORTED'"]);
    fails(Rank, &["•SIGNAL ['DOMAIN ERROR' ⋄]"]);
    fails(Syntax, &["0 •SIGNAL 'DOMAIN ERROR'"]);
}

fn exact(n: i64, d: i64) -> AplValue { AplValue::scalar(num_rational::BigRational::new(n.into(), d.into())).unwrap() }

#[test]
fn scalar_math() {
    for (code, value) in [("*1", std::f64::consts::E), ("2⍟32", 5.), ("π1", std::f64::consts::PI), ("¯1○1", std::f64::consts::FRAC_PI_2)] {
        let a = run(code).unwrap().unwrap();
        assert!((a.as_number().unwrap().as_float().unwrap() - value).abs() < 1e-14, "{code}");
    }
    for code in ["1E¯13>|(¯4*0.5)-0J2", "(⌊3.3J2.5)=3J2", "(⌈3.3J2.5)=3J3", "0=0.1|0.3", "0=3|6.000000000000001", "0=1+*π0j1"] { equiv(code, "1x"); }
}

#[test]
fn search_depth_and_random() {
    for code in ["⌊0⍴1x", "⌈0⍴1x", "|0⍴1x", "!0⍴1x", "2x*0⍴1x"] {
        assert_eq!(run(code).unwrap().unwrap().prototype(), exact(0, 1).prototype(), "{code}");
    }
    for code in ["?100⍴9", "13?52", "?100⍴9x", "13x?52x"] {
        let a = run(code).unwrap().unwrap();
        let values: Vec<_> = a
            .elements()
            .map(|e| match e {
                Number(n) => n.as_integer().map(|n| n as usize).unwrap_or_else(|| n.as_float().unwrap() as usize),
                _ => panic!("numeric result"),
            })
            .collect();
        let (len, max) = if code.starts_with('?') { (100, 9) } else { (13, 52) };
        assert_eq!(a.shape(), &[len]);
        assert_eq!(a.is_exact(), code.contains('x'));
        assert!(values.iter().all(|&n| n > 0 && n <= max));
        if len == 13 { assert_eq!(values.iter().collect::<std::collections::HashSet<_>>().len(), len); }
    }
    let rolls = run("?100⍴0").unwrap().unwrap();
    assert!(rolls.elements().all(|e| matches!(e, Number(n) if n.as_float().is_some_and(|v| v > 0. && v < 1.))));
}

#[test]
fn each_error_recovery() {
    let mut session = Session::new();
    assert!(session.eval("f←{⎕←7 ⋄ 100⊃'abc'} ⋄ r←f¨⍬").error.is_none());
    assert_eq!(session.eval("f 0").error.unwrap().kind, Index);
    assert_eq!(session.eval("{⎕←8 ⋄ 1÷0}¨⍬").output, ["8"]);
    assert_eq!(session.eval("f 0").error.unwrap().kind, Index);
}

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
        let expected = if im == 0.0 { scalar(re) } else { AplValue::scalar(num_complex::Complex64::new(re, im)).unwrap() };
        assert_eq!(a, expected, "{code}");
        assert_eq!(run(&a.to_string()).unwrap().unwrap(), a, "display round-trip: {code}");
    }
    assert_eq!(run("1J2×1J¯2").unwrap().unwrap().to_string(), "5");
    assert_eq!(run("¯0J2").unwrap().unwrap().to_string(), "0j2");
    let a = run("1x 0.5 1J2").unwrap().unwrap();
    assert_eq!(
        a.elements().collect::<Vec<_>>(),
        &[exact(1, 1).at(0), number(0.5), AplValue::Number(num_complex::Complex64::new(1.0, 2.0).try_into().unwrap())]
    );
    for code in ["0/1J2", "1J2+0/1x", "+/0/1J2"] { assert_eq!(run(code).unwrap().unwrap().prototype(), number(0.0)); }
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
            for code in [format!("{x}{op}{y}"), format!("{y}{op}{x}")] { check(&code, exact(i64::from(expected), 1)); }
        }
    }

    fails(Domain, &[&format!("1{}x+0J1", "0".repeat(400))]);
    assert_eq!(AplValue::scalar(num_complex::Complex64::new(0.0, f64::INFINITY)), Err(Domain));
    let mut s = Session::new();
    let failed = s.eval("⎕←1J2 ⋄ 1J2÷0");
    assert_eq!(failed.output, ["1j2"]);
    assert_eq!(failed.error.unwrap().kind, Domain);
    assert_eq!(s.eval("1J2+3J4").output, ["4j6"]);
}

#[test]
fn exact_literals_arithmetic_and_roundtrips() {
    // basedpl's explicit exact-number extension, not Dyalog reference cases.
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

    let huge = format!("1{}", "0".repeat(400));
    assert_eq!(run(&format!("{huge}x÷{huge}x")).unwrap().unwrap(), exact(1, 1));
    fails(Domain, &[&format!("{huge}x+0")]);
    assert_eq!(run(&format!("{huge}1r{huge}0+0.5")).unwrap().unwrap(), scalar(1.5));

    let e = run("¯2+1r0").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "1r0");
}

#[test]
fn exact_arrays_prototypes_and_counts() {
    let a = run("9007199254740993x 0.5 1r3").unwrap().unwrap();
    assert_eq!(a.at(0), exact(9007199254740993, 1).at(0));
    assert_eq!(a.at(1), number(0.5));
    assert_eq!(a.at(2), exact(1, 3).at(0));
    let empty = run("0x/1r3").unwrap().unwrap();
    assert_eq!(empty.shape(), &[0]);
    assert_eq!(empty.prototype(), exact(0, 1).prototype());

    check("1x+0/1r3", empty);
    let mut session = Session::new();
    session.set("a", AplValue::empty(vec![usize::MAX, 0], number(0.)).unwrap()).unwrap();
    let largest = format!("{}x", usize::MAX);
    equiv_in(&mut session, "≢a", &largest);
    equiv_in(&mut session, "⍴a", &format!("{largest} 0x"));
    assert_eq!(run("1+0/1r3").unwrap().unwrap().prototype(), number(0.0));

    assert_eq!(run("2x/1r3").unwrap().unwrap().elements().collect::<Vec<_>>(), vec![exact(1, 3).at(0); 2]);

    let invalid = num_rational::BigRational::new_raw(1.into(), 0.into());
    assert_eq!(AplValue::scalar(invalid), Err(Domain));
    let raw = num_rational::BigRational::new_raw(2.into(), (-4).into());
    assert_eq!(scalar(raw), exact(-1, 2));
}

#[test]
fn exact_and_tolerant_comparisons() {
    // Approximate cases follow Dyalog 20 ⎕CT=1E¯14 (documentation-derived):
    // https://docs.dyalog.com/20.0/language-reference-guide/system-functions/ct/
    // Exact/exact and mixed promotion are basedpl's explicit extension.
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
            check(&code, exact(i64::from(expected), 1));
        }
    }

    assert_eq!(run("1x=0/1x").unwrap().unwrap().prototype(), exact(0, 1).prototype());
}

#[test]
fn real_gcd_reconstructs_rational_ratios() {
    for (x, y, expected) in [
        ("1", "(103993÷33102)", 1.0 / 33102.0),
        ("1x", "(π1)", 1.0 / 1725033.0),
        ("¯1", "(¯103993÷33102)", 1.0 / 33102.0),
        ("0", "¯0.75", 0.75),
        ("0", "0", 0.0),
        ("1", "1.000000000000005", 1.0),
        ("1E¯200", "3E¯200", 1e-200),
        ("1E200", "3E200", 1e200),
        ("1E¯200", "1E200", 1e-200),
    ] {
        for code in [format!("{x}∨{y}"), format!("{y}∨{x}")] {
            let Number(actual) = run(&code).unwrap().unwrap() else { panic!("expected a number: {code}") };
            let actual = actual.as_float().unwrap();
            assert!((actual - expected).abs() <= 1e-14 * expected, "{code}: {actual} != {expected}");
        }
    }
    let Number(outside) = run("1∨1.00000000000005").unwrap().unwrap() else { panic!("expected a number") };
    assert!(outside.as_float().unwrap() < 1e-12);
}

#[test]
fn errors_and_evaluation_order() {
    // The right argument fails before the parenthesized left argument is evaluated.
    let e = run("(1÷0)+(0×∞)").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "×");
    let e = run("¯2+1÷0").unwrap_err();
    assert_eq!(e.span.range, 5..7); // UTF-8 bytes, not glyph indices.
    assert_eq!(e.to_string(), "DOMAIN ERROR: division by zero\n --> test:1:5\n¯2+1÷0\n    ^");
    let e = run("(2\n 1÷0)").unwrap_err();
    assert!(e.to_string().contains("test:2:3\n 1÷0)\n  ^"));
}

#[test]
fn structural_completeness_and_source_lifetime() {
    assert!(matches!(parse(Source::new("test", "(2+⍝ )\n")), ParseStatus::Incomplete(_)));
    assert!(matches!(parse(Source::new("test", "(2+))")), ParseStatus::Invalid(_)));
    assert!(matches!(parse(Source::new("test", "('")), ParseStatus::Invalid(_)));
    let source = Source::new("old input", "(1÷0)");
    let weak = Arc::downgrade(&source);
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
    fails(Limit, &[&deep]);
    // Flat evaluation is iterative, not one Rust stack frame per function application.
    equiv(&format!("{}1", "1+".repeat(10_000)), "10001");
    let e = run("{⍵ ⍵}⍣129⊢1 2").unwrap_err();
    assert_eq!(e.kind, Limit);
    assert!(e.message.contains("array nesting"));
}

#[test]
fn based_values() {
    let n = scalar(3.0);
    let unit = n.enclose().unwrap();
    check("3", n.clone());
    check("⊂3", unit.clone());
    check("⊂⊂3", unit.enclose().unwrap());
    check(",3", vector(&[3.]));
    assert_ne!(n, unit);
    assert_ne!(unit, vector(&[3.]));
}

#[test]
fn array_invariants() {
    let scalar = scalar(7.0);
    let singleton = vector(&[7.0]);
    assert_ne!(scalar, singleton);
    assert!(scalar.shape().is_empty());
    assert_eq!(singleton.shape(), &[1]);
    assert_eq!(AplValue::new(vec![2, 2], vec![number(1.0); 3]), Err(Length));
    assert_eq!(AplValue::new(vec![], vec![]), Err(Length));
    assert_eq!(AplValue::new(vec![usize::MAX, 2], vec![number(1.0)]), Err(Limit));
    assert_eq!(AplValue::scalar(f64::NAN), Err(Domain));
    let rows = AplValue::empty(vec![0, 3], number(0.0)).unwrap();
    let cols = AplValue::empty(vec![3, 0], number(0.0)).unwrap();
    assert_ne!(rows, cols);
    assert!(rows.is_empty());
    assert_eq!(rows.shape(), &[0, 3]);
    assert!(AplValue::empty(vec![usize::MAX, 2, 0], number(0.0)).is_ok());
    assert_eq!(AplValue::empty(vec![], number(0.0)), Err(Length));
    let text = AplValue::empty(vec![0, 3], Character('x')).unwrap();
    assert_ne!(rows, text);
    assert_eq!(text.prototype(), Character(' '));
    let normalized = AplValue::new(vec![1], vec![scalar]).unwrap();
    assert_eq!(normalized, singleton);
}

#[test]
fn nested_prototypes_and_value_semantics() {
    // Dyalog 20 prototype examples, documentation-derived:
    // https://docs.dyalog.com/20.0/programming-reference-guide/introduction/arrays/prototypes-and-fill-items/
    let nested = AplValue::new(vec![2], vec![vector(&[1.0, 2.0]), vector(&[3.0, 4.0, 5.0])]).unwrap();
    assert_eq!(nested.prototype(), vector(&[0.0, 0.0]));
    let empty = AplValue::empty(vec![0], nested.prototype().clone()).unwrap();
    assert_eq!(empty.prototype(), nested.prototype());
    let mixed = AplValue::new(vec![2], vec![number(88.0), Character('X')]).unwrap();
    let a = AplValue::new(vec![], vec![mixed]).unwrap();
    let expected = AplValue::new(vec![2], vec![number(0.0), Character(' ')]).unwrap();
    assert_eq!(a.prototype(), expected);
    let saved = nested.clone();
    let mut detached: Vec<AplValue> = nested.elements().collect();
    detached[0] = number(9.0);
    let changed = AplValue::new(vec![2], detached).unwrap();
    assert_ne!(changed, saved);
    drop(nested);
    assert_eq!(saved.prototype(), vector(&[0.0, 0.0]));
}

#[test]
fn persistent_arrays_and_functions() {
    let mut s = Session::new();
    let r = s.eval("v←⍳10");
    assert!(r.error.is_none());
    assert!(r.output.is_empty());
    assert_eq!(r.value.unwrap().shape(), &[10]);
    equiv_in! { &mut s;
        "+/v" => "55",
        "f←+ ⋄ 2 f 3" => "5",
        "sum←+/ ⋄ sum 1 2 3" => "6",
        "f/1 2 3" => "6",
        "⍴v" => ",10x",
        "a←v ⋄ v←0 ⋄ +/a" => "55",
    }
}

#[test]
fn result_output_and_nonexecuting_parse() {
    let mut s = Session::new();
    equiv_in(&mut s, "x←2", "2");
    let r = s.eval("3 ⋄ f←+");
    assert!(r.error.is_none());
    assert!(r.value.is_none());
    assert_eq!(r.output, ["3"]);
    for _ in 0..2 { assert!(matches!(parse(Source::new("check", "x←99 ⋄ ⎕←8")), ParseStatus::Complete(_))); }
    equiv_in(&mut s, "x", "2");
    assert!(s.eval("").value.is_none());
    assert!(s.eval("2+2").error.is_none());
    let r = s.eval("x←5 ⋄ 1÷0");
    assert!(r.error.is_some());
    equiv_in(&mut s, "x", "5");
    // No whole-input transaction.
}

#[test]
fn binding_error_recovery() {
    let mut s = Session::new();
    s.eval("x←10");
    let r = s.eval("bad←{local←99 ⋄ 1÷0} ⋄ bad 0");
    assert_eq!(r.error.unwrap().kind, Domain);
    fails_in(&mut s, Value, &["local"]);
    equiv_in(&mut s, "x", "10");
    fails_in(&mut s, Limit, &["loop←{1+∇⍵} ⋄ loop 0"]);
    equiv_in(&mut s, "2+2", "4");
    fails_in(&mut s, Syntax, &["{⍵←1 ⋄ ⍵}2"]);
}

#[test]
fn definitions_retain_only_needed_sources() {
    let source = Source::new("definition.apl", "bad←{1÷⍵}");
    let weak = Arc::downgrade(&source);
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
    // Dyalog 20: operators acquire their function operand before binding the next operator.

    // Dyalog 20 binding-strength and replicate documentation; not reference executions.
    let mut s = Session::new();
    for _ in 0..2 { equiv_in(&mut s, "op←{⍶⍵} ⋄ +op 3", "3"); }
    for code in ["r←/ ⋄ +r 1 2 3", "+(/)1 2 3", "r←(/) ⋄ sum←+r ⋄ sum 1 2 3", "r←/ ⋄ alias←r ⋄ +(alias)1 2 3"] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), scalar(6.0));
    }
    for (code, expected) in [("1 0 1 r 2 4 6", vec![2.0, 6.0]), ("(,2)/3 4", vec![3.0, 3.0, 4.0, 4.0]), ("1 0 1/,3", vec![3.0, 3.0])] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&expected));
    }
}

#[test]
fn binder_limits() {
    let mut s = Session::new();
    assert_eq!(s.eval(&format!("{}7", "a←".repeat(10_000))).value.unwrap(), scalar(7.0));
    fails_in(&mut s, Limit, &[&format!("+{}1", "/".repeat(10_000))]);
    fails_in(&mut s, Limit, &[&format!("f←+ ⋄ {}", "f←f+f ⋄ ".repeat(128))]);
    assert_eq!(s.eval("2+2").value.unwrap(), scalar(4.0));
}

#[test]
fn real_infinities() { check("¯∞", scalar(f64::NEG_INFINITY)); }

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
    let weak = Arc::downgrade(&empty);
    s.eval_source(empty);
    assert!(weak.upgrade().is_some());
    let result = s.eval("f 0");
    assert!(result.error.is_none() && result.value.is_none());
    s.eval("f←+");
    assert!(weak.upgrade().is_none());
}

#[test]
fn guard_errors_and_recovery() {
    let mut s = Session::new();
    fails_in(&mut s, Domain, &["{0::1÷0 ⋄ 1÷⍵}0"]);
    fails_in(&mut s, Value, &["{0::fresh ⋄ fresh←1 ⋄ 1÷⍵}0"]);
    fails_in(&mut s, Domain, &["{2:1 ⋄ 0}0"]);
    fails_in(&mut s, Domain, &["{x←2 ⋄ {x+⍵}}0"]);
    equiv_in(&mut s, "f←{1:+ ⋄ 0}0 ⋄ 2 f 3", "5");
    assert!(matches!(parse(Source::new("guard", "f←{0::⎕←1}")), ParseStatus::Complete(_)));
    assert_eq!(s.eval("2+2").value.unwrap(), scalar(4.0));
}
