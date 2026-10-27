//! The JIT on what BPL's reference cases can't reach: vectors of every type, kernels that mix widths, and folds at every length around those that a block stage folds a block at a time. The reference harness runs at sizes where blocks hold a few items, so a compiled part there runs only its loop over leftover items, one at a time. Its cases check compiled parts against the interpreter. `compiled_stages_match_interpreted` checks stages that compile through the real lowering against the same stages with no compiled parts, at lengths that reach each loop.

use basedpl::jit::{Cmp, Fold, Input, Kernel, Op, Read, Step, Ty, Value, Within};
use rustymath::reduce::{minimum, Product, Reduction, Sum};

/// f32 items widened to f64 and halved, then `⌊0`, which is stored. Each result is rustymath's value, a zero of either sign for a zero, or NaN beside NaN, where rustymath gives the other argument.
#[test]
fn mixed_widths_match_rustymath() {
    let kernel = Kernel {
        inputs: vec![Input::Array(Ty::F32, Read::Forward), Input::Scalar(Ty::F64), Input::Scalar(Ty::F64)],
        steps: vec![
            Step { op: Op::Convert, ty: Ty::F64, x: Value::Input(0), y: None },
            Step { op: Op::Multiply, ty: Ty::F64, x: Value::Step(0), y: Some(Value::Input(1)) },
            Step { op: Op::Minimum, ty: Ty::F64, x: Value::Step(1), y: Some(Value::Input(2)) },
        ],
        stores: vec![2], folds: vec![],
    }.compiled().unwrap().0;
    for x in [vec![f32::NAN, -0.0, 3.0, -3.0, 5.0, 1.5, -7.25, 0.0, 2.0, -0.0, 9.0], vec![-0.0; 3]] {
        let mut out = vec![0.0_f64; x.len()];
        unsafe { kernel.run(x.len(), x.len(), &[x.as_ptr().cast()], &[0.5, 0.0].map(f64::to_bits), &[out.as_mut_ptr().cast()], &mut []) };
        let expected = x.iter().map(|&v| minimum(f64::from(v) * 0.5, 0.0));
        assert!(out.iter().zip(&x).zip(expected).all(|((&a, &v), b)| a == b || a.is_nan() && v.is_nan()), "{out:?}");
    }
}

