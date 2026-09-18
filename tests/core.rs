use miniapl::{
    parse, Array,
    Element::{self, *},
    Error, ErrorKind,
    ErrorKind::*,
    ParseStatus, Session, Source,
};
use std::rc::Rc;

fn run(code: &str) -> Result<Option<Array>, Error> {
    let result = Session::new().eval_source(Source::new("test", code));
    match result.error { Some(e) => Err(e), None => Ok(result.value) }
}

fn number(n: f64) -> Element { Element::Number(n.try_into().unwrap()) }
fn scalar(n: impl TryInto<miniapl::Number>) -> Array { Array::scalar(n).unwrap() }
fn vector(values: &[f64]) -> Array { Array::from_parts(vec![values.len()], values.iter().copied().map(number).collect(), number(0.0)).unwrap() }
fn ints(values: &[i64]) -> Array { Array::integers(vec![values.len()], values.to_vec()).unwrap() }

#[track_caller]
fn check_in(session: &mut Session, code: &str, expected: Array) {
    let result = session.eval(code);
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    assert_eq!(result.value, Some(expected), "{code}");
}

#[track_caller]
fn check(code: &str, expected: Array) { check_in(&mut Session::new(), code, expected); }

#[track_caller]
fn equiv_in(session: &mut Session, code: &str, expected: &str) { check_in(session, code, run(expected).unwrap().expect("expected an array")); }

#[track_caller]
fn equiv(code: &str, expected: &str) { equiv_in(&mut Session::new(), code, expected); }

macro_rules! equiv {
    ($($code:expr => $expected:expr),* $(,)?) => {
        $(equiv($code, $expected);)*
    };
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

fn chars(text: &str) -> Array {
    let data: Vec<_> = text.chars().map(Character).collect();
    let shape = if data.len() == 1 { vec![] } else { vec![data.len()] };
    Array::from_parts(shape, data, Character(' ')).unwrap()
}

#[test]
fn explicit_output_without_echo() {
    let mut s = Session::new();
    let quiet = || miniapl::EvalOptions { echo: false, ..miniapl::EvalOptions::default() };
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
    equiv_in(&mut s, "x", "7");
}

#[test]
fn calls_with_array_arguments() {
    let mut s = Session::new();
    s.eval("x←42 ⋄ mean←+/÷≢ ⋄ bad←{⎕←⍵ ⋄ 1÷⍵}");
    for (function, codes, expected) in [
        ("mean", vec!["1 2 3"], "2"),
        ("-", vec!["10x", "1x 2x"], "9x 8x"),
        ("/[1]", vec!["1 0", "2 2⍴⍳4"], "1 2⍴1 2"),
        ("⊢", vec!["(1r3 2x)'ab'(0 3⍴0x)"], "(1r3 2x)'ab'(0 3⍴0x)"),
        ("{k←⍵ ⋄ {k+⍵}⍵}", vec!["3x"], "6x"),
        ("{x←⍵}", vec!["7"], "7"),
    ] {
        let args: Vec<_> = codes.iter().map(|c| run(c).unwrap().unwrap()).collect();
        let r = s.call_with(function, &args, miniapl::EvalOptions { echo: false, ..miniapl::EvalOptions::default() });
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
    let r = s.call_with("{∇⍵}", &[scalar(0.0)], miniapl::EvalOptions { timeout: Some(std::time::Duration::ZERO), ..miniapl::EvalOptions::default() });
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
    s.set("u", Array::floats(vec![20000], (0..20000).map(f64::from).collect()).unwrap()).unwrap();
    assert_eq!(s.eval_timeout("∪u", Duration::from_millis(2)).error.unwrap().kind, Timeout);
    let interrupt = miniapl::InterruptHandle::default();
    let handle = interrupt.clone();
    let cancel = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        handle.interrupt();
    });
    let r = s.eval_with("{∇⍵}0", miniapl::EvalOptions { interrupt, timeout: Some(Duration::from_secs(2)), ..miniapl::EvalOptions::default() });
    cancel.join().unwrap();
    assert_eq!(r.error.unwrap().kind, Interrupt);
    equiv_in(&mut s, "keep", "42");
}

#[test]
fn leading_unit_axis_broadcasting() {
    for op in ["+", "+¨", "(+⍤0)"] {
        equiv! {
            &format!("(2 3⍴⍳6){op}10 20") => "2 3⍴11 12 13 24 25 26",
            &format!("(2 1⍴10 20){op}1 3⍴1 2 3") => "2 3⍴11 12 13 21 22 23",
            &format!("(1 1⍴10){op}1 2 3") => "3 1⍴11 12 13",
            &format!("(1 0⍴0x){op}2 1⍴0x") => "2 0⍴0x",
        }
        fails(Length, &[&format!("(2 3⍴0){op}1 2 3"), &format!("(0 2⍴0){op}3 2⍴0")]);
    }
    let result = run("(2 1⍴10x 20x)+1 3⍴1x 2x 3x").unwrap().unwrap();
    assert_eq!(result.shape(), [2, 3]);
    assert_eq!(result.as_integers(), Some([11, 12, 13, 21, 22, 23].as_slice()));
    equiv! {
        "(2 1⍴1 2)<1 3⍴1 2 3" => "2 3⍴0x 1x 1x 0x 0x 1x",
        "(2 1⍴1x 2x)<1 3⍴1x 2x 3x" => "2 3⍴0x 1x 1x 0x 0x 1x",
        "(2 1 2⍴1 2 3 4)(+⍤1)1 3 2⍴10 20 30 40 50 60" => "2 3 2⍴11 22 31 42 51 62 13 24 33 44 53 64",
        "(2 1 2⍴1 2 3 4)+1 3 1⍴10 20 30" => "2 3 2⍴11 12 21 22 31 32 13 14 23 24 33 34",
        "(2 3⍴⍳6)+[2]10 20 30" => "2 3⍴11 22 33 14 25 36",
        "(2 1⍴10 20)+[2 1]3 1⍴1 2 3" => "2 3⍴11 12 13 21 22 23",
        "(⊂2 1⍴1 2)+⊂1 3⍴10 20 30" => "⊂2 3⍴11 21 31 12 22 32",
        "(1 2⍴9223372036854775807x 1x)+1x 2x" => "2 2⍴9223372036854775808x 2x 9223372036854775809x 3x",
    }
    let mut s = Session::new();
    for op in ["¨", "⍤0"] {
        let r = s.eval(&format!("(1 0⍴0)({{⎕←9 ⋄ ⍺+⍵}}{op})2 1⍴0"));
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.output, ["9", "⍬"]);
    }
    equiv! {
        "(⍳1)(+⍤0)⍳3" => "2 3 4",
        "1 2 (+⍤0)2 3⍴0" => "2 3⍴1 1 1 2 2 2",
    }
}

