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
fn scalar(n: impl TryInto<miniapl::Number>) -> Array { Array::scalar(n).unwrap() }
fn vector(values: &[f64]) -> Array { Array::from_parts(vec![values.len()], values.iter().copied().map(number).collect(), number(0.0)).unwrap() }

#[track_caller]
fn check_in(session: &mut Session, code: &str, expected: Array) {
    let result = session.eval(code);
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    assert_eq!(result.value, Some(expected), "{code}");
}

#[track_caller]
fn check(code: &str, expected: Array) { check_in(&mut Session::new(), code, expected); }

fn chars(text: &str) -> Array {
    let data: Vec<_> = text.chars().map(Character).collect();
    let shape = if data.len() == 1 { vec![] } else { vec![data.len()] };
    Array::from_parts(shape, data, Character(' ')).unwrap()
}

#[test]
fn pick_and_partition() {
    let nested = |shape, cells: Vec<Array>| Array::new(shape, cells.into_iter().map(Nested).collect()).unwrap();
    check("((2 1)1 2)⊃2 3⍴('ABC' 1)('DEF' 2)('GHI' 3)('JKL' 4)('MNO' 5)('PQR' 6)", chars("K"));
    check("(5⍴⊂⍬)⊃10", scalar(10.));
    check("⍬⊃1 2", vector(&[1., 2.]));
    check("2⌷3 4⍴⍳12", vector(&[5., 6., 7., 8.]));
    check("2⌷[2]3 4⍴⍳12", vector(&[2., 6., 10.]));
    check("⍴⍬ ⍬⌷3 4⍴0", vector(&[0., 0.]));
    check("⍬⌷1", scalar(1.));
    check("⌷1 2", vector(&[1., 2.]));
    check("⊆1 2", nested(vec![], vec![vector(&[1., 2.])]));
    check("⊆1", scalar(1.));
    check("⊆(1 2)(3 4)", nested(vec![2], vec![vector(&[1., 2.]), vector(&[3., 4.])]));
    for code in ["(,1)⊂'abcd'", "1⊆'abcd'", "(,1)⊆'abcd'"] { check(code, nested(vec![1], vec![chars("abcd")])); }
    check("3 2 2 1 0 1⊆'abcdef'", nested(vec![2], vec![chars("abcd"), Array::new(vec![1], vec![Character('f')]).unwrap()]));
    check("1 1 0 1⊆[1]4 2⍴⍳8", nested(vec![2, 2], vec![vector(&[1., 3.]), vector(&[2., 4.]), vector(&[7.]), vector(&[8.])]));
    check("0⊂2 3⍴0", Array::empty(vec![0], Nested(Array::empty(vec![2, 0], number(0.)).unwrap())).unwrap());
    check("1 0 1⊆0 3⍴0", Array::empty(vec![0, 2], Nested(vector(&[]))).unwrap());
    for (code, kind) in [("0⊃1 2", Index), ("1 1⊃1 2", Length), ("1⊆3", Rank), ("¯1⊂1 2", Domain), ("1 0⊆1 2 3", Length), ("1 2⌷1 2", Length), ("0⌷1 2", Index)]
    { assert_eq!(run(code).unwrap_err().kind, kind, "{code}"); }
}

#[test]
fn grading() {
    for (code, expected) in [
        ("⍒2 1 2 1", vec![1., 3., 2., 4.]),
        ("⍋1 (1+8E¯15) 1", vec![1., 3., 2.]),
        ("⍋1j2 1 1j¯2 'a' 0", vec![5., 3., 2., 1., 4.]),
        ("⍋9007199254740993x 9007199254740992 9007199254740992x", vec![2., 3., 1.]),
        ("⍋(2 2⍴1 2 3 4)(1 4⍴1 2 0 0)", vec![1., 2.]),
        ("⍋(1 2⍴1 2)(1 2)", vec![2., 1.]),
        ("⍋(0 5 2⍴0)(0 3 4⍴0)(0 1⍴'')⍬", vec![4., 1., 2., 3.]),
        ("⍋(0⍴⊂1 2)(0⍴0)(0⍴'')", vec![2., 1., 3.]),
        ("⍋3 0⍴0", vec![1., 2., 3.]),
        ("⍋⍬", vec![]),
        ("'cba'⍋'azb?c'", vec![5., 3., 1., 2., 4.]),
        ("(2 2⍴'ABBA')⍋3 2⍴'BAABBA'", vec![1., 2., 3.]),
    ] { check(code, vector(&expected)); }
    for code in ["⍋1", "'a'⍋'ab'", "'ab'⍒'a'"] { assert_eq!(run(code).unwrap_err().kind, Rank, "{code}"); }
    for code in ["1 2⍋'ab'", "'ab'⍋1 2", "''⍋⍬"] { assert_eq!(run(code).unwrap_err().kind, Domain, "{code}"); }
}