/// Halves, stored, with their sum, greatest and least, and the product of `1 + h × 10⁻⁴` for each half `h`, in f32 and f64, at lengths around one block and for 12,588 items in blocks of 4,096 and of 1,000. Each block's sum and product lie within the rounding of two folds of its items in different orders, each with as many running results as rustymath's `slice` keeps: `n / 16 + 64` roundings, relative to the sum of the items' magnitudes or to the product. A sum of negative zeros is `¯0`. The greatest and least are exact. Beside NaN items they're NaN or the greatest or least of the other items, where the greatest and least items come first and NaNs follow them 32 items later.
#[test]
fn folds_match_rustymath() {
    macro_rules! check {
        ($f:ty, $ty:expr) => {{
            let kernel = Kernel {
                inputs: vec![Input::Array($ty, Read::Forward), Input::Scalar($ty), Input::Scalar($ty), Input::Scalar($ty)],
                steps: vec![
                    Step { op: Op::Multiply, ty: $ty, x: Value::Input(0), y: Some(Value::Input(1)) },
                    Step { op: Op::Multiply, ty: $ty, x: Value::Step(0), y: Some(Value::Input(2)) },
                    Step { op: Op::Add, ty: $ty, x: Value::Step(1), y: Some(Value::Input(3)) },
                ],
                stores: vec![0], folds: vec![(Fold::Sum, 0), (Fold::Product, 2), (Fold::Max, 0), (Fold::Min, 0)],
            }.compiled().unwrap().0;
            let c = |x: f64| x as $f;
            let varied: Vec<$f> = (0..12588u32).map(|i| (i.wrapping_mul(2654435761) % 1000) as $f / 7.0 - 70.0).collect();
            let nans: Vec<$f> = varied.iter().enumerate().map(|(i, &v)| match i { 0 => 1e3, 1 => -1e3, _ if i % 1500 == 32 || i % 1500 == 33 => <$f>::NAN, _ => v }).collect();
            let cases = [255, 256, 257, 300, 1000, 4095, 4096, 4127, 4128].map(|n| (n, n)).into_iter().chain([(12588, 4096), (12588, 1000)]);
            for x in [varied, vec![-0.0; 12588], nans] {
                for (n, block) in cases.clone() {
                    let halves: Vec<$f> = x[..n].iter().map(|&v| v * c(0.5)).collect();
                    let (mut out, mut totals) = (vec![0.0 as $f; n], vec![0; 4 * n.div_ceil(block)]);
                    unsafe { kernel.run(n, block, &[x.as_ptr().cast()], &[0.5, 1e-4, 1.0].map(f64::to_bits), &[out.as_mut_ptr().cast()], &mut totals) };
                    assert!(out.iter().zip(&halves).all(|(a, b)| a.to_bits() == b.to_bits()), "halves of {n} items");
                    for (k, h) in halves.chunks(block).enumerate() {
                        let got = |j: usize| f64::from_bits(totals[4 * k + j]);
                        let nan = h.iter().any(|v| v.is_nan());
                        let (most, least) = h.iter().filter(|v| !v.is_nan()).fold((f64::NEG_INFINITY, f64::INFINITY), |(a, b), &v| (a.max(v.into()), b.min(v.into())));
                        assert!((got(2) == most || nan && got(2).is_nan()) && (got(3) == least || nan && got(3).is_nan()), "extremes of block {k} of {n} items");
                        if nan { continue; }
                        let rounding = (h.len() / 16 + 64) as f64 * f64::from(<$f>::EPSILON);
                        let sum = <Sum as Reduction<$f>>::slice(h).0;
                        assert!((got(0) - f64::from(sum)).abs() <= rounding * h.iter().map(|&v| f64::from(v).abs()).sum::<f64>(), "sum of block {k} of {n} items");
                        assert!(sum.to_bits() != (-0.0 as $f).to_bits() || got(0).to_bits() == (-0.0f64).to_bits(), "sum of negative zeros of block {k} of {n} items");
                        let factors: Vec<$f> = h.iter().map(|&v| v * c(1e-4) + c(1.0)).collect();
                        let product = <Product as Reduction<$f>>::slice(&factors).0;
                        assert!((got(1) - f64::from(product)).abs() <= rounding * f64::from(product).abs(), "product of block {k} of {n} items");
                    }
                }
            }
        }};
    }
    check!(f32, Ty::F32);
    check!(f64, Ty::F64);
}

/// The lengths and block sizes the integer tests run at: around a 16-item group and a 64-item iteration, in one block and in several.
const LENGTHS: [(usize, usize); 11] = [(1, 1), (15, 15), (16, 16), (17, 17), (63, 63), (64, 64), (65, 65), (130, 130), (200, 64), (200, 7), (1000, 1000)];