#[test]
fn selective_assignment() {
    equiv! {
        "a←1 ⋄ (⍬⊃a)←3 4 ⋄ a" => "3 4",
        "a←⍬ ⋄ (⍬⊃a)←3 4 ⋄ a" => "3 4",
        "a←1 2 ⋄ (⍬⊃a),←3 4 ⋄ a" => "1 2 3 4",
        "a←(1 2)(3 4) ⋄ (⍬⊃1⊃a)←5 6 7 ⋄ a" => "(5 6 7)(3 4)",
        "a←1 2 ⋄ (⍬⊃⌽a)←3 4 ⋄ a" => "4 3",
    }
    fails(Length, &["a←1 2 ⋄ (⍬⊃,a)←3 4 5", "a←1 2 ⋄ (⍬⊃⍬⌷a)←2 2⍴3 4"]);
    // Dyalog assignment-selective examples, checked in Dyalog 20 (IO=1, ML=1).
    equiv! {
        "a←'HELLO' ⋄ ((a∊'AEIOU')/a)←'*' ⋄ a" => "'H*LL*'",
        "z←3 4⍴⍳12 ⋄ (5↑,z)←0 ⋄ ,z" => "0 0 0 0 0 6 7 8 9 10 11 12",
        "m←3 3⍴⍳9 ⋄ (1 1⍉m)←0 ⋄ ,m" => "0 2 3 4 0 6 7 8 0",
        "a←'Andy' 'Karen' 'Liam' ⋄ (('a'=∊a)/∊a)←'*' ⋄ a" => "'Andy' 'K*ren' 'Li*m'",
        "a←'HELLO' 'WORLD' ⋄ (2↑¨a)←'*' ⋄ a" => "'**LLO' '**RLD'",
        "a←'HELLO' 'WORLD' ⋄ ((a='O')/¨a)←'*' ⋄ a" => "'HELL*' 'W*RLD'",
        "a←(1 2)(3 4) ⋄ (1↑a)←⊂8 9 10 ⋄ a" => "(8 9 10)(3 4)",
        "a←3⍴0x ⋄ (5⍴a)+←1x ⋄ a" => "2x 2x 1x",
        "a←1 2 3 ⋄ (⌽a[1 3])←8 9 ⋄ a" => "9 2 8",
        "a←1 2 3 ⋄ (0↑a)←9 ⋄ a" => "1 2 3",
        "a←0⍴⊂2 3⍴⍳6 ⋄ (⌽[2]¨a)←9 ⋄ ⍴⊃a" => "2 3",
        "a←1 2 ⋄ (3↑a)←4 ⋄ a" => "4 4",
        "a←(1 2)(3 4) ⋄ (⊃a)←7 8 9 ⋄ 1⊃a" => "7 8 9",
    }
    for select in ["⊃a", "1⊃a", "first a"] { equiv(&format!("first←⊃ ⋄ a←1 2 ⋄ ({select})←3 4 ⋄ a"), "(3 4)2"); }
    equiv! {
        "a←1 ⋄ (⊃a)←3 4 ⋄ a" => "⊂3 4",
        "a←(1 2)(3 4) ⋄ (⊃¨a)←(5 6)(7 8) ⋄ a" => "((5 6)2)((7 8)4)",
        "a←1 2 3 ⋄ ((1∘↑)a)←9 ⋄ a" => "9 2 3",
        "a←1 2 ⋄ ((1∘⊃)a)←3 4 ⋄ a" => "(3 4)2",
        "a←1 2 ⋄ ((⍬∘⊃)a)←3 4 5 ⋄ a" => "3 4 5",
        "a←(1 2)(3 4) ⋄ ((1∘⊃)¨a)←(5 6)(7 8) ⋄ a" => "((5 6)2)((7 8)4)",
    }
    for select in [",⊃a", "⍬⌷⊃a", "1⌷a", "⊃¨a"] { fails(Length, &[&format!("a←1 2 ⋄ ({select})←2 2⍴3 4")]); }
    fails(Index, &["a←⍬ ⋄ (⊃a)←3 4"]);
    assert!(run("a←1 2 ⋄ (1+a)←0").is_err());
    let r = Session::new().eval("a←1 2 ⋄ ((⎕←1)/a)←3 ⋄ a");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["1", "3 3"]);
}

#[test]
fn indexed_modified_and_strand_assignment() {
    equiv! {
        "a←9 10 11 ⋄ 1 2 a[2] 3 4 5 6[3]" => "4",
        "'a' 2[1] 2[1] 2[1]" => "'a'",
        "1+a←3" => "4",
        "{1+a←3 ⋄ 9}0" => "4",
        "{a←1 ⋄ +a+←3 ⋄ 9}0" => "3",
        "a←1+b←2 ⋄ a b" => "3 2",
        "a←1 2 3 ⋄ 2×a[2]+←10" => "20",
        "a←1 2 3 ⋄ 1+(⌽a)←4 5 6" => "5 6 7",
        "(a b)c←(3 4)5 ⋄ a b c" => "3 4 5",
        "a←1 2 3 ⋄ rev←⌽ ⋄ (rev a)←4 5 6 ⋄ a" => "6 5 4",
        "a←1 ⋄ f←+ ⋄ 2×a f←3" => "6",
        "{a←1 ⋄ f←+ ⋄ a f←3 ⋄ a f}0" => "3 3",
        "{a←1 ⋄ f←+ ⋄ a f∘⊢←3 ⋄ a}0" => "4",
        "a←1 2 ⋄ r←0 ⋄ 1+a +⍤r←3 4" => "4 5",
        "a←1 ⋄ b←2 ⋄ a b+←3 4 ⋄ a b" => "4 6",
        "a←1 ⋄ b←2 ⋄ (a b)+←3 ⋄ a b" => "4 5",
        "a←1 ⋄ a a+←3 4 ⋄ a" => "8",
    }
    let r = Session::new().eval("a←1 ⋄ b←2 ⋄ a b{⎕←⍺ ⋄ ⍺+⍵}←3 4");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["1", "2"]);
    let r = Session::new().eval("a←0 ⋄ (⎕←a)+a←⎕←3");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["3", "3", "6"]);
    equiv! {
        "a←1 2 3 ⋄ a[2]←9 ⋄ a" => "1 9 3",
        "a←1 2 3 ⋄ r←a[2 2]+←10 20 ⋄ a" => "1 32 3",
        "a←1 2 3 ⋄ r←a[2 2]+←10 20 ⋄ r" => "10 20",
        "a←3 5⍴0 ⋄ a[1 1 3;1 3 3 5]+←1 ⋄ ,a" => "2 0 4 0 2 0 0 0 0 0 1 0 2 0 1",
        "a←1 2 3 ⋄ b←a ⋄ a[1 3]←8 9 ⋄ b" => "1 2 3",
        "a←1x 2x ⋄ a×←2x ⋄ a" => "2x 4x",
    }
    equiv("a←1x ⋄ a+←9223372036854775807x ⋄ a", "9223372036854775808x");
    equiv! {
        "a←1 ⋄ f←{a+←1 ⋄ a} ⋄ (f 0) a" => "2 1",
        "(a (b c))←1 (2 3) ⋄ a b c" => "1 2 3",
        "a b←1 ⋄ a b" => "1 1",
        "a←1 2 3 ⋄ a[3 1][2]←9 ⋄ a" => "9 2 3",
        "a←(1 2)(3 4) ⋄ a[⊂(,2)(,1)]←9 ⋄ ⊃2⊃a" => "9",
    }
    let mut s = Session::new();
    assert!(s.eval("a←1 2 ⋄ f←{⎕←a ⋄ ⍺+⍵} ⋄ a[1 2]f←10 20").error.is_none());
    equiv_in(&mut s, "a", "11 22");
}

#[test]
fn general_axis_forms() {
    // Captured from Dyalog 20.0.53963.0, IO=1, CT=1e-14, ML=1.
    equiv! {
        "⍴,[2 3]2 3 4⍴⍳24" => "2 12",
        "⍴,[1.5]2 3⍴⍳6" => "2 1 3",
        "⍴,[⍬]2 3⍴⍳6" => "2 3 1",
        "↑[0.5](1 2)(3 4)" => "2 2⍴1 3 2 4",
        "↑[1 3](2 3⍴⍳6)(2 3⍴6+⍳6)" => "2 2 3⍴1 2 3 7 8 9 4 5 6 10 11 12",
        "1 2,[0.5]3 4" => "2 2⍴1 2 3 4",
        "1,[1.5]2 3" => "2 2⍴1 2 1 3",
        "1 2+[1]2 3⍴⍳6" => "2 3⍴2 3 4 6 7 8",
        "2 1↑[2 1]3 4⍴⍳12" => "1 2⍴1 2",
        "1 1↓[2 1]3 4⍴⍳12" => "2 3⍴6 7 8 10 11 12",
        "(2 1)(1 2)⌷[2 1]2 3⍴⍳6" => "2 2⍴2 1 5 4",
    }
    equiv("⊃⊂[2 1]2 3⍴⍳6", "3 2⍴1 4 2 5 3 6");
    for code in [",[1 3]2 3 4⍴⍳24", "⊂[1 1]2 3⍴⍳6", "1+[1]2 3⍴⍳6", "2↑[1 2]3 4⍴⍳12"] { assert!(run(code).is_err(), "{code}"); }
    equiv! {
        "⍴⊂[3]2 0 4⍴0x" => "2x 0x",
        "⍴⊃⊂[3]2 0 4⍴0x" => ",4x",
    }
}