#[test]
fn float_storage_and_kernels() {
    let mut s = Session::new();
    check_in(&mut s, "v←⍳1000 ⋄ +/v+v", scalar(1_001_000.));
    check_in(&mut s, "×/1 2 3 4", scalar(24.));
    check_in(&mut s, "{⍺+⍵}/1E100 ¯1E100 1", scalar(0.));
    check_in(&mut s, "+/1x 2x 3x", exact(6, 1));
    check_in(&mut s, "0 1÷0 2", vector(&[1., 0.5]));
    for code in ["⍳3", "⌽⍳3", "2 3⍴⍳6", "⍬", "0 3⍴0"] { assert!(run(code).unwrap().unwrap().as_floats().is_some(), "{code}"); }
    for code in ["1x 2", "1j2 3", "'abc'", "(1 2)(3 4)", "0⍴1x"] { assert!(run(code).unwrap().unwrap().as_floats().is_none(), "{code}"); }
    for code in ["+/1E308 1E308", "×/1E308 1E308", "1÷0 1", "1E308×2 3"] { assert_eq!(run(code).unwrap_err().kind, Domain, "{code}"); }
    assert_eq!(Array::floats(vec![1], vec![f64::NAN]), Err(Domain));
    assert_eq!(Array::floats(vec![2], vec![1.]), Err(Length));
    assert_eq!(Array::floats(vec![1], vec![-0.]).unwrap().as_floats().unwrap()[0].to_bits(), 0);
}

#[test]
fn characters_nesting_and_empty_fill() {
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
        ("'ab'=1 2", vector(&[0.0, 0.0])),
        ("'abc'='axc'", vector(&[1.0, 0.0, 1.0])),
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
    for (code, kind) in
        [("'abc", Syntax), ("'a\nb'", Syntax), ("'a'+1", Domain), ("'a'<'b'", Domain), ("1.5↑1 2", Domain), ("¯1⍴2", Domain), ("1000001⍴1", Limit)]
    { assert_eq!(run(code).unwrap_err().kind, kind, "{code}"); }
    assert!(matches!(parse(Source::new("quoted", "'({⍝⋄})'")), ParseStatus::Complete(_)));
}