/// Item `i` of a varied sequence of 64-bit patterns, whose first items are 0, all ones, and the top and bottom bits.
fn bits(i: usize) -> u64 {
    match i { 0 => 0, 1 => u64::MAX, 2 => 1 << 63, 3 => !(1 << 63), 4 => 1, _ => (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(i as u32 % 64) }
}

/// Integer arithmetic, comparisons, Boolean logic, a select and a conversion from Booleans at each integer width, beside a Boolean input that makes each group 16 items. Each stored result, and each block's sum, product and Boolean count, match Rust's wrapping results.
#[test]
fn integers_match_rust() {
    macro_rules! check {
        ($t:ty, $ty:expr, $abs:expr) => {{
            let s = |op, ty, x, y: Option<usize>| Step { op, ty, x: Value::Step(x), y: y.map(Value::Step) };
            let i = |op, ty, x, y: Option<usize>| Step { op, ty, x: Value::Input(x), y: y.map(Value::Input) };
            let kernel = Kernel {
                inputs: vec![Input::Array($ty, Read::Forward), Input::Array($ty, Read::Forward), Input::Array(Ty::Bool, Read::Forward)],
                steps: vec![
                    i(Op::Add, $ty, 0, Some(1)), i(Op::Subtract, $ty, 0, Some(1)), i(Op::Multiply, $ty, 0, Some(1)),
                    i(Op::Maximum, $ty, 0, Some(1)), i(Op::Minimum, $ty, 0, Some(1)), i(Op::Negate, $ty, 0, None), i(Op::Magnitude, $ty, 0, None),
                    i(Op::Compare(Cmp::Lt), Ty::Bool, 0, Some(1)), i(Op::Compare(Cmp::Ge), Ty::Bool, 0, Some(1)),
                    Step { op: Op::And, ty: Ty::Bool, x: Value::Step(7), y: Some(Value::Input(2)) },
                    Step { op: Op::Xor, ty: Ty::Bool, x: Value::Step(8), y: Some(Value::Input(2)) },
                    s(Op::Not, Ty::Bool, 10, None),
                    s(Op::Select(Value::Input(2)), $ty, 0, Some(1)),
                    s(Op::Convert, $ty, 11, None),
                ],
                stores: (0..14).collect(),
                folds: vec![(Fold::Sum, 0), (Fold::Max, 0), (Fold::Min, 0)],
            }.compiled().unwrap().0;
            let abs: fn($t) -> $t = $abs;
            for (n, block) in LENGTHS {
                let x: Vec<$t> = (0..n).map(|k| bits(k) as $t).collect();
                let y: Vec<$t> = (0..n).map(|k| bits(k + 7) as $t).collect();
                let m: Vec<u8> = (0..n).map(|k| (bits(k + 3) >> 17) as u8 & 1).collect();
                let mut outs: Vec<Vec<u64>> = (0..14).map(|_| vec![0; n]).collect();
                let places: Vec<*mut u8> = outs.iter_mut().map(|o| o.as_mut_ptr().cast()).collect();
                let mut totals = vec![0u64; 3 * n.div_ceil(block)];
                unsafe { kernel.run(n, block, &[x.as_ptr().cast(), y.as_ptr().cast(), m.as_ptr().cast()], &[], &places, &mut totals) };
                let got = |s: usize, k: usize| -> $t { unsafe { *outs[s].as_ptr().cast::<$t>().add(k) } };
                let flag = |s: usize, k: usize| -> bool { let v = unsafe { *outs[s].as_ptr().cast::<u8>().add(k) }; assert!(v <= 1, "a Boolean byte of {v}"); v == 1 };
                for k in 0..n {
                    let (a, b, mk) = (x[k], y[k], m[k] == 1);
                    let (lt, ge) = (a < b, a >= b);
                    let not = !(ge ^ mk);
                    let expected = [a.wrapping_add(b), a.wrapping_sub(b), a.wrapping_mul(b), a.max(b), a.min(b), a.wrapping_neg(), abs(a)];
                    for (s, &e) in expected.iter().enumerate() { assert_eq!(got(s, k), e, "step {s} of item {k} of {n}"); }
                    assert_eq!([flag(7, k), flag(8, k), flag(9, k), flag(10, k), flag(11, k)], [lt, ge, lt & mk, ge ^ mk, not], "Booleans of item {k} of {n}");
                    assert_eq!(got(12, k), if mk { a.wrapping_add(b) } else { a.wrapping_sub(b) }, "select of item {k} of {n}");
                    assert_eq!(got(13, k), not as $t, "conversion of item {k} of {n}");
                }
                for (j, from) in (0..n).step_by(block).enumerate() {
                    let r = from..n.min(from + block);
                    let t = &totals[3 * j..3 * j + 3];
                    let wide = |v: $t| v as i64 as u64;
                    assert_eq!(t[0], wide(r.clone().fold(0 as $t, |s, k| s.wrapping_add(x[k].wrapping_add(y[k])))), "sum of block {j} of {n}");
                    let sums = || r.clone().map(|k| x[k].wrapping_add(y[k]));
                    assert_eq!([t[1], t[2]], [wide(sums().max().unwrap()), wide(sums().min().unwrap())], "maximum and minimum of block {j} of {n}");
                }
            }
        }};
    }
    check!(u8, Ty::U8, |v| v);
    check!(i16, Ty::I16, |v| v.wrapping_abs());
    check!(i32, Ty::I32, |v| v.wrapping_abs());
    check!(i64, Ty::I64, |v| v.wrapping_abs());
}

/// An item to convert: a Boolean or an integer, or a float.
#[derive(Clone, Copy, Debug)]
enum Item { Int(i128), Float(f64) }

/// Every conversion between the seven types, beside a byte input that makes each group 16 items. Each result matches Rust's `as` after a float's floor, with 1 and 0 for Booleans. The floats include NaN, both infinities, both zeros, halves and values past every integer's range.
#[test]
fn conversions_match_rust() {
    let floats: Vec<f64> = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, 0.5, -0.5, 255.5, 256.0, -1.5, 32767.5, -32769.0, 2147483647.5, 2147483648.0, -2147483649.0, 9.3e18, -9.3e18, 1e30, 3.7, -3.7]
        .into_iter().chain((0..60).map(|k| (bits(k) as i64 as f64) / (1u64 << (k % 50)) as f64)).collect();
    let n = floats.len();
    macro_rules! source {
        (Bool) => { (0..n).map(|k| (bits(k) >> 9) as u8 & 1).collect::<Vec<u8>>() };
        (F32) => { floats.iter().map(|&f| f as f32).collect::<Vec<f32>>() };
        (F64) => { floats.clone() };
        ($t:ident) => { (0..n).map(|k| bits(k) as $t).collect::<Vec<$t>>() };
    }
    macro_rules! items {
        (Bool, $x:expr) => { $x.iter().map(|&v| Item::Int(v.into())).collect::<Vec<Item>>() };
        (F32, $x:expr) => { $x.iter().map(|&v| Item::Float(v.into())).collect::<Vec<Item>>() };
        (F64, $x:expr) => { $x.iter().map(|&v| Item::Float(v)).collect::<Vec<Item>>() };
        ($t:ident, $x:expr) => { $x.iter().map(|&v| Item::Int(v.into())).collect::<Vec<Item>>() };
    }
    macro_rules! convert {
        (f32, $v:expr) => { match $v { Item::Int(i) => i as f32, Item::Float(f) => f as f32 } };
        (f64, $v:expr) => { match $v { Item::Int(i) => i as f64, Item::Float(f) => f } };
        ($t:ty, $v:expr) => { match $v { Item::Int(i) => i as $t, Item::Float(f) => f.floor() as $t } };
    }
    macro_rules! pair {
        ($from:ident $from_src:ident, $to:ident $to_t:tt) => {{
            let x = source!($from_src);
            let items = items!($from_src, x);
            let pad = vec![0u8; n];
            let kernel = Kernel { inputs: vec![Input::Array(Ty::$from, Read::Forward), Input::Array(Ty::U8, Read::Forward)], steps: vec![Step { op: Op::Convert, ty: Ty::$to, x: Value::Input(0), y: None }], stores: vec![0], folds: vec![] }.compiled().unwrap().0;
            let mut out = vec![0u64; n];
            unsafe { kernel.run(n, n, &[x.as_ptr().cast(), pad.as_ptr().cast()], &[], &[out.as_mut_ptr().cast()], &mut []) };
            for (k, &item) in items.iter().enumerate() {
                let got: $to_t = unsafe { *out.as_ptr().cast::<$to_t>().add(k) };
                let expected: $to_t = convert!($to_t, item);
                assert!(got == expected || got != got && expected != expected, "{} to {}: item {k}, {item:?}, gave {got:?} for {expected:?}", stringify!($from), stringify!($to));
            }
        }};
    }
    macro_rules! from {
        ($from:ident $src:ident) => {{
            pair!($from $src, U8 u8); pair!($from $src, I16 i16); pair!($from $src, I32 i32); pair!($from $src, I64 i64); pair!($from $src, F32 f32); pair!($from $src, F64 f64);
        }};
    }
    from!(Bool Bool); from!(U8 u8); from!(I16 i16); from!(I32 i32); from!(I64 i64); from!(F32 F32); from!(F64 F64);
}

