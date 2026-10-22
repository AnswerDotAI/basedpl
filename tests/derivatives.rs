//! Checks `∂` against numerical derivatives at pseudo-random points. Each column of `f∂ x` must match the derivative that its case's `Oracle` takes. Complex step works only for an analytic function whose computation keeps the imaginary part of 1E¯200 that the oracle adds to `x`. Functions that aren't analytic, such as `|` and `⌈`, use a central difference. Each column must also match a forward pass along its direction. A case with one number also checks reverse mode, through an extra number that `f` ignores. A case with more numbers checks that u·(Jv) from a forward pass equals (uᵀJ)·v from a transpose.
use basedpl::{Session, Value};

/// How a case's numerical derivative is taken.
#[derive(Clone, Copy)]
enum Oracle { Complex, Central }

/// A function, as BPL source, with the range its argument's items are drawn from and the shape of that argument.
struct Case { f: &'static str, low: f64, high: f64, shape: &'static [usize], oracle: Oracle }

const fn case(f: &'static str, low: f64, high: f64, shape: &'static [usize], oracle: Oracle) -> Case { Case { f, low, high, shape, oracle } }

const CASES: &[Case] = &[
    case("[1 2 3]↣⌻", -2.0, 2.0, &[], Oracle::Complex),
    case("[0.5 ¯1 0 2]↣⌻", -2.0, 2.0, &[], Oracle::Complex),
    case("{⍵×⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{-⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{÷⍵}", 0.5, 2.0, &[], Oracle::Complex),
    case("{*⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{⍟⍵}", 0.5, 3.0, &[], Oracle::Complex),
    case("{√⍵}", 0.5, 3.0, &[], Oracle::Complex),
    case("{|⍵}", 0.5, 3.0, &[], Oracle::Central),
    case("{|⍵}", -3.0, -0.5, &[], Oracle::Central),
    case("{⊼⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{⊽⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{π⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{≤⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{⍵*3}", 0.5, 2.0, &[], Oracle::Complex),
    case("{3*⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{⍵*⍵}", 0.5, 2.0, &[], Oracle::Complex),
    case("{2⍟⍵}", 0.5, 3.0, &[], Oracle::Complex),
    case("{⍵⍟5}", 1.5, 3.0, &[], Oracle::Complex),
    case("{3√⍵}", 0.5, 3.0, &[], Oracle::Complex),
    case("{⍵√8}", 1.5, 3.0, &[], Oracle::Complex),
    case("{2π⍵}", 0.5, 3.0, &[], Oracle::Complex),
    case("{⍵π2}", 0.5, 3.0, &[], Oracle::Complex),
    case("{5÷⍵}", 0.5, 3.0, &[], Oracle::Complex),
    case("{⍵⌈0.3}", -2.0, 2.0, &[], Oracle::Central),
    case("{0.3⌊⍵}", -2.0, 2.0, &[], Oracle::Central),
    case("{0.7|⍵}", 0.1, 3.0, &[], Oracle::Central),
    case("{⍵|2.5}", 0.4, 3.0, &[], Oracle::Central),
    case("{0○⍵}", -0.9, 0.9, &[], Oracle::Complex),
    case("{1○⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{2○⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{3○⍵}", -1.0, 1.0, &[], Oracle::Complex),
    case("{4○⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{5○⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{6○⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{7○⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{1○⁻¹⍵}", -0.9, 0.9, &[], Oracle::Complex),
    case("{2○⁻¹⍵}", -0.9, 0.9, &[], Oracle::Complex),
    case("{3○⁻¹⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{4○⁻¹⍵}", 1.5, 3.0, &[], Oracle::Complex),
    case("{5○⁻¹⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{6○⁻¹⍵}", 1.5, 3.0, &[], Oracle::Complex),
    case("{7○⁻¹⍵}", -0.9, 0.9, &[], Oracle::Complex),
    // Complex step can't check a power at a negative base: BPL computes it through logarithms, and arg z = π-h÷|x| rounds to π.
    case("{(1○⍵)×⍵*2}", 0.5, 2.0, &[], Oracle::Complex),
    case("{⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{1○0.1 0.5 0.9+⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{(⍳4)×⍵}", -2.0, 2.0, &[], Oracle::Complex),
    case("{1○⍵×2}", -2.0, 2.0, &[4], Oracle::Complex),
    case("{⌽⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{2↑⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{⍵,⍵×2}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{⍉2 2⍴*⍵}", -2.0, 2.0, &[4], Oracle::Complex),
    case("{1 0 2#⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{(⊂2 0)⌷⍵×3}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{<⍵×⍵}", 0.5, 2.0, &[4], Oracle::Complex),
    case("{⍵+⍉⍵×⍵}", -2.0, 2.0, &[2, 2], Oracle::Complex),
    case("{⍵⌈⌽⍵}", -2.0, 2.0, &[3], Oracle::Central),
    case("{+/⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{+\\⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{×\\⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{×/⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{⌈\\⍵}", -2.0, 2.0, &[4], Oracle::Central),
    case("{⌊/⍵}", -2.0, 2.0, &[4], Oracle::Central),
    case("{[1 2 3⋄4 5 6]+.×⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{2 3⍴⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{(⊂0 0 2)⌷⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{5↑⍵×⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{,{(+/⍵)×⍵}⍤1⊢⍵}", -2.0, 2.0, &[2, 2], Oracle::Complex),
    case("{{⍵×1○⍵}¨⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{⍵⌻0.5 1.5}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{⌹⍵+[3 0⋄0 3]}", -1.0, 1.0, &[2, 2], Oracle::Complex),
    case("{1 2⌹⍵+[3 0⋄0 3]}", -1.0, 1.0, &[2, 2], Oracle::Complex),
    case("{⍵⌹[2 1⋄1 3]}", -2.0, 2.0, &[2], Oracle::Complex),
    case("{⍵⌹[1 0⋄0 1⋄1 1]}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{-.×⍵+[3 0⋄0 3]}", -1.0, 1.0, &[2, 2], Oracle::Complex),
    case("{10⊥⍵}", -2.0, 2.0, &[3], Oracle::Complex),
    case("{⍵⊥1 2 3}", 0.5, 2.0, &[], Oracle::Complex),
    case("{⍵⊥1 2 3}", 0.5, 2.0, &[3], Oracle::Complex),
    case("{+/,⊃[⍵;2×⍵]}", -2.0, 2.0, &[3], Oracle::Complex),
];