#[test]
fn matrix_axes_scan_and_cell_assembly() {
    // Dyalog 20.0.53963.0. Direct primitive scans deliberately differ from equivalent dfns.
    for (code, expected) in [
        ("+/2 3⍴⍳6", vector(&[6.0, 15.0])),
        ("+⌿2 3⍴⍳6", vector(&[5.0, 7.0, 9.0])),
        ("-\\1 2 3", vector(&[1.0, -1.0, 2.0])),
        ("+\\1E100 ¯1E100 1", vector(&[1e100, 0.0, 1.0])),
        ("{⍺+⍵}\\1E100 ¯1E100 1", vector(&[1e100, 0.0, 0.0])),
        ("+/3 0⍴0", vector(&[0.0, 0.0, 0.0])),
        ("+/0 3⍴0", vector(&[])),
        ("+\\2 3⍴⍳6", Array::new(vec![2, 3], [1., 3., 6., 4., 9., 15.].into_iter().map(number).collect()).unwrap()),
        ("+⍀2 3⍴⍳6", Array::new(vec![2, 3], [1., 2., 3., 5., 7., 9.].into_iter().map(number).collect()).unwrap()),
        ("+\\0 3⍴0", Array::empty(vec![0, 3], number(0.0)).unwrap()),
        ("↑(1 2)(3 4 5)", Array::new(vec![2, 3], [1., 2., 0., 3., 4., 5.].into_iter().map(number).collect()).unwrap()),
        ("↑1 (2 3)", Array::new(vec![2, 2], [1., 0., 2., 3.].into_iter().map(number).collect()).unwrap()),
        ("↑0⍴(1 2)(3 4 5)", Array::empty(vec![0, 2], number(0.0)).unwrap()),
        ("↑⊂1 2", vector(&[1.0, 2.0])),
        ("+/0⍴(1 2)(3 4)", Array::new(vec![], vec![Nested(vector(&[0., 0.]))]).unwrap()),
        ("+/''", scalar(0.0)),
        ("1 0⌿2 3⍴⍳6", Array::new(vec![1, 3], [1., 2., 3.].into_iter().map(number).collect()).unwrap()),
    ] { check(code, expected); }
    let r = Session::new().eval("f←{⎕←⍺ ⍵ ⋄ ⍺-⍵} ⋄ f\\1 2 3");
    assert_eq!(r.output, ["1 2", "2 3", "1 ¯1", "1 ¯1 2"]);
    check("m←2 3⍴⍳6 ⋄ r←⌿ ⋄ +r m", vector(&[5., 7., 9.]));
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
    for (code, shape, values) in [
        ("⍉[1 2 3 ⋄ 4 5 6]", vec![3, 2], vec![1., 4., 2., 5., 3., 6.]),
        ("⌽[1 2 3 ⋄ 4 5 6]", vec![2, 3], vec![3., 2., 1., 6., 5., 4.]),
        ("⊖[1 2 3 ⋄ 4 5 6]", vec![2, 3], vec![4., 5., 6., 1., 2., 3.]),
        ("1 2⌽[1 2 3 ⋄ 4 5 6]", vec![2, 3], vec![2., 3., 1., 6., 4., 5.]),
        ("[1 2 3 ⋄ 4 5 6],8 9", vec![2, 4], vec![1., 2., 3., 8., 4., 5., 6., 9.]),
        ("1 2↑[1 2 3 ⋄ 4 5 6]", vec![1, 2], vec![1., 2.]),
        ("¯3 ¯2↑[1 2 3 ⋄ 4 5 6]", vec![3, 2], vec![0., 0., 2., 3., 5., 6.]),
        ("1 ¯1↓[1 2 3 ⋄ 4 5 6]", vec![1, 2], vec![4., 5.]),
        ("2 3↑7", vec![2, 3], vec![7., 0., 0., 0., 0., 0.]),
        ("[1 2 3 ⋄ 4 5 6][2;1]", vec![], vec![4.]),
        ("[1 2 3 ⋄ 4 5 6][;2]", vec![2], vec![2., 5.]),
        ("[1 2 3 ⋄ 4 5 6][2;]", vec![3], vec![4., 5., 6.]),
        ("(10 20 30)[3 1]", vec![2], vec![30., 10.]),
        ("+/[1][1 2 3 ⋄ 4 5 6]", vec![3], vec![5., 7., 9.]),
        ("s←+/ ⋄ s[1][1 2 3 ⋄ 4 5 6]", vec![3], vec![5., 7., 9.]),
        ("1 1⍉[1 2 3 ⋄ 4 5 6]", vec![2], vec![1., 5.]),
        ("2 ¯1 1/10 20 30", vec![4], vec![10., 10., 0., 30.]),
        ("2 ¯1 1\\10 20", vec![4], vec![10., 10., 0., 20.]),
    ] { check(code, Array::from_parts(shape, values.into_iter().map(number).collect(), number(0.)).unwrap()); }
    check("1 0 1\\'ab'", chars("a b"));
    for (code, kind) in
        [("(1 2)[0]", Index), ("(1 2)[¯1]", Index), ("(1 2)[3]", Index), ("(1 2)[1.5]", Domain), ("[1 2 ⋄ 3 4][1]", Rank), ("+/[0][1 2 ⋄ 3 4]", Domain)]
    { assert_eq!(run(code).unwrap_err().kind, kind, "{code}"); }
    for shape in [vec![0, 3], vec![3, 0], vec![2, 3], vec![2, 2, 3]] {
        let shape = shape.iter().map(usize::to_string).collect::<Vec<_>>().join(" ");
        let source = format!("{shape}⍴⍳12");
        for op in ["⌽⌽", "⊖⊖", "⍉⍉", "↑↓"] { assert_eq!(run(&format!("{op}{source}")).unwrap(), run(&source).unwrap()); }
    }
    check("[({⍵=0:1 ⋄ ⍵+2}0 ⋄ 3) ⋄ (4 ⋄ 5)][;1]", vector(&[1., 4.]));
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
    for (code, kind) in [("x←{}0", Value), ("1+{}0", Value), ("{6::7 ⋄ 1÷⍵}0", Domain), ("{1:1:2}0", Syntax), ("10{g←{⍺+⍵} ⋄ g ⍵}3", Value)] {
        assert_eq!(s.eval(code).error.unwrap().kind, kind, "{code}");
    }
}