/// A run-by-run array read through a table of its runs' first items, forward, backward and at a stride within each run, plus a plain array, stored and summed. Calls start at a run's start, inside a run and one item before a run's end, and blocks end inside runs, so each loop starts and ends runs. The items are integers and halves, so each block's sum is exact in any order.
#[test]
fn runs_read_through_tables() {
    let (len, runs) = (24, 40);
    let source: Vec<f64> = (0..4000).map(|k| k as f64).collect();
    // Runs whose first items descend, each far enough inside the source that a backward or strided run stays in it.
    let starts: Vec<usize> = (0..runs).map(|r| 3000 - 70 * r).collect();
    for (within, step) in [(Within::Forward, 1isize), (Within::Backward, -1), (Within::Strided, 3)] {
        let kernel = Kernel {
            inputs: vec![Input::Array(Ty::F64, Read::Runs(within)), Input::Array(Ty::F64, Read::Forward)],
            steps: vec![Step { op: Op::Add, ty: Ty::F64, x: Value::Input(0), y: Some(Value::Input(1)) }],
            stores: vec![0], folds: vec![(Fold::Sum, 0)],
        }.compiled().unwrap().0;
        let item = |p: usize| (starts[p / len] as isize + (p % len) as isize * step) as usize;
        for (from, n, block) in [(0, runs * len, 1000), (5, 300, 64), (len - 1, 97, 7)] {
            let other: Vec<f64> = (0..n).map(|k| 0.5 * k as f64).collect();
            let (mut out, mut totals) = (vec![0.0_f64; n], vec![0_u64; n.div_ceil(block)]);
            let stride = if within == Within::Strided { vec![step as u64] } else { vec![] };
            let scalars: Vec<u64> = stride.into_iter().chain([len as u64, (len - from % len) as u64]).collect();
            unsafe { kernel.run(n, block, &[source.as_ptr().cast(), starts[from / len..].as_ptr().cast(), other.as_ptr().cast()], &scalars, &[out.as_mut_ptr().cast()], &mut totals) };
            let expected: Vec<f64> = (0..n).map(|k| source[item(from + k)] + other[k]).collect();
            assert_eq!(out, expected, "{within:?} from {from}");
            let sums: Vec<u64> = expected.chunks(block).map(|b| b.iter().sum::<f64>().to_bits()).collect();
            assert_eq!(totals, sums, "{within:?} from {from}");
        }
    }
}