#[test]
fn boxed_display_and_function_trees() {
    let mut s = Session::new();
    assert!(s.eval("]box on -style=max -trains=tree -fns=on").error.is_none());
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
fn format_and_execute() {
    equiv! {
        "2⍕3.125 ¯3.125 2.675 ¯2.675" => "' 3.13 ¯3.13 2.68 ¯2.68'",
        "0⍕2.5 ¯2.5 1.5 ¯1.5" => "' 3 ¯3 2 ¯2'",
        "2⍕¯0.001 0.001" => "'  0.00 0.00'",
        "¯2⍕3.25 ¯3.25 325 0.0325" => "' 3.3E0 ¯3.3E0 3.3E2 3.3E¯2'",
        "¯2⍕2 2⍴3.125 0.002 1000 20" => "↑' 3.1E0 2.0E¯3' ' 1.0E3 2.0E1 '",
        "⍕¯1E¯100j¯2E¯99" => "'¯1E¯100j¯2E¯99'",
        "⍕1E¯6 1E¯7 1E16 1E17" => "'0.000001 1E¯7 10000000000000000 1E17'",
        "⍎⍕5E¯324 ¯1.2345678901234567E200 1E¯100j2E100" => "5E¯324 ¯1.2345678901234567E200 1E¯100j2E100",
    }
    for (code, text) in [
        ("⍕12.34", "12.34"),
        ("⍕1x 1r3", "1x 1r3"),
        ("⍕⍬", ""),
        ("4 1⍕1.1 2 ¯4 2.547", " 1.1 2.0¯4.0 2.5"),
        ("7 ¯3⍕5 15 155 1555", "5.00E0 1.50E1 1.55E2 1.56E3 "),
        ("0 2⍕1 2", " 1.00 2.00"),
        ("0 2⍕1r3 2r3", " 0.33 0.67"),
        ("0 0⍕9223372036854775808x", " 9223372036854775808"),
        ("0 20⍕÷3", " 0.3333333333333333____"),
    ] { check(code, chars(text)); }
    equiv! {
        "⍴⍕1" => ",1",
        "⍴⍕0 3⍴0" => "0 5",
        "⍴⍕3 0⍴0" => "3 0",
        "⍴5 2⍕2 3 4⍴⍳24" => "2 3 20",
        ",3 0 6 2⍕3 2⍴10.1 15 1001 22.357 101 1110.1" => "' 10 15.00*** 22.36101******'",
        ",⍕2 2⍴1 12.3 123 4" => "'  1 12.3123  4  '",
    }
    equiv("⍎⍕1x 1r3", "1x 1r3");
    let mut s = Session::new();
    let r = s.eval("a←⍎'1+1 ⋄ 2+2'");
    assert_eq!(r.value, Some(scalar(4.)));
    assert_eq!(r.output, ["2"]);
    equiv_in! { &mut s;
        "a" => "4",
        "f←{a←10 ⋄ ⍎'a+⍵'} ⋄ f 3" => "13",
        "a" => "4",
        "''⍎'a+1'" => "5",
    }
    assert!(s.eval("⍎''").value.is_none());
    fails_in(&mut s, Value, &["a←⍎''"]);
    let failed = s.eval("⍎'⎕←7 ⋄ 1÷0'");
    assert_eq!(failed.output, ["7"]);
    assert_eq!(failed.error.unwrap().span.source.text, "⎕←7 ⋄ 1÷0");
    fails(Domain, &["1⍕1j2", "¯1 2⍕1", "⍎1", "0.5⍕1"]);
}

#[test]
fn inverse_power() {
    equiv! {
        "W←×∘*⍨⍣¯1 ⋄ ⌊1E12×W 0 1 (*1) ¯0.1 1j1" => "0 567143290409 1000000000000 ¯111832559159 656966069230j325450339413",
        "W←×∘*⍨⍣¯1 ⋄ W ¯1÷*1" => "¯1",
        "W←×∘*⍨⍣¯1 ⋄ W (0x 0x)(0⍴0x)" => "(0 0)⍬",
        "W←×∘*⍨⍣¯1 ⋄ x←1E¯100 ¯1E¯100 1E¯12 ¯1E¯12 0.099 ¯0.099 1E300 ⋄ ∧/1E¯12>|1-(W x)×(*W x)÷x" => "1x",
        "W←×∘*⍨⍣¯1 ⋄ x←¯1j0.1 1j¯1 ⋄ ∧/1E¯12>|1-(W x)×(*W x)÷x" => "1x",
    }
    fails(Domain, &["(×∘*⍨⍣¯1)¯1", "(×∘*⍨⍣¯1)'a'"]);
    equiv! {
        "(∘.-∘4 5)⍣¯1⊢2 2⍴¯3 ¯4 ¯2 ¯3" => "1 2",
        "(4 5∘(∘.-))⍣¯1⊢2 2⍴3 2 4 3" => "1 2",
        "(∘.×∘4x 5x)⍣¯1⊢2 2⍴2x 5r2 1x 5r4" => "1r2 1r4",
        "(∘.×∘(2 2⍴1 2 3 4))⍣¯1⊢3 2 2⍴1 2 3 4 2 4 6 8 3 6 9 12" => "1 2 3",
        "(4∘(∘.×))⍣¯1⊢4 8" => "1 2",
        "(∘.×∘4 5)⍣¯1⊢4 5" => "1",
        "(∘.+∘4 5)⍣¯1⊢0 2⍴0" => "⍬",
        "(4 5∘(∘.-))⍣¯1⊢2 2 2⍴3 2 1 0 4 3 2 1" => "2 2⍴1 2 3 4",
        "(∘.+∘4 5)⍣¯1⊢3 0 2⍴0" => "3 0⍴0",
    }
    fails(Domain, &["(∘.×∘4 5)⍣¯1⊢2 2⍴4 5 8 11", "(∘.×∘4 5)⍣¯1⊢2 3⍴⍳6", "(∘.×∘⍬)⍣¯1⊢2 0⍴0", "(∘.×∘0 0)⍣¯1⊢2 2⍴0"]);
    equiv! {
        "⍳⍣¯1⊢1 2 3" => ",3",
        "⍳⍣¯1⊢⍳2 3" => "2 3",
        "{(5○⍨-⍵)=⍵∘○⍣¯1⊢5}⍳12" => "12⍴1x",
        "(1∘+⍣¯3)10" => "7",
        "((32∘+)∘(×∘1.8)⍣¯1)32 212" => "0 100",
        "(+\\⍣¯1)1 3 6 10" => "1 2 3 4",
        "(+\\⍣¯1)1x 3x 6x" => "1x 2x 3x",
        "(≠\\⍣¯1)1x 1x 0x 0x" => "1x 0x 1x 0x",
        "(2∘⊥⍣¯1)9" => "1 0 0 1",
        "(2x∘⊥⍣¯1)9x" => "1x 0x 0x 1x",
        "(0x 60x 60x∘⊥⍣¯1)3661x" => "1x 1x 1x",
        "(2x∘⊤⍣¯1)1x 0x 1x" => "5x",
        "(2∘⊥⍣¯1)2.5" => "1 0.5",
        "(2 1∘⍉⍣¯1)2 3⍴⍳6" => "3 2⍴1 4 2 5 3 6",
        "(+⍨⍣¯1)3x" => "3r2",
        "0.5=1∘○(1∘○⍣¯1)0.5" => "1x",
        "11○(1∘○⍣¯1)0.5" => "0",
        "4○1E300" => "1E300",
        "¯4○¯1E300" => "¯1E300",
        "((2x∘+)⍤0⍣¯1)3x 4x" => "1x 2x",
        "(⍸⍣¯1)1 3 3" => "1x 0x 2x",
        "(⍸⍣¯1)(1 2)(2 1)" => "2 2⍴0x 1x 1x 0x",
        "(⍸⍣¯1)⍬" => "0⍴0x",
        "(2∘⊥⍣¯1)0" => "⍬",
        "3x(+⍣¯1)5x" => "2x",
        "((2x∘+)¨⍣¯1)3x 4x" => "1x 2x",
        "(×⍨⍣¯1)4" => "2",
        "(⌽⍣¯1)⍳3x" => "3x 2x 1x",
        "(⊂⍣¯1)⊂1x 2x" => "1x 2x",
    }
    fails(Domain, &["({⍵}⍣¯1)3", "(0∘×⍣¯1)0", "(⍸⍣¯1)3 1", "(⍸⍣¯1)0", "(2∘⊥⍣¯1)¯1", "(2 2∘⊥⍣¯1)5", "(⊂[2]⍣¯1)⊂[2]2 3⍴⍳6"]);
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
        (",↑(1x 2x)(3x)", vec![1, 2, 3, 0]),
        (",⍉2 2⍴⍳4x", vec![1, 3, 2, 4]),
        (",(⍳2x)∘.×⍳2x", vec![1, 2, 2, 4]),
        ("0x@2⍳3x", vec![1, 0, 3]),
        ("{+/,⍵}⌺3⍳3x", vec![3, 6, 5]),
        ("2x 2x⊤3x", vec![1, 1]),
        ("⍴2 3⍴1x", vec![2, 3]),
        ("⍳0x", vec![]),
        ("⍴1x", vec![]),
        ("⊃0⍴⊂1r2 1r3", vec![0, 0]),
    ] {
        let actual = run(code).unwrap().unwrap();
        assert_eq!(actual, ints(&values), "{code}");
        assert_eq!(actual.as_integers(), Some(values.as_slice()), "{code}");
    }
    equiv! {
        "avg←+/÷≢ ⋄ avg 1x 2x 4x" => "7r3",
        "+/⍳3x" => "6x",
        "×/⍳4x" => "24x",
        "1x 2x+.×3x 4x" => "11x",
        "⌊3r2" => "1x",
        "⌈3r2" => "2x",
        "|¯3x" => "3x",
        "3x⌊4x" => "3x",
        "3x⌈4x" => "4x",
        "¯3x|5x" => "¯1x",
        "6x∨4x" => "2x",
        "6x∧4x" => "12x",
        "!20x" => "2432902008176640000x",
        "5x!10x" => "252x",
        "2x*10x" => "1024x",
        "2x*¯1x" => "1r2",
        "2x⊥1x 0x 1x" => "5x",
        "≢(1x 2x)(1r3 'a')" => "2x",
        "≢0⍴1x" => "0x",
    }
    for code in ["9223372036854775807x+1x", "¯9223372036854775808x÷¯1x", "-¯9223372036854775808x", "|¯9223372036854775808x", "2x*63x"] {
        equiv(code, "9223372036854775808x");
    }
    equiv! {
        "¯9223372036854775808x-1x" => "¯9223372036854775809x",
        "9223372036854775807x×2x" => "18446744073709551614x",
    }
    for code in ["(9223372036854775807x+1x 0x)-1x", "9223372036854775808r1-1x 2x"] {
        assert_eq!(run(code).unwrap().unwrap().as_integers(), Some([i64::MAX, i64::MAX - 1].as_slice()));
    }
    equiv! {
        "+/9223372036854775807x 1x ¯1x" => "9223372036854775807x",
        "(1÷3)+(1÷6)" => "0.5",
        "1x+0.5" => "1.5",
        "≢(1x 2x)(3 4)" => "2",
        "≢'abc'" => "3",
        "≢0⍴⊂1x 2" => "0",
        "=/⍬" => "1x",
        "≠/⍬" => "0x",
        "⊃?0⍴⊂1x 2x" => "0x 0x",
    }
    assert!(!run("?0x").unwrap().unwrap().is_exact());
    assert!(!run("?3x 3").unwrap().unwrap().is_exact());
    assert!(run("⍳2x 3x").unwrap().unwrap().is_exact());
    assert!(!run("⍳2x 3").unwrap().unwrap().is_exact());
    for code in ["0∨3x", "3x∨0", "1x×3", "3x+0"] { equiv(code, "3"); }
}