fn exact(n: i64, d: i64) -> Array { Array::scalar(num_rational::BigRational::new(n.into(), d.into())).unwrap() }

#[test]
fn scalar_math() {
    for (code, values) in [
        ("|3 ¯3 3J4", vec![3., 3., 5.]),
        ("2 10 ¯2.5|7 ¯13 8", vec![1., 7., -2.]),
        ("0 3|¯2 6", vec![-2., 0.]),
        ("⌊1.000000000000001 ¯1.000000000000001", vec![1., -1.]),
        ("⌈¯2.3 0.1 3", vec![-2., 1., 3.]),
        ("2 3⌊3 2", vec![2., 2.]),
        ("2 3⌈3 2", vec![3., 3.]),
        ("2 ¯2 0*3 3 0", vec![8., -8., 1.]),
        ("9 11○3J4", vec![3., 4.]),
    ] { check(code, vector(&values)); }
    for (code, value) in [("*1", std::f64::consts::E), ("2⍟32", 5.), ("○1", std::f64::consts::PI), ("¯1○1", std::f64::consts::FRAC_PI_2)] {
        let a = run(code).unwrap().unwrap();
        assert!((a.as_number().unwrap().as_float().unwrap() - value).abs() < 1e-14, "{code}");
    }
    for code in ["1E¯13>|(¯4*0.5)-0J2", "(⌊3.3J2.5)=3J2", "(⌈3.3J2.5)=3J3", "0=0.1|0.3", "0=3|6.000000000000001", "0=1+*○0j1"] {
        check(code, scalar(1.));
    }
    for (code, n, d) in [("2x*¯3x", 1, 8), ("2r3|7r3", 1, 3), ("⌊¯4r3", -2, 1), ("⌈¯4r3", -1, 1)] { check(code, exact(n, d)); }
    for code in ["⍟0", "0*¯1", "13○1", "0J1⌊2"] { assert_eq!(run(code).unwrap_err().kind, Domain, "{code}"); }
}