/// Special floats: zeros of both signs, infinities, NaN, subnormals at f64 and at f32, and the largest f64, which is infinite at f32.
const SPECIALS: [f64; 10] = [0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 5e-324, -1e-310, 1e-40, -1e-45, f64::MAX];

/// The arrays of `n` items that `compiled_stages_match_interpreted` reads, by name, made under the kinds in force. Floats `x` and `y`, and their f32 copies `xf` and `yf`, hold whole numbers and `SPECIALS`. Every third item of `x` is `0.75`. Most items of `y` are the item of `x` plus a whole number from ¯2 to 2, and some are its negation, so comparisons give each outcome. `b` and `bf` hold whole numbers, whose sums are exact in any order. `p` and `q` hold Booleans. The rest hold ordinary integers in the range of the fixed width that the test gives each of them.
fn arrays(n: usize) -> Vec<(&'static str, basedpl::Value)> {
    use rustymath::buffer::FloatWidth::{F32, F64};
    let b: Vec<f64> = (0..n).map(|i| (i as u64 * 2654435761 % 125) as f64 - 62.0).collect();
    let x: Vec<f64> = (0..n).map(|i| if i % 61 == 0 { SPECIALS[i / 61 % 10] } else if i % 3 == 0 { 0.75 } else { b[i] }).collect();
    let y: Vec<f64> = (0..n).map(|i| if i % 67 == 0 { SPECIALS[i / 67 % 10] } else if i % 7 == 6 { -x[i] } else { x[i] + (i % 5) as f64 - 2.0 }).collect();
    let floats = |v: &[f64], width| basedpl::Value::floats([n], width, v.iter().copied()).unwrap();
    let ints = |of: fn(u64) -> i64, at: usize| basedpl::Value::integers([n], (0..n).map(|i| of(bits(i + at))).collect()).unwrap();
    let booleans = |of: fn(usize) -> bool| basedpl::Value::booleans([n], (0..n).map(of).collect()).unwrap();
    vec![
        ("x", floats(&x, F64)), ("y", floats(&y, F64)), ("b", floats(&b, F64)), ("xf", floats(&x, F32)), ("yf", floats(&y, F32)), ("bf", floats(&b, F32)),
        ("p", booleans(|i| i % 3 == 0)), ("q", booleans(|i| i % 5 < 2)),
        ("iu", ints(|v| v as u8 as i64, 0)), ("ju", ints(|v| v as u8 as i64, 9)), ("ih", ints(|v| v as i16 as i64, 0)), ("jh", ints(|v| v as i16 as i64, 9)),
        ("iw", ints(|v| v as i32 as i64, 0)), ("jw", ints(|v| v as i32 as i64, 9)), ("il", ints(|v| v as i64, 0)), ("jl", ints(|v| v as i64, 9)),
    ]
}