#[test]
fn matrix_division() {
    equiv! {
        "⌹2x" => "1r2",
        "3x 5x 7x⌹3 2⍴1x 1x 1x 2x 1x 3x" => "1x 2x",
        "⌹1x 2x" => "1r5 2r5",
        "⌹2 2⍴0x 2x 1x 0x" => "2 2⍴0x 1x 1r2 0x",
        "⌹3 0⍴0x" => "0 3⍴0x",
        "(3 2⍴1)⌹3 0⍴0" => "0 2⍴0",
    }
    fails(Domain, &["⌹0", "⌹2 2⍴1", "⌹2 2⍴1x", "⌹'ab'"]);
    fails(Length, &["⌹2 3⍴1", "1 2⌹3 2⍴1"]);
    fails(Rank, &["⌹1 1 1⍴1"]);
}

#[test]
fn at_and_stencil() {
    equiv! {
        "⌽@2 4⍳5" => "1 4 3 2 5",
        "10×@2 4⍳5" => "1 20 3 40 5",
        "0@(2∘|)⍳5" => "0 2 0 4 0",
        "10 20@2 2⍳3" => "1 20 3",
        "a←⍳3 ⋄ b←0@2⊢a ⋄ a" => "1 2 3",
        "{⍺}⌺3⊢2 0⍴0" => "2 1⍴1 ¯1",
        "{+/,⍵}⌺(2 1⍴3 2)⍳8" => "3 9 15 21",
        "{⍴⍵}⌺3⊢2 3⍴⍳6" => "2 2⍴3",
    }
    let mut s = Session::new();
    s.eval("G←2 3⍴('ABC' 1)('DEF' 2)('GHI' 3)('JKL' 4)('MNO' 5)('PQR' 6)");
    equiv_in(&mut s, "G[((1 2)1)((2 3)2)]", "'DEF' 6");
    s.eval("H←('' '*' @((1 2)1)((2 3)2))G");
    equiv_in! { &mut s;
        "((1 2)1)⊃H" => "''",
        "((2 3)2)⊃H" => "'*'",
    }
    let r = s.eval("{⎕←99 ⋄ ⍵}@⍬⍳3");
    assert!(r.error.is_none());
    assert_eq!(r.output, ["99", "1 2 3"]);
    fails(Domain, &["{⍵}⌺3⍳1", "{⍵}⌺0⍳3", "{⍵}⌺⍬⍳3", "0@{2 0 1}⍳3"]);
}

#[test]
fn encode_decode() {
    equiv! {
        "60⊥3 13" => "193",
        "2x⊥1x 0x 1x 0x" => "10x",
        "0x 10x⊤125x" => "12x 5x",
        "2⊥⍬" => "0",
        "⍬⊥1" => "0",
        "0 0 2⊤3" => "0 1 1",
        "⍴(0 3⍴0)⊤2 2⍴1" => "0 3 2 2",
        "(2 1⍴2 10)⊥1 0 1" => "5 101",
    }
    fails(Length, &["2 3⊥1 2 3"]);
}

#[test]
fn pick_and_partition() {
    equiv! {
        "((2 1)1 2)⊃2 3⍴('ABC' 1)('DEF' 2)('GHI' 3)('JKL' 4)('MNO' 5)('PQR' 6)" => "'K'",
        "(5⍴⊂⍬)⊃10" => "10",
        "⍬⊃1 2" => "1 2",
        "2⌷3 4⍴⍳12" => "5 6 7 8",
        "2⌷[2]3 4⍴⍳12" => "2 6 10",
        "⍴⍬ ⍬⌷3 4⍴0" => "0 0",
        "⍬⌷1" => "1",
        "⌷1 2" => "1 2",
        "⊆1 2" => "⊂1 2",
        "⊆1" => "1",
        "⊆(1 2)(3 4)" => "(1 2)(3 4)",
        "3 2 2 1 0 1⊆'abcdef'" => "'abcd'(,'f')",
        "1 1 0 1⊆[1]4 2⍴⍳8" => "2 2⍴(1 3)(2 4)(,7)(,8)",
        "0⊂2 3⍴0" => "0⍴⊂2 0⍴0",
        "1 0 1⊆0 3⍴0" => "0 2⍴⊂⍬",
    }
    for code in ["(,1)⊂'abcd'", "1⊆'abcd'", "(,1)⊆'abcd'"] { equiv(code, ",⊂'abcd'"); }
    fails(Index, &["0⊃1 2"]);
    fails(Rank, &["1 1⊃1 2", "1⊆3"]);
    fails(Domain, &["¯1⊂1 2"]);
    fails(Length, &["1 0⊆1 2 3", "1 2⌷1 2"]);
    fails(Index, &["0⌷1 2"]);
}

#[test]
fn grading() {
    equiv! {
        "⍒2 1 2 1" => "1x 3x 2x 4x",
        "⍋1 (1+8E¯15) 1" => "1x 3x 2x",
        "⍋1j2 1 1j¯2 'a' 0" => "5x 3x 2x 1x 4x",
        "⍋9007199254740993x 9007199254740992 9007199254740992x" => "2x 3x 1x",
        "⍋(2 2⍴1 2 3 4)(1 4⍴1 2 0 0)" => "1x 2x",
        "⍋(1 2⍴1 2)(1 2)" => "2x 1x",
        "⍋(0 5 2⍴0)(0 3 4⍴0)(0 1⍴'')⍬" => "4x 1x 2x 3x",
        "⍋(0⍴⊂1 2)(0⍴0)(0⍴'')" => "2x 1x 3x",
        "⍋3 0⍴0" => "1x 2x 3x",
        "⍋⍬" => "0⍴0x",
        "'cba'⍋'azb?c'" => "5x 3x 1x 2x 4x",
        "(2 2⍴'ABBA')⍋3 2⍴'BAABBA'" => "1x 2x 3x",
    }
    fails(Rank, &["⍋1", "'a'⍋'ab'", "'ab'⍒'a'"]);
    fails(Domain, &["1 2⍋'ab'", "'ab'⍋1 2", "''⍋⍬"]);
}

#[test]
fn float_storage_and_kernels() {
    let mut s = Session::new();
    equiv_in! { &mut s;
        "v←⍳1000 ⋄ +/v+v" => "1001000",
        "×/1 2 3 4" => "24",
        "{⍺+⍵}/1E100 ¯1E100 1" => "0",
        "+/1x 2x 3x" => "6x",
        "0 1÷0 2" => "1 0.5",
    }
    for code in ["⍳3", "⌽⍳3", "2 3⍴⍳6", "⍬", "0 3⍴0"] { assert!(run(code).unwrap().unwrap().as_floats().is_some(), "{code}"); }
    for code in ["1x 2", "1j2 3", "'abc'", "(1 2)(3 4)", "0⍴1x"] { assert!(run(code).unwrap().unwrap().as_floats().is_none(), "{code}"); }
    fails(Domain, &["+/1E308 1E308", "×/1E308 1E308", "1÷0 1", "1E308×2 3"]);
    assert_eq!(Array::floats(vec![1], vec![f64::NAN]), Err(Domain));
    assert_eq!(Array::floats(vec![2], vec![1.]), Err(Length));
    assert_eq!(Array::floats(vec![1], vec![-0.]).unwrap().as_floats().unwrap()[0].to_bits(), 0);
}