#[test]
fn search_depth_and_random() {
    for (code, expected) in [
        ("x←1 ⋄ y←1+8E¯15 ⋄ z←1+16E¯15 ⋄ x y⍳z", scalar(2.)),
        ("≠1 (1+8E¯15) (1+16E¯15)", vector(&[1., 0., 1.])),
        ("1 2~1+8E¯15", vector(&[2.])),
        ("1x 2x~1000000000000001r1000000000000000", Array::new(vec![2], vec![exact(1, 1).at(0), exact(2, 1).at(0)]).unwrap()),
        ("(3 0⍴0)⍳2 0⍴0", vector(&[1., 1.])),
        ("≠3 0⍴0", vector(&[1., 0., 0.])),
        ("⍸0 1 0 2", vector(&[2., 4., 4.])),
        ("1 2⍸1-1E¯15", scalar(0.)),
        ("≡0⍴(1 2)3", scalar(2.)),
        ("1≢,1", scalar(1.)),
        ("1x≡1", scalar(1.)),
        ("(0⍴1x)≡⍬", scalar(1.)),
        ("⍳,3", vector(&[1., 2., 3.])),
        ("1 1∪2 2", vector(&[1., 1., 2., 2.])),
        ("1 2∊1+8E¯15", vector(&[1., 0.])),
        ("(,1)⍷1", scalar(0.)),
        ("(2 0⍴0)⍷1 3⍴1", Array::new(vec![1, 3], vec![number(0.); 3]).unwrap()),
        ("⍳⍬", Array::new(vec![], vec![Nested(vector(&[]))]).unwrap()),
        ("⍸0", Array::empty(vec![0], Nested(vector(&[]))).unwrap()),
    ] { check(code, expected); }
    for code in ["⌊0⍴1x", "⌈0⍴1x", "|0⍴1x", "!0⍴1x", "2x*0⍴1x"] {
        assert_eq!(run(code).unwrap().unwrap().prototype(), exact(0, 1).prototype(), "{code}");
    }
    for code in ["?100⍴9", "13?52"] {
        let a = run(code).unwrap().unwrap();
        let values: Vec<_> = a
            .elements()
            .map(|e| match e { Number(n) => n.as_float().unwrap() as usize, _ => panic!("numeric result") })
            .collect();
        let (len, max) = if code.starts_with('?') { (100, 9) } else { (13, 52) };
        assert_eq!(a.shape(), &[len]);
        assert!(values.iter().all(|&n| n > 0 && n <= max));
        if len == 13 { assert_eq!(values.iter().collect::<std::collections::HashSet<_>>().len(), len); }
    }
    let rolls = run("?100⍴0").unwrap().unwrap();
    assert!(rolls.elements().all(|e| matches!(e, Number(n) if n.as_float().is_some_and(|v| v > 0. && v < 1.))));
    for (code, error) in [("3?2", Domain), ("?¯1", Domain), ("⍸¯1", Domain), ("2⍳2", Rank), ("(2 3⍴0)⍳1 2", Length), ("(2 2⍴1)~1", Rank), ("2 1⍸1", Domain)]
    { assert_eq!(run(code).unwrap_err().kind, error, "{code}"); }
}