/// A small xorshift generator, so that each run draws the same points.
struct Points(u64);

impl Points {
    fn next(&mut self, low: f64, high: f64) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        low + (high - low) * (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The floats that `code`, one expression, gives, ravelled.
#[track_caller]
fn floats(session: &Session, code: &str) -> Vec<f64> {
    let result = session.eval(&format!("0+,({code})"));
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    result.value.as_ref().and_then(Value::as_floats).unwrap_or_else(|| panic!("{code} gave no floats")).to_vec()
}

/// The BPL literal for `items` in `shape`, with `¯` for negative numbers. An empty shape gives the number itself, an atom.
fn literal(items: &[f64], shape: &[usize]) -> String {
    let items: Vec<String> = items.iter().map(|x| format!("{x:e}").replace('-', "¯").replace('e', "E")).collect();
    if shape.is_empty() { return items[0].clone(); }
    let shape: Vec<String> = shape.iter().map(usize::to_string).collect();
    format!("([{}]⍴[{}])", shape.join(" "), items.join(" "))
}

#[test]
fn derivatives_match_numerical_ones() {
    let mut points = Points(0x9E37_79B9_7F4A_7C15);
    let mut failures = Vec::new();
    for c in CASES {
        for _ in 0..3 {
            let n: usize = c.shape.iter().product();
            let x: Vec<f64> = (0..n).map(|_| points.next(c.low, c.high)).collect();
            let s = Session::new();
            let result = s.eval(&format!("f←{} ⋄ x←{}", c.f, literal(&x, c.shape)));
            assert!(result.error.is_none(), "{}: {:?}", c.f, result.error);
            let jacobian = floats(&s, "f∂x");
            let m = jacobian.len() / n;
            for j in 0..n {
                let direction = if c.shape.is_empty() { "e←1".to_owned() } else { format!("e←(⍴x)⍴{j}=⍳{n}") };
                let ok = s.eval(&direction);
                assert!(ok.error.is_none(), "{direction}: {:?}", ok.error);
                let column = match c.oracle {
                    Oracle::Complex => floats(&s, "{(⊢/∨⍵)÷1E¯200}f x+0j1E¯200×e"),
                    Oracle::Central => floats(&s, "((f x+1E¯6×e)-f x-1E¯6×e)÷2E¯6"),
                };
                let tolerance = match c.oracle { Oracle::Complex => 1e-9, Oracle::Central => 1e-5 };
                if column.len() != m { failures.push(format!("{} at {x:?}, direction {j}: {m} items against {}", c.f, column.len())); continue; }
                for i in 0..m {
                    let (got, want) = (jacobian[i * n + j], column[i]);
                    if (got - want).abs() > tolerance * (1.0 + want.abs()) { failures.push(format!("{} at {x:?}, item {i}, direction {j}: {got} against {want}", c.f)); }
                }
                // A forward pass along `e` gives the same column, whichever mode built the Jacobian.
                let forward = floats(&s, "{f x+⍵×e}∂0");
                if forward.len() != m { failures.push(format!("{} at {x:?}, direction {j}: a forward pass gave {} items against {m}", c.f, forward.len())); continue; }
                for (i, &want) in forward.iter().enumerate() {
                    let got = jacobian[i * n + j];
                    if (got - want).abs() > 1e-9 * (1.0 + want.abs()) { failures.push(format!("{} at {x:?}, item {i}, direction {j}: {got} against a forward pass's {want}", c.f)); }
                }
            }
            let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs()));
            if n == 1 {
                // A second number that `f` ignores sends `∂` down reverse mode, which must give the same derivatives.
                let reverse = floats(&s, "⊣/{f ⍵₀}∂ x,0");
                for (i, (&got, &want)) in reverse.iter().zip(&jacobian).enumerate() {
                    if !close(got, want) { failures.push(format!("{} at {x:?}, item {i}: reverse mode's {got} against {want}", c.f)); }
                }
            } else {
                // The dot-product test: u·(Jv) from a forward pass must equal (uᵀJ)·v from a transpose.
                let r = floats(&s, "f x").len();
                let u: Vec<f64> = (0..r).map(|_| points.next(-1.0, 1.0)).collect();
                let v: Vec<f64> = (0..n).map(|_| points.next(-1.0, 1.0)).collect();
                let ok = s.eval(&format!("u←(⍴f x)⍴{} ⋄ v←(⍴x)⍴{}", literal(&u, &[r]), literal(&v, &[n])));
                assert!(ok.error.is_none(), "{}: {:?}", c.f, ok.error);
                let jv = floats(&s, "{f x+⍵×v}∂0");
                let ujt = floats(&s, "{+/,u×(f ⍵)}∂ x");
                let (a, b): (f64, f64) = (u.iter().zip(&jv).map(|(p, q)| p * q).sum(), v.iter().zip(&ujt).map(|(p, q)| p * q).sum());
                if !close(a, b) { failures.push(format!("{} at {x:?}: u·(Jv) is {a}, (uᵀJ)·v is {b}", c.f)); }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