#[test]
fn unicode_conversion() {
    equiv! {
        "⎕C 42 'Pete' 'Πέτρος'" => "42 'pete' 'πέτροσ'",
        "1⎕C 2 3⍴'aBcΣςß'" => "2 3⍴'ABCΣΣß'",
        "¯1⎕C 'İẞᾈΣ'" => "'ißᾀσ'",
        "⎕C 'ẞİﬀᾀ'" => "'ßİﬀᾀ'",
        "(1 1⍴¯3)⎕C 'ίσως'" => "'ίσωσ'",
        "⎕C 2 0⍴⊂'Ab'" => "2 0⍴⊂'  '",
        "u←⎕ucs ⋄ u 2 2⍴'A⍳λ😀'" => "2 2⍴65x 9075x 955x 128512x",
        "⎕UCS 2 2⍴65x 9075x 955x 128512x" => "2 2⍴'A⍳λ😀'",
        "⎕UCS 2 0⍴''" => "2 0⍴0x",
        "⎕UCS 0 3⍴0" => "0 3⍴''",
        "'UTF-8'⎕UCS 'Æ😀'" => "195x 134x 240x 159x 152x 128x",
        "('UTF-8' 0)⎕UCS 195 134 240 159 152 128" => "'Æ😀'",
        "'UTF-16'⎕UCS 'A😀'" => "65x 55357x 56832x",
        "'UTF-16'⎕UCS 65 55357 56832" => "'A😀'",
        "(⊂'UTF-32')⎕UCS 'A😀'" => "65x 128512x",
        "'UTF-32'⎕UCS 65 128512" => "'A😀'",
        "'UTF-8'⎕UCS 'A'" => ",65x",
        "'UTF-8'⎕UCS 65" => ",'A'",
        "'UTF-16'⎕UCS ⍬" => "''",
    }
    fails(
        Domain,
        &[
            "0⎕C 'a'",
            "1 2⎕C 'a'",
            "⎕UCS ¯1",
            "⎕UCS 55296",
            "⎕UCS 1114112",
            "⎕UCS 1.5",
            "⎕UCS 65 'B'",
            "⎕UCS 0⍴⊂'ab'",
            "'utf-8'⎕UCS 'a'",
            "'UTF-8'⎕UCS 192 128",
            "'UTF-8'⎕UCS 256",
            "'UTF-16'⎕UCS 55357",
            "'UTF-16'⎕UCS 65536",
        ],
    );
    fails(Rank, &["'UTF-8'⎕UCS 2 2⍴'a'"]);
    fails(Unsupported, &["('UTF-8' 83)⎕UCS 'abc'"]);
}

#[test]
fn characters_nesting_and_empty_fill() {
    equiv! { "⎕A" => "'ABCDEFGHIJKLMNOPQRSTUVWXYZ'", "⎕d" => "'0123456789'", "3↑⎕a" => "'ABC'" }
    fails(Syntax, &["⎕A←'abc'"]);
    fails(Unsupported, &["⎕IO←0"]);
    // Dyalog 20.0.53963.0, ⎕IO=1, ⎕CT=1E¯14. Expected structures are independent.
    for (code, expected) in [
        ("'a'", chars("a")),
        ("'界λ'", chars("界λ")),
        ("'can''t'", chars("can't")),
        ("''", chars("")),
        ("⍬", vector(&[])),
        ("⊃⍬", scalar(0.0)),
        ("⊃⊂1 2", vector(&[1.0, 2.0])),
        ("⊃0⍴(1 2)(3 4 5)", vector(&[0.0, 0.0])),
        ("3↑''", chars("   ")),
        ("¯4↑1 2", vector(&[0.0, 0.0, 1.0, 2.0])),
        ("2↓'abcd'", chars("cd")),
        ("¯2↓'abcd'", chars("ab")),
        ("5↓'ab'", chars("")),
        ("0⍴'abc'", chars("")),
        ("3⍴''", chars("   ")),
        ("⍬⍴1 2", scalar(1.0)),
        ("-''", vector(&[])),
        ("1÷''", vector(&[])),
        ("'ab'=1 2", ints(&[0, 0])),
        ("'abc'='axc'", ints(&[1, 0, 1])),
        ("+'abc'", chars("abc")),
        ("+''", chars("")),
        ("'abc' 'def' 'ghi'[2]", Array::new(vec![], vec![Nested(chars("def"))]).unwrap()),
        ("gg←2 3 4 5 ⋄ 9,gg[2],3 4", vector(&[9., 3., 3., 4.])),
    ] { check(code, expected); }
    let nested = Array::new(vec![], vec![Nested(vector(&[4.0, 6.0]))]).unwrap();
    let rows = Array::new(vec![2], vec![Nested(vector(&[1., 2.])), Nested(vector(&[3., 4.]))]).unwrap();
    let boxed = Array::new(vec![], vec![Nested(rows)]).unwrap();
    check("↓↓↓2 2⍴⍳4", Array::new(vec![], vec![Nested(boxed)]).unwrap());
    check("+/(1 2)(3 4)", nested);
    let a = run("(1 2)(3 4)+10").unwrap().unwrap();
    assert_eq!(a, Array::new(vec![2], vec![Nested(vector(&[11.0, 12.0])), Nested(vector(&[13.0, 14.0]))]).unwrap());
    check("0 3⍴⍬", Array::empty(vec![0, 3], number(0.0)).unwrap());
    assert_eq!(run("0↑1x").unwrap().unwrap().prototype(), exact(0, 1).prototype());
    fails(Syntax, &["'abc", "'a\nb'"]);
    fails(Domain, &["'a'+1", "'a'<'b'", "1.5↑1 2", "¯1⍴2"]);
    fails(Limit, &["1000001⍴1"]);
    assert!(matches!(parse(Source::new("quoted", "'({⍝⋄})'")), ParseStatus::Complete(_)));
}

#[test]
fn matrix_axes_scan_and_cell_assembly() {
    // Dyalog 20.0.53963.0. Direct primitive scans deliberately differ from equivalent dfns.
    equiv! {
        "+/2 3⍴⍳6" => "6 15",
        "+⌿2 3⍴⍳6" => "5 7 9",
        "-\\1 2 3" => "1 ¯1 2",
        "+\\1E100 ¯1E100 1" => "1E100 0 1",
        "{⍺+⍵}\\1E100 ¯1E100 1" => "1E100 0 0",
        "+/3 0⍴0" => "0 0 0",
        "+/0 3⍴0" => "⍬",
        "+\\2 3⍴⍳6" => "2 3⍴1 3 6 4 9 15",
        "+⍀2 3⍴⍳6" => "2 3⍴1 2 3 5 7 9",
        "+\\0 3⍴0" => "0 3⍴0",
        "↑(1 2)(3 4 5)" => "2 3⍴1 2 0 3 4 5",
        "↑1 (2 3)" => "2 2⍴1 0 2 3",
        "↑0⍴(1 2)(3 4 5)" => "0 2⍴0",
        "↑⊂1 2" => "1 2",
        "+/0⍴(1 2)(3 4)" => "⊂0 0",
        "+/''" => "0",
        "1 0⌿2 3⍴⍳6" => "1 3⍴1 2 3",
    }
    let r = Session::new().eval("f←{⎕←⍺ ⍵ ⋄ ⍺-⍵} ⋄ f\\1 2 3");
    assert_eq!(r.output, ["1 2", "2 3", "1 ¯1", "1 ¯1 2"]);
    equiv("m←2 3⍴⍳6 ⋄ r←⌿ ⋄ +r m", "5 7 9");
}