#[test]
fn each_commute_and_reduction() {
    for (code, expected) in [
        ("e←¨ ⋄ sum←+/ ⋄ sum e (1 2)(3 4 5)", vector(&[3., 12.])),
        ("+/¨¨(1 2)(3 4)", Array::new(vec![2], vec![Nested(vector(&[1., 2.])), Nested(vector(&[3., 4.]))]).unwrap()),
        ("2⍨3", scalar(2.)),
        ("2-⍨5", scalar(3.)),
        ("3+/1 2", vector(&[])),
        ("2+⌿2 3⍴⍳6", Array::new(vec![1, 3], vec![number(5.), number(7.), number(9.)]).unwrap()),
        ("-/⍬", scalar(0.)),
        ("÷/⍬", scalar(1.)),
        ("⌊/⍬", scalar(f64::MAX)),
        ("⌈/⍬", scalar(-f64::MAX)),
    ] { check(code, expected); }
    let mut session = Session::new();
    let result = session.eval("r←{⎕←7 ⋄ ⍵}¨⍬");
    assert!(result.error.is_none());
    assert_eq!(result.output, ["7"]);
    assert_eq!(result.value.unwrap(), vector(&[]));
    assert!(session.eval("{⍵=2:{}⍵ ⋄ ⍵}¨1 2 3").value.is_none());
    for (code, kind) in [("÷¨⍬", Domain), ("2¨3", Domain), ("0+/5", Rank), ("4+/1 2", Length), ("3+/⍬", Length), ("2+\\1 2", Syntax)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
}

#[test]
fn composition_rank_and_dyadic_operators() {
    for (code, expected) in [
        ("c←∘ ⋄ sum←+/c⍳ ⋄ sum¨2 4 6", vector(&[3., 10., 21.])),
        ("'abc'⍴⍛⍴'z'", Array::new(vec![3], vec![Character('z'); 3]).unwrap()),
        ("⍳⍤0⊢1 3 2", Array::new(vec![3, 3], [1., 0., 0., 1., 2., 3., 1., 2., 0.].map(number).to_vec()).unwrap()),
        ("({⍳3}⍤1)0 2⍴0", Array::empty(vec![0, 3], number(0.)).unwrap()),
        ("op←{⍺⍺+⍵⍵×⍵} ⋄ (2 op 3)4", scalar(14.)),
        ("op←{⍺⍺ ⍵⍵ ⍵} ⋄ (+/op⍳)4", scalar(10.)),
        ("f←{k←3 ⋄ g←{k+⍵} ⋄ op←{⍺⍺ ⍵⍵ ⍵} ⋄ (+op g)⍵} ⋄ f 4", scalar(7.)),
    ] { check(code, expected); }
    let mut session = Session::new();
    let r = session.eval("{⎕←⍵ ⋄ ⍳3}⍤0⊢⍬");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["0", "⍬"]);
    let r = session.eval("f←{⎕←⍵ ⋄ ⍵} ⋄ 2 +⍥f 3");
    assert_eq!(r.output, ["3", "2", "5"]);
    for (code, kind) in [("(⍳1)(+⍤0)⍳3", Length), ("1 2 (+⍤0)2 3⍴0", Rank), ("+⍤⍬⊢3", Length), ("+⍤0.5⊢3", Domain)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
}

#[test]
fn key_and_power() {
    check("1 1 2{+/⍵}⌸10 20 30", vector(&[30., 30.]));
    check("{⍳3}⌸⍬", Array::empty(vec![0, 3], number(0.)).unwrap());
    check("1(+⍣{⍺>4})0", scalar(5.));
    check("(2∘×⍣0)3", scalar(3.));
    check("{≢⍵}⌸1 (1+8E¯15)(1+16E¯15)", vector(&[2., 1.]));
    let mut s = Session::new();
    let result = s.eval("r←⍬ {⎕←⍴⍵ ⋄ ⍳3}⌸0 2⍴0");
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.output, ["0 2"]);
    assert_eq!(s.eval("({⎕←7 ⋄ ⍵}⍣0)3").output, ["3"]);
    assert!(s.eval("({}⍣1)3").value.is_none());
    for (code, kind) in [("{⍺ ⍵}⌸7", Rank), ("1 2{⍵}⌸3 4 5", Length), ("(+⍣0.5)1", Domain), ("(+⍣(,1))2", Rank)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
}

#[test]
fn products() {
    for (code, expected) in [
        ("(2 3⍴⍳6)+.×3 2⍴⍳6", Array::new(vec![2, 2], [22., 28., 49., 64.].map(number).to_vec()).unwrap()),
        ("(2 0⍴0)+.×0 3⍴0", Array::new(vec![2, 3], vec![number(0.); 6]).unwrap()),
        ("(,2)+.×1 2 3", scalar(12.)),
        ("jot←∘ ⋄ dot←. ⋄ times←× ⋄ 2 jot dot times 3", scalar(6.)),
        ("⍬∘.+7 8", Array::empty(vec![0, 2], number(0.)).unwrap()),
    ] { check(code, expected); }
    let mut s = Session::new();
    let r = s.eval("r←⍬∘.{⎕←⍺ ⍵ ⋄ ⍺+⍵}7 8");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["0 7"]);
    let r = s.eval("r←(0 2⍴0)+.{⎕←⍺ ⍵ ⋄ ⍺×⍵}2 3⍴⍳6");
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output, ["0 1", "0 2", "0 3", "0 4", "0 5", "0 6"]);
    assert_eq!(run("1 2+.×1 2 3").unwrap_err().kind, Length);
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
    check("×/0/1J2", scalar(1.0));
    check("⍳3J0", vector(&[1.0, 2.0, 3.0]));
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
    check("1x÷2", scalar(0.5));
    check("(1÷3)+(1÷6)", scalar(0.5));
    check("1r4+0.5", scalar(0.75));
    let huge = format!("1{}", "0".repeat(400));
    assert_eq!(run(&format!("{huge}x÷{huge}x")).unwrap().unwrap(), exact(1, 1));
    assert_eq!(run(&format!("{huge}x+0")).unwrap_err().kind, Domain);
    assert_eq!(run(&format!("{huge}1r{huge}0+0.5")).unwrap().unwrap(), scalar(1.5));
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
    assert_eq!(a.at(0), exact(9007199254740993, 1).at(0));
    assert_eq!(a.at(1), number(0.5));
    assert_eq!(a.at(2), exact(1, 3).at(0));
    let empty = run("0x/1r3").unwrap().unwrap();
    assert_eq!(empty.shape(), &[0]);
    assert_eq!(empty.prototype(), exact(0, 1).prototype());
    check("+/0/1r3", exact(0, 1));
    check("×/0/1r3", exact(1, 1));
    check("1x+0/1r3", empty);
    assert_eq!(run("1+0/1r3").unwrap().unwrap().prototype(), &number(0.0));
    check("⍳3x", vector(&[1.0, 2.0, 3.0]));
    assert_eq!(run("2x/1r3").unwrap().unwrap().elements().collect::<Vec<_>>(), vec![exact(1, 3).at(0); 2]);
    for (code, kind) in [("⍳3r2", Domain), ("⍳¯1x", Domain), ("⍳1000001x", Limit), ("⍳999999999999999999999x", Limit), ("1r2/3", Domain)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
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
            assert_eq!(run(&code).unwrap().unwrap(), Array::scalar(u8::from(expected) as f64).unwrap(), "{code}");
        }
    }
    check("0.3=0.3 (0.1+0.2) 0.4", vector(&[1.0, 1.0, 0.0]));
    check("1 2≤2 1", vector(&[1.0, 0.0]));
    check("(1=1+8E¯15)((1+8E¯15)=1+16E¯15)(1=1+16E¯15)", vector(&[1.0, 1.0, 0.0]));
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
        ("⌹1", Unsupported),
        ("⍳¯1", Domain),
        ("(+ -)", Syntax),
    ] { assert_eq!(run(code).unwrap_err().kind, kind, "{code}"); }
    // The right argument fails before the parenthesized left argument is evaluated.
    let e = run("(1÷0)+(2×1e308)").unwrap_err();
    assert_eq!(&e.span.source.text[e.span.range], "×");
    let e = run("¯2+1÷0").unwrap_err();
    assert_eq!(e.span.range, 5..7); // UTF-8 bytes, not glyph indices.
    assert_eq!(e.to_string(), "DOMAIN ERROR: division by zero\n --> test:1:5\n¯2+1÷0\n    ^");
    let e = run("(2\n 1÷0)").unwrap_err();
    assert!(e.to_string().contains("test:2:3\n 1÷0)\n  ^"));
    check("2+2", scalar(4.0));
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
    check(&format!("{}1", "1+".repeat(10_000)), scalar(10_001.0));
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
    for (code, expected) in
        [("+/v", 55.0), ("f←+ ⋄ 2 f 3", 5.0), ("sum←+/ ⋄ sum 1 2 3", 6.0), ("f/1 2 3", 6.0), ("-/1 2 3", 2.0), ("+/⍳0", 0.0), ("×/⍳0", 1.0), ("+/7", 7.0)]
    { check_in(&mut s, code, scalar(expected)); }
    for (code, expected) in
        [("1 2 3+10", vec![11.0, 12.0, 13.0]), ("10-1 2 3", vec![9.0, 8.0, 7.0]), ("1 2+3 4", vec![4.0, 6.0]), ("⍴v", vec![10.0]), (",7", vec![7.0])]
    { check_in(&mut s, code, vector(&expected)); }
    check_in(&mut s, "a←v ⋄ v←0 ⋄ +/a", scalar(55.0));
    check_in(&mut s, "≢,7", scalar(1.0));
    assert_eq!(s.eval("⍴7").value.unwrap().shape(), &[0]);
    assert_eq!(s.eval("2+⍳0").value.unwrap().shape(), &[0]);
    for (code, kind) in [("⍳1.5", Domain), ("⍳1000001", Limit), ("missing", Value), ("{⍺-⍵}/⍳0", Domain)] {
        assert_eq!(s.eval(code).error.unwrap().kind, kind);
    }
}