/// Each compiled operation, in a stage of two or more steps, gives what the same stage gives with no compiled parts (`settings::Compiles`), under dynamic and static kinds. The lengths reach each loop of a kernel: fusion defers calls on 2¹⁷ items, which a stage splits into blocks of a power of two items, at most 4,096 (`blocks::Blocks::block`), and 1, 7 or 37 items more make a last block of their own. Floats compare within the accuracy rule, as the reference harness compares them, and Booleans and integers exactly.
#[test]
fn compiled_stages_match_interpreted() {
    use basedpl::settings::{self, Compiles};
    const SETUP: &str = "u←iuₓ₁ ⋄ v←juₓ₁ ⋄ h←ihₓ₂ ⋄ k←jhₓ₂ ⋄ w←iwₓ₄ ⋄ z←jwₓ₄ ⋄ l←ilₓ₈ ⋄ m←jlₓ₈";
    let (mut dynamic, mut fixed): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    dynamic.extend(["+", "-", "×", "⌈", "⌊"].map(|g| format!("--x{g}y")));
    fixed.extend(["+", "-", "×", "÷", "⌈", "⌊"].map(|g| format!("--xf{g}yf")));
    for g in ["=", "≠", "<", "≤", ">", "≥"] {
        fixed.extend([format!("~x{g}y"), format!("~xf{g}yf"), format!("~yf{g}0.75"), format!("~0.75{g}yf")]);
    }
    dynamic.extend(["-|x", "~p∧q", "~p∨q", "--xf+x", "+/b-1", "⌈/b-1"].map(String::from));
    fixed.extend(["--x÷y", "(0.75≤)⍚[- ⊢]xf", "+/yf>0.75", "0ₓ₄+/xf<yf", "-|xf", "√|xf", "√|x", "--u+v", "--h×k", "--w-z", "--l⌈m", "~u<v", "~l≥m", "-|h", "--u+h", "+/h×k", "⌈/u+v", "+/u>v", "+/bf-1", "⌈/bf-1"].map(String::from));
    let options = || basedpl::EvalOptions { echo: false, ..Default::default() };
    for n in [0, 1, 7, 37].map(|more| (1 << 17) + more) {
        for (prefs, cases) in [("", &dynamic), (r#"•prefs ["kinds":"static"]"#, &fixed)] {
            settings::reset();
            // Static kinds make ordinary integers at the `int` width, so the arrays are made under the kinds that read them.
            basedpl::Session::new().eval_with(prefs, options());
            let data = arrays(n);
            for code in cases {
                // The value of `code` under `compiles`, computed in full, and the number of stage runs that used compiled code.
                let run = |compiles| {
                    settings::reset();
                    settings::set_compiles(compiles);
                    let session = basedpl::Session::new();
                    let eval = |code: &str| { let e = session.eval_with(code, options()); assert!(e.error.is_none(), "{code}: {:?}", e.error); e.value };
                    eval(prefs);
                    for (name, value) in &data { session.set(name, value.clone()).unwrap(); }
                    eval(SETUP);
                    let before = settings::compiled_runs();
                    let value = eval(code).unwrap_or_else(|| panic!("{code} gave no value"));
                    // Reading an item computes a pending result in full.
                    value.elements().next();
                    (value, settings::compiled_runs() - before)
                };
                let (compiled, runs) = run(Compiles::Always);
                assert!(runs > 0, "{code} on {n} items ran no compiled part");
                let interpreted = run(Compiles::Never).0;
                assert_eq!(basedpl::reference::difference(&compiled, &interpreted, None, 0.0, true), None, "{code} on {n} items");
            }
        }
    }
}