#[test]
fn dyalog_array_literals_and_completeness() {
    for (code, expected) in [
        ("(42 ⋄)", vector(&[42.])),
        ("(⋄42)", vector(&[42.])),
        ("(1 ⋄ ⋄ 2)", vector(&[1., 2.])),
        ("(\n42\n)", vector(&[42.])),
        ("[1 2 ⋄ 3]", Array::new(vec![2, 2], [1., 2., 3., 0.].into_iter().map(number).collect()).unwrap()),
        ("[42 ⋄]", Array::new(vec![1, 1], vec![number(42.)]).unwrap()),
        ("[1 ⋄ 2]", Array::new(vec![2, 1], [1., 2.].into_iter().map(number).collect()).unwrap()),
        ("[\n{a←⍵+1 ⋄ a}2 3 ⍝ first row\n5\n]", Array::new(vec![2, 2], [3., 4., 5., 0.].into_iter().map(number).collect()).unwrap()),
        ("(a←2 ⋄ a+3)", vector(&[2., 5.])),
    ] { check(code, expected); }
    let mut s = Session::new();
    let r = s.eval("(⎕←1 ⋄ ⎕←2)");
    assert_eq!(r.output, ["1", "2", "1 2"]);
    check("(1 2 ⋄ 3 4)", Array::new(vec![2], vec![Nested(vector(&[1., 2.])), Nested(vector(&[3., 4.]))]).unwrap());
    for code in ["[1 2 ⋄", "(1 ⋄", "[({⍵}1 ⋄ 2) ⋄"] { assert!(matches!(parse(Source::new("partial", code)), ParseStatus::Incomplete(_))); }
    for code in ["[1 ⋄ 2)", "(1 ⋄ 2]", "[⋄]"] { assert!(matches!(parse(Source::new("invalid", code)), ParseStatus::Invalid(_))); }
}

#[test]
fn structural_slices_and_brackets() {
    equiv! {
        "⍉[1 2 3 ⋄ 4 5 6]" => "3 2⍴1 4 2 5 3 6",
        "⌽[1 2 3 ⋄ 4 5 6]" => "2 3⍴3 2 1 6 5 4",
        "⊖[1 2 3 ⋄ 4 5 6]" => "2 3⍴4 5 6 1 2 3",
        "1 2⌽[1 2 3 ⋄ 4 5 6]" => "2 3⍴2 3 1 6 4 5",
        "[1 2 3 ⋄ 4 5 6],8 9" => "2 4⍴1 2 3 8 4 5 6 9",
        "1 2↑[1 2 3 ⋄ 4 5 6]" => "1 2⍴1 2",
        "¯3 ¯2↑[1 2 3 ⋄ 4 5 6]" => "3 2⍴0 0 2 3 5 6",
        "1 ¯1↓[1 2 3 ⋄ 4 5 6]" => "1 2⍴4 5",
        "2 3↑7" => "2 3⍴7 0 0 0 0 0",
        "[1 2 3 ⋄ 4 5 6][2;1]" => "4",
        "[1 2 3 ⋄ 4 5 6][;2]" => "2 5",
        "[1 2 3 ⋄ 4 5 6][2;]" => "4 5 6",
        "(10 20 30)[3 1]" => "30 10",
        "+/[1][1 2 3 ⋄ 4 5 6]" => "5 7 9",
        "s←+/ ⋄ s[1][1 2 3 ⋄ 4 5 6]" => "5 7 9",
        "1 1⍉[1 2 3 ⋄ 4 5 6]" => "1 5",
        "2 ¯1 1/10 20 30" => "10 10 0 30",
        "2 ¯1 1\\10 20" => "10 10 0 20",
    }
    equiv("1 0 1\\'ab'", "'a b'");
    fails(Index, &["(1 2)[0]", "(1 2)[¯1]", "(1 2)[3]"]);
    fails(Domain, &["(1 2)[1.5]"]);
    fails(Rank, &["[1 2 ⋄ 3 4][1]"]);
    fails(Domain, &["+/[0][1 2 ⋄ 3 4]"]);
    for shape in [vec![0, 3], vec![3, 0], vec![2, 3], vec![2, 2, 3]] {
        let shape = shape.iter().map(usize::to_string).collect::<Vec<_>>().join(" ");
        let source = format!("{shape}⍴⍳12");
        for op in ["⌽⌽", "⊖⊖", "⍉⍉", "↑↓"] { assert_eq!(run(&format!("{op}{source}")).unwrap(), run(&source).unwrap()); }
    }
    equiv("[({⍵=0:1 ⋄ ⍵+2}0 ⋄ 3) ⋄ (4 ⋄ 5)][;1]", "1 4");
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
        ("(/ {+⍺⍺ ⍵})1 2 3", Some(6.), vec!["6"]),
    ] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value, expected.map(scalar), "{code}");
        assert_eq!(r.output, output, "{code}");
    }
    fails_in(&mut s, Value, &["x←{}0", "1+{}0"]);
    fails_in(&mut s, Domain, &["{6::7 ⋄ 1÷⍵}0"]);
    fails_in(&mut s, Syntax, &["{1:1:2}0"]);
    fails_in(&mut s, Value, &["10{g←{⍺+⍵} ⋄ g ⍵}3"]);
}

fn exact(n: i64, d: i64) -> Array { Array::scalar(num_rational::BigRational::new(n.into(), d.into())).unwrap() }

#[test]
fn scalar_math() {
    equiv! {
        "⌊1.5j0.5" => "1j1",
        "0.2=3.8j7.6∨5.2j6.8" => "1x",
        "⌈1000×3.8j7.6∧5.2j6.8" => "¯159600j326800",
        "|3 ¯3 3J4" => "3 3 5",
        "2 10 ¯2.5|7 ¯13 8" => "1 7 ¯2",
        "0 3|¯2 6" => "¯2 0",
        "⌊1.000000000000001 ¯1.000000000000001" => "1 ¯1",
        "⌈¯2.3 0.1 3" => "¯2 1 3",
        "2 3⌊3 2" => "2 2",
        "2 3⌈3 2" => "3 3",
        "2 ¯2 0*3 3 0" => "8 ¯8 1",
        "9 11○3J4" => "3 4",
    }
    for (code, value) in [("*1", std::f64::consts::E), ("2⍟32", 5.), ("○1", std::f64::consts::PI), ("¯1○1", std::f64::consts::FRAC_PI_2)] {
        let a = run(code).unwrap().unwrap();
        assert!((a.as_number().unwrap().as_float().unwrap() - value).abs() < 1e-14, "{code}");
    }
    for code in ["1E¯13>|(¯4*0.5)-0J2", "(⌊3.3J2.5)=3J2", "(⌈3.3J2.5)=3J3", "0=0.1|0.3", "0=3|6.000000000000001", "0=1+*○0j1"] { equiv(code, "1x"); }
    equiv! {
        "2x*¯3x" => "1r8",
        "2r3|7r3" => "1r3",
        "⌊¯4r3" => "¯2x",
        "⌈¯4r3" => "¯1x",
    }
    fails(Domain, &["⍟0", "0*¯1", "13○1", "0J1⌊2"]);
}

#[test]
fn search_depth_and_random() {
    equiv! {
        "x←1 ⋄ y←1+8E¯15 ⋄ z←1+16E¯15 ⋄ x y⍳z" => "2x",
        "≠1 (1+8E¯15) (1+16E¯15)" => "1x 0x 1x",
        "1 2~1+8E¯15" => ",2",
        "1x 2x~1000000000000001r1000000000000000" => "1x 2x",
        "(3 0⍴0)⍳2 0⍴0" => "1x 1x",
        "≠3 0⍴0" => "1x 0x 0x",
        "⍸0 1 0 2" => "2x 4x 4x",
        "1 2⍸1-1E¯15" => "0x",
        "≡0⍴(1 2)3" => "2",
        "1≢,1" => "1x",
        "1x≡1" => "1x",
        "(0⍴1x)≡⍬" => "1x",
        "⍳,3" => "1 2 3",
        "1 1∪2 2" => "1 1 2 2",
        "1 2∊1+8E¯15" => "1x 0x",
        "(,1)⍷1" => "0x",
        "(2 0⍴0)⍷1 3⍴1" => "1 3⍴0x",
        "⍳⍬" => "⊂⍬",
        "⍸0" => "0⍴⊂0⍴0x",
    }
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
    fails(Domain, &["3?2", "?¯1", "⍸¯1"]);
    fails(Rank, &["2⍳2"]);
    fails(Length, &["(2 3⍴0)⍳1 2"]);
    fails(Rank, &["(2 2⍴1)~1"]);
    fails(Domain, &["2 1⍸1"]);
}

#[test]
fn each_commute_and_reduction() {
    equiv! {
        "e←¨ ⋄ sum←+/ ⋄ sum e (1 2)(3 4 5)" => "3 12",
        "+/¨¨(1 2)(3 4)" => "(1 2)(3 4)",
        "2⍨3" => "2",
        "2-⍨5" => "3",
        "3+/1 2" => "⍬",
        "2+⌿2 3⍴⍳6" => "1 3⍴5 7 9",
        "-/⍬" => "0",
        "÷/⍬" => "1",
    }
    check("⌊/⍬", scalar(f64::MAX));
    check("⌈/⍬", scalar(-f64::MAX));
    let mut session = Session::new();
    let result = session.eval("r←{⎕←7 ⋄ ⍵}¨⍬");
    assert!(result.error.is_none());
    assert_eq!(result.output, ["7"]);
    assert_eq!(result.value.unwrap(), vector(&[]));
    assert!(session.eval("{⍵=2:{}⍵ ⋄ ⍵}¨1 2 3").value.is_none());
    fails(Domain, &["÷¨⍬", "2¨3"]);
    fails(Rank, &["0+/5"]);
    fails(Length, &["4+/1 2", "3+/⍬"]);
    fails(Syntax, &["2+\\1 2"]);
}