#[test]
fn result_output_and_nonexecuting_parse() {
    let mut s = Session::new();
    check_in(&mut s, "x←2", scalar(2.0));
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
    check_in(&mut s, "x", scalar(2.0));
    assert!(s.eval("").value.is_none());
    assert!(s.eval("2+2").error.is_none());
    let r = s.eval("x←5 ⋄ 1÷0");
    assert!(r.error.is_some());
    check_in(&mut s, "x", scalar(5.0));
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
    ] { check_in(&mut s, code, scalar(expected)); }
    for code in ["1 0 1/2 4 6", "rep←/ ⋄ 1 0 1 rep 2 4 6"] {
        let r = s.eval(code);
        assert!(r.error.is_none(), "{code}: {:?}", r.error);
        assert_eq!(r.value.unwrap(), vector(&[2.0, 6.0]));
    }
    assert_eq!(s.eval("f←{⎕←1 ⋄ ⍵} ⋄ h←{⎕←2 ⋄ ⍵} ⋄ t←f+h ⋄ t 3").output, ["2", "1", "6"]);
    let r = s.eval("bad←{local←99 ⋄ 1÷0} ⋄ bad 0");
    assert_eq!(r.error.unwrap().kind, Domain);
    assert_eq!(s.eval("local").error.unwrap().kind, Value);
    check_in(&mut s, "x", scalar(10.0));
    assert_eq!(s.eval("loop←{1+∇⍵} ⋄ loop 0").error.unwrap().kind, Limit);
    check_in(&mut s, "2+2", scalar(4.0));
    assert_eq!(s.eval("{⍵←1 ⋄ ⍵}2").error.unwrap().kind, Syntax);
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
    // Dyalog 20 primitive-functions-by-category: singleton scalar extension, not NumPy broadcasting.
    for (code, values) in [
        ("(,2)+3 4", vec![5.0, 6.0]),
        ("3 4+,2", vec![5.0, 6.0]),
        ("2+,3", vec![5.0]),
        ("(,2)/⍳0", vec![]),
        ("(⍳0)/,3", vec![]),
        ("(,2)+⍳0", vec![]),
        ("1000001/⍳0", vec![]),
    ] { check(code, vector(&values)); }
    for (code, kind) in [("0.5/⍳0", Domain), ("2 3/1 2 3", Length), ("1000001/1", Limit), ("99999999999999999999x/1", Limit)] {
        assert_eq!(run(code).unwrap_err().kind, kind, "{code}");
    }
}

#[test]
fn binder_limits_and_single_execution() {
    let mut s = Session::new();
    assert_eq!(s.eval("x←1 ⋄ (⎕←x)+(⎕←(x←2))").output, ["2", "2", "4"]);
    assert_eq!(s.eval("apply←{⍺⍺ ⍵} ⋄ f←{⎕←⍵ ⋄ ⍵} ⋄ (f apply)/1 2 3").output, ["3", "3", "3"]);
    assert_eq!(s.eval("offset←{+/⍺⍺+⍵} ⋄ (1 2 offset)3").value.unwrap(), scalar(9.0));
    assert_eq!(s.eval(&format!("{}7", "a←".repeat(10_000))).value.unwrap(), scalar(7.0));
    assert_eq!(s.eval(&format!("+{}1", "/".repeat(10_000))).error.unwrap().kind, Limit);
    assert_eq!(s.eval(&format!("f←+ ⋄ {} f 0", "f←f+f ⋄ ".repeat(127))).error.unwrap().kind, Limit);
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
    for (code, kind) in
        [("{0::1÷0 ⋄ 1÷⍵}0", Domain), ("{0::fresh ⋄ fresh←1 ⋄ 1÷⍵}0", Value), ("{2:1 ⋄ 0}0", Domain), ("{x←2 ⋄ {x+⍵}}0", Syntax), ("{1:+ ⋄ 0}0", Syntax)]
    { assert_eq!(s.eval(code).error.unwrap().kind, kind, "{code}"); }
    assert!(matches!(parse(Source::new("guard", "f←{0::⎕←1}")), ParseStatus::Complete(_)));
    assert_eq!(s.eval("2+2").value.unwrap(), scalar(4.0));
}