#[test]
fn composition_rank_and_dyadic_operators() {
    equiv! {
        "c←∘ ⋄ sum←+/c⍳ ⋄ sum¨2 4 6" => "3 10 21",
        "'abc'⍴⍛⍴'z'" => "'zzz'",
        "¯11∘○⍛+⌿2 3⍴1 2 3 4 5 6" => "4j1 5j2 6j3",
        "c←∘ ⋄ b←⍛ ⋄ r←⌿ ⋄ ¯11 c ○ b + r 2 3⍴1 2 3 4 5 6" => "4j1 5j2 6j3",
        "-∘+∘×/1 2 3" => "0",
        "⍳⍤0⊢1 3 2" => "3 3⍴1 0 0 1 2 3 1 2 0",
        "({⍳3}⍤1)0 2⍴0" => "0 3⍴0",
        "op←{⍺⍺+⍵⍵×⍵} ⋄ (2 op 3)4" => "14",
        "op←{⍺⍺ ⍵⍵ ⍵} ⋄ (+/op⍳)4" => "10",
        "f←{k←3 ⋄ g←{k+⍵} ⋄ op←{⍺⍺ ⍵⍵ ⍵} ⋄ (+op g)⍵} ⋄ f 4" => "7",
    }
    let mut session = Session::new();
    let r = session.eval("{⎕←⍵ ⋄ ⍳3}⍤0⊢⍬");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["0", "⍬"]);
    let r = session.eval("f←{⎕←⍵ ⋄ ⍵} ⋄ 2 +⍥f 3");
    assert_eq!(r.output, ["3", "2", "5"]);
    fails(Length, &["+⍤⍬⊢3"]);
    fails(Domain, &["+⍤0.5⊢3"]);
}

#[test]
fn key_and_power() {
    equiv! {
        "1 1 2{+/⍵}⌸10 20 30" => "30 30",
        "{⍳3}⌸⍬" => "0 3⍴0",
        "1(+⍣{⍺>4})0" => "5",
        "(2∘×⍣0)3" => "3",
        "{≢⍵}⌸1 (1+8E¯15)(1+16E¯15)" => "2x 1x",
    }
    let mut s = Session::new();
    let result = s.eval("r←⍬ {⎕←⍴⍵ ⋄ ⍳3}⌸0 2⍴0");
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.output, ["0 2"]);
    assert_eq!(s.eval("({⎕←7 ⋄ ⍵}⍣0)3").output, ["3"]);
    assert!(s.eval("({}⍣1)3").value.is_none());
    fails(Rank, &["{⍺ ⍵}⌸7"]);
    fails(Length, &["1 2{⍵}⌸3 4 5"]);
    fails(Domain, &["(+⍣0.5)1"]);
    fails(Rank, &["(+⍣(,1))2"]);
}

#[test]
fn products() {
    equiv! {
        "1 2 3,.-3 3⍴4 5 6" => "(¯3 ¯2 ¯1)(¯4 ¯3 ¯2)(¯5 ¯4 ¯3)",
        "(⍳∘≢(∘.⌷)⊂)2 3 3⍴⍳18" => "(3 3⍴1 2 3 4 5 6 7 8 9)(3 3⍴10 11 12 13 14 15 16 17 18)",
        "(2 3⍴⍳6)+.×3 2⍴⍳6" => "2 2⍴22 28 49 64",
        "(2 0⍴0)+.×0 3⍴0" => "2 3⍴0",
        "(,2)+.×1 2 3" => "12",
        "jot←∘ ⋄ dot←. ⋄ times←× ⋄ 2 jot dot times 3" => "6",
        "⍬∘.+7 8" => "0 2⍴0",
    }
    let mut s = Session::new();
    let r = s.eval("r←⍬∘.{⎕←⍺ ⍵ ⋄ ⍺+⍵}7 8");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["0 7"]);
    let r = s.eval("r←(0 2⍴0)+.{⎕←⍺ ⍵ ⋄ ⍺×⍵}2 3⍴⍳6");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["0 1", "0 2", "0 3", "0 4", "0 5", "0 6"]);
    fails(Length, &["1 2+.×1 2 3"]);
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
        let expected = if im == 0.0 { scalar(re) } else { Array::scalar(num_complex::Complex64::new(re, im)).unwrap() };
        assert_eq!(a, expected, "{code}");
        assert_eq!(run(&a.to_string()).unwrap().unwrap(), a, "display round-trip: {code}");
    }
    assert_eq!(run("1J2×1J¯2").unwrap().unwrap().to_string(), "5");
    assert_eq!(run("¯0J2").unwrap().unwrap().to_string(), "0j2");
    let a = run("1x 0.5 1J2").unwrap().unwrap();
    assert_eq!(a.elements().collect::<Vec<_>>(), &[exact(1, 1).at(0), number(0.5), Element::Number(num_complex::Complex64::new(1.0, 2.0).try_into().unwrap())]);
    for code in ["0/1J2", "1J2+0/1x", "+/0/1J2"] { assert_eq!(run(code).unwrap().unwrap().prototype(), &number(0.0)); }
    equiv! {
        "×/0/1J2" => "1",
        "⍳3J0" => "1 2 3",
    }
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
    fails(Syntax, &["1J", "1J¯", "1J2E¯", "1J2x", "1xJ2", "1r2J3", "1J2J3"]);
    fails(Domain, &["1J1E309", "1E309J1", "1E308J1E308+1E308J1E308", "1J2÷0", "÷0J0", "⍳1J1E¯15", "1J2/3", "1J2<1J2", "1J2≤2", "2>1J2", "2≥1J2"]);
    fails(Domain, &[&format!("1{}x+0J1", "0".repeat(400))]);
    assert_eq!(Array::scalar(num_complex::Complex64::new(0.0, f64::INFINITY)), Err(Domain));
    let mut s = Session::new();
    let failed = s.eval("⎕←1J2 ⋄ 1J2÷0");
    assert_eq!(failed.output, ["1j2"]);
    assert_eq!(failed.error.unwrap().kind, Domain);
    assert_eq!(s.eval("1J2+3J4").output, ["4j6"]);
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
    equiv! {
        "1x÷2" => "0.5",
        "(1÷3)+(1÷6)" => "0.5",
        "1r4+0.5" => "0.75",
    }
    let huge = format!("1{}", "0".repeat(400));
    assert_eq!(run(&format!("{huge}x÷{huge}x")).unwrap().unwrap(), exact(1, 1));
    fails(Domain, &[&format!("{huge}x+0")]);
    assert_eq!(run(&format!("{huge}1r{huge}0+0.5")).unwrap().unwrap(), scalar(1.5));
    fails(Domain, &["1r0", "0r0", "1x÷0x", "÷0x"]);
    fails(Syntax, &["1r", "1r¯", "1.5x", "1E2x", "1r2.5", "1r2r3", "1x2"]);
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
    equiv! {
        "+/0/1r3" => "0x",
        "×/0/1r3" => "1x",
    }
    check("1x+0/1r3", empty);
    assert_eq!(run("1+0/1r3").unwrap().unwrap().prototype(), &number(0.0));
    equiv("⍳3x", "1x 2x 3x");
    assert_eq!(run("2x/1r3").unwrap().unwrap().elements().collect::<Vec<_>>(), vec![exact(1, 3).at(0); 2]);
    fails(Domain, &["⍳3r2", "⍳¯1x"]);
    fails(Limit, &["⍳1000001x", "⍳999999999999999999999x"]);
    fails(Domain, &["1r2/3"]);
    let invalid = num_rational::BigRational::new_raw(1.into(), 0.into());
    assert_eq!(Array::scalar(invalid), Err(Domain));
    let raw = num_rational::BigRational::new_raw(2.into(), (-4).into());
    assert_eq!(scalar(raw), exact(-1, 2));
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
            check(&code, exact(i64::from(expected), 1));
        }
    }
    equiv! {
        "0.3=0.3 (0.1+0.2) 0.4" => "1x 1x 0x",
        "1 2≤2 1" => "1x 0x",
        "(1=1+8E¯15)((1+8E¯15)=1+16E¯15)(1=1+16E¯15)" => "1x 1x 0x",
    }
    assert_eq!(run("1x=0/1x").unwrap().unwrap().prototype(), exact(0, 1).prototype());
    fails(Syntax, &["=⍳0"]);
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
        ("(2×3) + 4 ⍝ comment )", 10.0),
        // Dyalog 20, ⎕DIV=0; documentation-derived, not reference-interpreter executions:
        // https://docs.dyalog.com/20.0/language-reference-guide/system-functions/div/
        ("0÷0", 1.0),
        ("0÷¯0", 1.0),
        ("¯0", 0.0),
    ] {
        let a = run(code).unwrap().unwrap();
        assert_eq!(a, scalar(expected), "{code}");
    }
    assert_eq!(run("⍝ only a comment").unwrap(), None);
    assert_eq!(run("").unwrap(), None);
    assert_eq!(run("¯0").unwrap().unwrap().to_string(), "0");
    assert_eq!(run("¯2").unwrap().unwrap().to_string(), "¯2");
}

#[test]
fn errors_and_evaluation_order() {
    fails(Domain, &["1÷0", "÷0", "1e309", "1e308×2"]);
    fails(Syntax, &["1e", "1e-2", "¯", ".", "2+", ")", "(2+3"]);
    fails(Length, &["1 2+3 4 5"]);
    fails(Domain, &["⍳¯1"]);
    // The right argument fails before the parenthesized left argument is evaluated.
    let e = run("(1÷0)+(2×1e308)").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "×");
    let e = run("¯2+1÷0").unwrap_err();
    assert_eq!(e.span.range, 5..7); // UTF-8 bytes, not glyph indices.
    assert_eq!(e.to_string(), "DOMAIN ERROR: division by zero\n --> test:1:5\n¯2+1÷0\n    ^");
    let e = run("(2\n 1÷0)").unwrap_err();
    assert!(e.to_string().contains("test:2:3\n 1÷0)\n  ^"));
    equiv("2+2", "4.0");
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
    fails(Limit, &[&deep]);
    // Flat evaluation is iterative, not one Rust stack frame per function application.
    equiv(&format!("{}1", "1+".repeat(10_000)), "10001");
}

#[test]
fn array_invariants() {
    let scalar = scalar(7.0);
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
    assert!(rows.is_empty());
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
    let mut detached: Vec<Element> = nested.elements().collect();
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
    equiv_in! { &mut s;
        "+/v" => "55",
        "f←+ ⋄ 2 f 3" => "5",
        "sum←+/ ⋄ sum 1 2 3" => "6",
        "f/1 2 3" => "6",
        "-/1 2 3" => "2",
        "+/⍳0" => "0",
        "×/⍳0" => "1",
        "+/7" => "7",
        "1 2 3+10" => "11 12 13",
        "10-1 2 3" => "9 8 7",
        "1 2+3 4" => "4 6",
        "⍴v" => ",10",
        ",7" => ",7",
        "a←v ⋄ v←0 ⋄ +/a" => "55",
        "≢,7" => "1",
    }
    assert_eq!(s.eval("⍴7").value.unwrap().shape(), &[0]);
    assert_eq!(s.eval("2+⍳0").value.unwrap().shape(), &[0]);
    fails_in(&mut s, Domain, &["⍳1.5"]);
    fails_in(&mut s, Limit, &["⍳1000001"]);
    fails_in(&mut s, Value, &["missing"]);
    fails_in(&mut s, Domain, &["{⍺-⍵}/⍳0"]);
}

#[test]
fn result_output_and_nonexecuting_parse() {
    let mut s = Session::new();
    equiv_in(&mut s, "x←2", "2");
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
    equiv_in(&mut s, "x", "2");
    assert!(s.eval("").value.is_none());
    assert!(s.eval("2+2").error.is_none());
    let r = s.eval("x←5 ⋄ 1÷0");
    assert!(r.error.is_some());
    equiv_in(&mut s, "x", "5");
    // No whole-input transaction.
}

#[test]
fn executing_binding_gate() {
    let mut s = Session::new();
    equiv_in! { &mut s;
        "add←{⍺+⍵} ⋄ add/1 2 3" => "6",
        "avg←+/÷≢ ⋄ avg 2 4 9" => "5",
        "apply←{⍺⍺ ⍵} ⋄ (-apply)3" => "¯3",
        "offset←{⍺⍺+⍵} ⋄ (2 offset)3" => "5",
        "inc←{later ⍵} ⋄ later←{1+⍵} ⋄ inc 4" => "5",
        "x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller 0" => "10",
        "total←add/ ⋄ total 1 2 3" => "6",
        "{2×⍵}3" => "6",
    }
    for code in ["1 0 1/2 4 6", "rep←/ ⋄ 1 0 1 rep 2 4 6"] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&[2.0, 6.0]));
    }
    assert_eq!(s.eval("f←{⎕←1 ⋄ ⍵} ⋄ h←{⎕←2 ⋄ ⍵} ⋄ t←f+h ⋄ t 3").output, ["2", "1", "6"]);
    let r = s.eval("bad←{local←99 ⋄ 1÷0} ⋄ bad 0");
    assert_eq!(r.error.unwrap().kind, Domain);
    fails_in(&mut s, Value, &["local"]);
    equiv_in(&mut s, "x", "10");
    fails_in(&mut s, Limit, &["loop←{1+∇⍵} ⋄ loop 0"]);
    equiv_in(&mut s, "2+2", "4");
    fails_in(&mut s, Syntax, &["{⍵←1 ⋄ ⍵}2"]);
    assert_eq!(s.eval("outer←{inner←{⍵} ⋄ inner ⍵} ⋄ outer 1").value.unwrap(), scalar(1.0));
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
        assert_eq!(r.value.unwrap(), scalar(6.0));
    }
    for (code, expected) in [("1 0 1 r 2 4 6", vec![2.0, 6.0]), ("(,2)/3 4", vec![3.0, 3.0, 4.0, 4.0]), ("1 0 1/,3", vec![3.0, 3.0])] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&expected));
    }
    assert!(s.eval("op←{⍵⍵ ⍵}").error.is_none());
    assert_eq!(s.eval("(+op-)3").value.unwrap(), scalar(-3.));
    assert!(s.eval("outer←{inner←{⍵⍵ ⍵} ⋄ 1}").error.is_none());
    assert_eq!(s.eval("outer 0").value.unwrap(), scalar(1.));
}

#[test]
fn singleton_agreement_and_empty_counts() {
    // Shared Dyalog cases; additional leading/unit-axis broadcasting is tested separately.
    equiv! {
        "(,2)+3 4" => "5 6",
        "3 4+,2" => "5 6",
        "2+,3" => ",5",
        "(,2)/⍳0" => "⍬",
        "(⍳0)/,3" => "⍬",
        "(,2)+⍳0" => "⍬",
        "1000001/⍳0" => "⍬",
    }
    fails(Domain, &["0.5/⍳0"]);
    fails(Length, &["2 3/1 2 3"]);
    fails(Limit, &["1000001/1", "99999999999999999999x/1"]);
}

#[test]
fn binder_limits_and_single_execution() {
    let mut s = Session::new();
    assert_eq!(s.eval("x←1 ⋄ (⎕←x)+(⎕←(x←2))").output, ["2", "2", "4"]);
    assert_eq!(s.eval("apply←{⍺⍺ ⍵} ⋄ f←{⎕←⍵ ⋄ ⍵} ⋄ (f apply)/1 2 3").output, ["3", "3", "3"]);
    assert_eq!(s.eval("offset←{+/⍺⍺+⍵} ⋄ (1 2 offset)3").value.unwrap(), scalar(9.0));
    assert_eq!(s.eval(&format!("{}7", "a←".repeat(10_000))).value.unwrap(), scalar(7.0));
    fails_in(&mut s, Limit, &[&format!("+{}1", "/".repeat(10_000))]);
    fails_in(&mut s, Limit, &[&format!("f←+ ⋄ {} f 0", "f←f+f ⋄ ".repeat(127))]);
    assert_eq!(s.eval("2+2").value.unwrap(), scalar(4.0));
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
    assert!(weak.upgrade().is_some());
    let result = s.eval("f 0");
    assert!(result.error.is_none() && result.value.is_none());
    s.eval("f←+");
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
        assert_eq!(r.value.unwrap(), scalar(expected), "{code}");
    }
    let r = s.eval("guarded←{0::7 ⋄ ⎕←2 ⋄ 1÷⍵} ⋄ guarded 0");
    assert_eq!(r.output, ["2", "7"]); // Output is not transactional.
    fails_in(&mut s, Domain, &["{0::1÷0 ⋄ 1÷⍵}0"]);
    fails_in(&mut s, Value, &["{0::fresh ⋄ fresh←1 ⋄ 1÷⍵}0"]);
    fails_in(&mut s, Domain, &["{2:1 ⋄ 0}0"]);
    fails_in(&mut s, Syntax, &["{x←2 ⋄ {x+⍵}}0", "{1:+ ⋄ 0}0"]);
    assert!(matches!(parse(Source::new("guard", "f←{0::⎕←1}")), ParseStatus::Complete(_)));
    assert_eq!(s.eval("2+2").value.unwrap(), scalar(4.0));
}
