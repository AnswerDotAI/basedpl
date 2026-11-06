//! The pervasive functions on stored values, through rustymath's formulas.
//!
//! `formula` is the one map from a primitive to its rustymath formula object. `resolve` resolves that object into a `Kernel` for a call's argument kinds, and the facts that planning reads without kinds, such as `folds` and `truth_fold`, come from the same object. `map` applies a kernel to whole arrays, `number` to single numbers, and `scatter` to writes at positions through rustymath's `operation::at`. All three share one kind search, `searched`. Under dynamic kinds, a kernel that overflows runs again at the next integer kind. At 64 bits a kernel wraps (`Numeric::wraps`). A float kernel that fails an item outside its real domain runs again on complex numbers. Under static kinds, a failure ends the call, and `number` then gives the result item by item or the error. Folds, scans and inner products are in the submodules `fold`, `scan` and `product`.
//!
//! `copied_halves` runs a call with 16-bit floats on 32-bit copies of them, because the CPU runs 16-bit floats only for testing. Such a call rounds a function of several steps, such as `|`, once, where a 16-bit computation would round each step.
use crate::{
    Number, Value, agreement::{Agree, Agreement}, array::{Items, Lanes, Promotion, Widening}, number::{Arithmetic, Math, Numeric},
    plan::{self, Kind, Pervasive}, primitive::{Comparison, Exact, FoldKind, Primitive, Real, Special}
};
use rustymath::{
    agreement::{Mapping, Missing, Read}, buffer::{Buffer, DType, FloatWidth, Floats, View, Width}, gather::Writes, operation::Kernel, reduce,
    search::Numbered, with_floats, with_ints
};
use std::borrow::Cow;

/// A use of the rustymath formula object that a primitive computes, through `formula`.
pub(crate) trait Visit { type Out; fn visit<K: rustymath::facts::Formula<N>, const N: usize>(self, object: K) -> Self::Out; }

/// `v` on the formula that `p` computes with `arity` arguments, or `None` where `p` has none. This is the one map from a primitive to its rustymath formula. `mathish` picks the Mathish forms, `integral` the floor quotient for `÷` on integers, and approximate forms compare within `t`. `○` and `○⁻¹` pick their formula by their code, which only `resolve` reads.
pub(crate) fn formula<V: Visit>(p: Primitive, arity: usize, mathish: bool, integral: bool, t: rustymath::tolerant::Tolerance, v: V) -> Option<V::Out> {
    use crate::number::{Arithmetic::*, Math::*};
    use rustymath::{self as rm, tolerant as tol};
    use Comparison::*;
    Some(match (p, arity == 2) {
        (Primitive::Arithmetic(Plus), false) => v.visit(rm::Conj),
        (Primitive::Arithmetic(Plus), true) => v.visit(rm::Add),
        (Primitive::Arithmetic(Minus), false) => v.visit(rm::Negate),
        (Primitive::Arithmetic(Minus), true) => v.visit(rm::Subtract),
        (Primitive::Arithmetic(Times), false) => v.visit(rm::Sign),
        (Primitive::Arithmetic(Times), true) => v.visit(rm::Multiply),
        (Primitive::Arithmetic(Divide), false) => v.visit(rm::Reciprocal),
        (Primitive::Arithmetic(Divide) | Primitive::Exact(Exact::Quotient), true) if integral || p == Primitive::Exact(Exact::Quotient) =>
            if mathish { v.visit(tol::MathishFloorDivide(t)) } else { v.visit(tol::FloorDivide(t)) },
        (Primitive::Arithmetic(Divide), true) => if mathish { v.visit(rm::MathishDivide) } else { v.visit(rm::Divide) },
        (Primitive::Math(Magnitude), false) => v.visit(rm::Abs),
        (Primitive::Math(Magnitude), true) => v.visit(tol::Residue(t)),
        (Primitive::Math(Floor) | Primitive::Exact(Exact::Floor), false) => v.visit(tol::Floor(t)),
        (Primitive::Math(Floor), true) => v.visit(rm::reduce::Minimum),
        (Primitive::Math(Ceiling) | Primitive::Exact(Exact::Ceiling), false) => v.visit(tol::Ceil(t)),
        (Primitive::Math(Ceiling), true) => v.visit(rm::reduce::Maximum),
        (Primitive::Math(Power), false) => v.visit(rm::Exp),
        (Primitive::Math(Power), true) => v.visit(rm::Pow),
        (Primitive::Math(Log), false) => v.visit(rm::Ln),
        (Primitive::Math(Log), true) => if mathish { v.visit(rm::MathishLogb) } else { v.visit(rm::Logb) },
        (Primitive::Math(Circle), false) => v.visit(rm::Cis),
        (Primitive::Math(Pi), false) => v.visit::<_, 1>(rm::PiTimes),
        (Primitive::Math(Pi), true) => v.visit::<_, 2>(rm::PiTimes),
        (Primitive::Math(Root), false) => v.visit(rm::Sqrt),
        (Primitive::Math(Root), true) => v.visit(rm::Root),
        (Primitive::Real(Real::Sqrt), false) => v.visit(rm::RealSqrt),
        (Primitive::Real(Real::Ln), false) => v.visit(rm::RealLn),
        (Primitive::Real(Real::Pow), true) => v.visit(rm::RealPow),
        (Primitive::Math(Factorial), false) => v.visit(rm::Factorial),
        (Primitive::Math(Factorial), true) => v.visit(rm::Flipped(rm::Binom)),
        (Primitive::Math(Gcd), true) => v.visit(tol::Gcd(t)),
        (Primitive::Math(Lcm), true) => v.visit(tol::Lcm(t)),
        (Primitive::Math(Nand), true) => v.visit(rm::Nand),
        (Primitive::Math(Nor), true) => v.visit(rm::Nor),
        (Primitive::Math(Not), false) => v.visit(rm::Not),
        (Primitive::Compare(Equal), true) => v.visit(tol::Equal(t)),
        (Primitive::Compare(NotEqual), true) => v.visit(tol::NotEqual(t)),
        (Primitive::Compare(Less), true) => v.visit(tol::Less(t)),
        (Primitive::Compare(LessEqual), true) => v.visit(tol::LessEqual(t)),
        (Primitive::Compare(Greater), true) => v.visit(tol::Greater(t)),
        (Primitive::Compare(GreaterEqual), true) => v.visit(tol::GreaterEqual(t)),
        (Primitive::Bitwise(table), false) => v.visit::<_, 1>(rm::Bitwise(table)),
        (Primitive::Bitwise(table), true) => v.visit::<_, 2>(rm::Bitwise(table)),
        (Primitive::Special(special), _) => match special {
            Special::Log1p => v.visit(rm::Log1p), Special::Expm1 => v.visit(rm::Expm1), Special::LambertW => v.visit(rm::LambertW), Special::Gammaln => v.visit(rm::Lgamma),
            Special::Digamma => v.visit(rm::Digamma), Special::Erf => v.visit(rm::Erf), Special::Erfc => v.visit(rm::Erfc),
            Special::Erfinv => v.visit(rm::Erfinv), Special::Polygamma => v.visit(rm::Polygamma), Special::Beta => v.visit(rm::Beta),
            Special::Gammainc => v.visit(rm::Gammainc), Special::Betainc => v.visit(rm::Betainc), Special::Sign => v.visit(rm::Sign),
            Special::BetaincAt([a, b]) => v.visit(rm::BetaincAt(f64::from_bits(a), f64::from_bits(b))),
            Special::BinomDn => v.visit(rm::BinomDn), Special::BinomDk => v.visit(rm::BinomDk), _ => return None,
        },
        _ => return None,
    })
}

/// The identity of a fold by dyadic `p`, from its formula: the identity on the right, else on the left (`Formula::identity`), since an empty fold gives the identity whichever side it holds on. `None` where the formula declares no identity.
pub(crate) fn fold_identity(p: Primitive) -> Option<f64> {
    struct Of;
    impl Visit for Of {
        type Out = Option<f64>;
        fn visit<K: rustymath::facts::Formula<N>, const N: usize>(self, object: K) -> Self::Out {
            let id = object.identity();
            id.last().copied().flatten().or(id.first().copied().flatten())
        }
    }
    formula(p, 2, false, false, rustymath::tolerant::Tolerance::DEFAULT, Of).flatten()
}

/// Whether `p` with two arguments, or with one where not `dyadic`, gives Booleans, from its formula (`Formula::BOOLEAN`).
pub(crate) fn booleans(p: Primitive, dyadic: bool) -> bool {
    struct Of;
    impl Visit for Of {
        type Out = bool;
        fn visit<K: rustymath::facts::Formula<N>, const N: usize>(self, _: K) -> bool { K::BOOLEAN }
    }
    formula(p, 1 + usize::from(dyadic), false, false, rustymath::tolerant::Tolerance::DEFAULT, Of).unwrap_or(false)
}

/// The formula that `p` computes, resolved for arguments of types `types` in `variant` and compared within the tolerance of `variant`'s kind (`Numeric::tolerance_of`): the one rule that turns a call into a kernel. `code` is a constant left argument that picks the formula, as `○`'s does. A resolution depends on nothing else, so each one is kept and reused, and a call doesn't resolve its kernel again. `None` where `p` has no formula for them.
fn resolved(p: Primitive, code: Option<i64>, variant: rustymath::facts::Variant, types: &[DType], numeric: Numeric) -> Option<Kernel> {
    use crate::number::Math;
    type Key = (Primitive, Option<i64>, rustymath::facts::Variant, bool, [Option<DType>; 3], u64);
    thread_local! { static KERNELS: std::cell::RefCell<rustymath::hashing::HashMap<Key, Option<Kernel>>> = Default::default(); }
    struct At<'a>(rustymath::facts::Variant, &'a [DType]);
    impl Visit for At<'_> {
        type Out = Option<Kernel>;
        fn visit<K: rustymath::facts::Formula<N>, const N: usize>(self, object: K) -> Self::Out {
            Kernel::try_resolve(object, self.0, <[DType; N]>::try_from(self.1).ok()?).ok().flatten()
        }
    }
    let tolerance = numeric.tolerance_of(variant.dtype);
    let mathish = numeric.mathish_at(variant.dtype);
    let key = (p, code, variant, mathish, std::array::from_fn(|i| types.get(i).copied()), tolerance.value().to_bits());
    if let Some(kernel) = KERNELS.with_borrow(|kernels| kernels.get(&key).cloned()) { return kernel; }
    let circle = match p {
        Primitive::Math(Math::Circle) => Some((false, false)), Primitive::Math(Math::Arc) => Some((true, false)),
        Primitive::Real(Real::Circle) => Some((false, true)), Primitive::Real(Real::Arc) => Some((true, true)), _ => None,
    };
    let kernel = match (circle, code) {
        (Some((inverse, real_part)), Some(code)) => types.last().and_then(|&kind| Kernel::circle(code, inverse, real_part, variant, [kind]).ok().flatten()),
        _ => formula(p, types.len(), mathish, types.iter().all(|kind| kind.is_integral()), tolerance, At(variant, types)).flatten(),
    };
    KERNELS.with_borrow_mut(|kernels| kernels.insert(key, kernel.clone()));
    kernel
}

pub(crate) use rustymath::reduce::Reduce;

/// Whether `kernel` gives every result its result kind for any values of its argument types (`Kernel::may_raise`). That decides the integer kind a call computes at (`total`) and the functions a select can run on floats. A call's own operands can narrow it further, as a constant's value does (`Pervasive::never_fails`).
pub(crate) fn settles(kernel: &Kernel) -> bool { !kernel.may_raise() }

/// The variant that a call computes in at `dtype` under `numeric`: checked under dynamic kinds and wrapping where `dtype`'s integers wrap (`Numeric::wraps`). Mathish is the formula's own fact: `resolved` picks the Mathish formula.
pub(crate) fn variant(dtype: DType, numeric: Numeric) -> rustymath::facts::Variant {
    let wrapping = dtype.is_integral() && numeric.wraps(Some(dtype), numeric.fixed);
    rustymath::facts::Variant { dtype, checked: !numeric.is_static(), wrapping }
}

/// Whether `p` gives whole-number results from floats of `kind` under `numeric`, which `whole` makes integers: `⌊`, `⌈` and monadic `×` under dynamic kinds, and the `ₓ` forms of `⌊`, `⌈` and dyadic `÷` under either. Static kinds keep `⌊`, `⌈` and `×` in floats.
pub(crate) fn integral(p: Primitive, dyadic: bool, kind: DType, numeric: Numeric) -> bool {
    use crate::number::{Arithmetic::Times, Math::{Ceiling, Floor}};
    let exact = matches!(p, Primitive::Exact(Exact::Floor | Exact::Ceiling | Exact::Quotient));
    let whole = exact || !numeric.is_static() && !dyadic && matches!(p, Primitive::Math(Floor | Ceiling) | Primitive::Arithmetic(Times));
    whole && matches!(kind, DType::Float(_))
}

/// The whole floats `floats` as integers, saturating as Rust's `as` does (`Buffer::cast_results`): NaN gives 0, and an infinity or a value past the width gives the width's nearest limit, as `⌊2*70` gives the largest i64. Where integers wrap, the width is the `int` width. Otherwise it's the width that stores the floats' range (`ordinary_width`), from their least and greatest, so storage keeps the buffer as it is; i64 where those come out NaN.
fn whole(floats: View<'_>, numeric: Numeric) -> Buffer {
    use rustymath::{convert::saturated, reduce::{max, min}};
    let range = match floats {
        View::Floats(f) if !numeric.wrapping() => with_floats!(f, |xs| { let (lo, hi) = (min(xs), max(xs)); (!lo.is_nan() && !hi.is_nan()).then(|| (saturated::<i64, _>(lo), saturated::<i64, _>(hi))) }),
        _ => None,
    };
    let width = if numeric.wrapping() { numeric.integer_width() } else { range.map_or(Width::I64, |(lo, hi)| crate::array::ordinary_width(Width::narrowest(lo, hi))) };
    Buffer::cast_results(floats, DType::Integer(width), false).expect("floats convert to integers")
}

/// `out`, `p`'s results computed at `kind`, with its whole results as integers where `integral` says `p` gives them.
fn whole_results(p: Primitive, dyadic: bool, kind: DType, out: Buffer, numeric: Numeric) -> Buffer {
    if integral(p, dyadic, kind, numeric) && matches!(out.view(), View::Floats(_)) { whole(out.view(), numeric) } else { out }
}

/// `n`, `p`'s result computed at `kind`, as an integer where `integral` says `p` gives one (`whole_results`).
fn whole_number(p: Primitive, dyadic: bool, kind: DType, n: Number, numeric: Numeric) -> Number {
    let (true, Ok(x)) = (integral(p, dyadic, kind, numeric), n.to_float()) else { return n };
    match Value::from_buffer_at(Vec::<usize>::new(), whole(View::Floats(Floats::F64(&[x])), numeric), numeric.fixed).map(|v| v.at(0)) { Ok(Value::Number(m)) => m, _ => n }
}

/// The circle code that the constant `c` is, for the kernels of `○`, when it's a whole number.
pub(crate) fn circle_code(c: f64) -> Option<i64> { (c.fract() == 0.0).then_some(c as i64) }
/// The code that `left`, the left argument of a function whose left argument picks its formula, as `○`'s does, gives: a real whole number, as `circle_code` reads it. The one reader of such a code, for eager calls and `∂` alike. `None` for anything else.
pub(crate) fn code_of(left: &Value) -> Option<i64> { left.as_real().and_then(circle_code) }

/// `run` on `values` with their 16-bit floats copied as 32-bit floats, rounding float results back to 16 bits where `halve` holds. Copies are needed because basedpl's kernel tables, folds, scans, inner products and scatters have no 16-bit instances, and one conversion here serves them all. rustymath's function objects have 16-bit forms, which compute each item in 32 bits, and Python's functions use those. `Number::halves` applies the same rule to numbers.
fn copied_halves(values: &[Option<&Value>], halve: bool, run: impl FnOnce(&[Option<&Value>]) -> Option<Value>) -> Option<Value> {
    let single =
        |v: &Value| crate::fusion::at_width(v, FloatWidth::F32).unwrap_or_else(|| v.to_float_width(FloatWidth::F32).expect("reals convert to 32-bit floats"));
    let copies: Vec<_> =
        values.iter().map(|v| v.map(|v| if v.float_width() == Some(FloatWidth::F16) { Cow::Owned(single(v)) } else { Cow::Borrowed(v) })).collect();
    let result = run(&copies.iter().map(|v| v.as_deref()).collect::<Vec<_>>());
    result.map(|value| halved(value, Some(halve)))
}
/// `run` on `values`, or, where a plan computes their 16-bit floats in 32 bits (`halve`), on 32-bit copies of them (`copied_halves`).
#[inline(always)]
fn planned_halves(values: &[Option<&Value>], halve: Option<bool>, run: impl FnOnce(&[Option<&Value>]) -> Option<Value>) -> Option<Value> {
    match halve { Some(halve) => copied_halves(values, halve, run), None => run(values) }
}

/// `value` with its 32-bit floats rounded to 16 bits where `halve` says that the call's float results do, as `plan::halved` gives its kind.
fn halved(value: Value, halve: Option<bool>) -> Value {
    if halve == Some(true) && matches!(value.as_items(), Items::Compact(View::Floats(Floats::F32(_)))) { value.to_float_width(FloatWidth::F16).expect("32-bit floats narrow") }
    else { value }
}

/// The kind search that `map`, `number` and `scatter` share. From `dtype`, it gets a kernel from `kernel` for each kind's variant and runs `attempt` through it. A Boolean kind with no kernel tries the next kind. Under dynamic kinds, an `Overflow` reruns at the next integer kind (`next_kind`). At 64 bits the variant wraps (`variant`) and reports no overflow. A `Domain` failure at a float kind reruns complex. A failure at an integer kind never reruns as floats. Gives `attempt`'s result and the kind it ran at, `None` where no kind has a kernel, or the failure that ended the search, which under static kinds is any failure.
fn searched<R>(
    mut dtype: DType, numeric: Numeric, kernel: impl Fn(rustymath::facts::Variant) -> Option<Kernel>,
    mut attempt: impl FnMut(&Kernel) -> Result<R, rustymath::error::Error>,
) -> Result<Option<(R, DType)>, rustymath::error::Error> {
    use rustymath::error::Kind;
    loop {
        let variant = variant(dtype, numeric);
        let Some(kernel) = kernel(variant) else {
            match next_kind(dtype, numeric).filter(|_| dtype == DType::Boolean) { Some(next) => { dtype = next; continue; } None => return Ok(None) }
        };
        match attempt(&kernel) {
            Ok(result) => return Ok(Some((result, dtype))),
            Err(e) if numeric.is_static() => return Err(e),
            Err(e) => match (e.kind, dtype, next_kind(dtype, numeric)) {
                (Kind::Overflow, DType::Boolean | DType::Integer(_), Some(next)) => dtype = next,
                (Kind::Domain, DType::Float(_), _) => dtype = dtype.complex(),
                _ => return Err(e),
            },
        }
    }
}

/// `searched` through `p`'s formula (`resolve`) on arguments of kinds `types`, from `dtype`, which `first_kind` refines for integers. `code` is a constant left argument that picks the formula, as `○`'s does.
fn computed<R>(
    p: Primitive, code: Option<i64>, types: &[DType], mut dtype: DType, numeric: Numeric,
    attempt: impl FnMut(&Kernel) -> Result<R, rustymath::error::Error>,
) -> Result<Option<(R, DType)>, rustymath::error::Error> {
    if dtype.is_integral() { dtype = first_kind(p, types.len() == 2, types.first().copied().filter(|_| types.len() == 2), types.last().copied(), dtype, numeric); }
    searched(dtype, numeric, |variant| resolved(p, code, variant, types, numeric), attempt)
}

/// Reads of `args` as rustymath kernels take them, with each value's kind: each value through its mapping onto `len` items, with `missing` for the positions a mapping reaches no item at. `None` where a value isn't compact.
pub(crate) fn with_reads<R>(args: &[(&Value, &Mapping, usize)], missing: Missing, f: impl FnOnce(&[Read<'_>], &[DType]) -> R) -> Option<R> {
    use rustymath::agreement::{Binding, ReadPlan};
    let views: Vec<View<'_>> = args.iter().map(|(value, ..)| value.view()).collect::<Option<_>>()?;
    let types: Vec<DType> = views.iter().map(|v| v.dtype()).collect();
    let sources: Vec<_> = views.iter().map(|&view| Binding::full(view)).collect();
    let plans: Vec<_> = args.iter().zip(&views).enumerate()
        .map(|(source, ((_, mapping, len), view))| ReadPlan::copy(source, view.len(), (*mapping).clone(), *len, missing.clone())).collect();
    let reads: Vec<_> = plans.iter().map(|plan| Read { sources: &sources, plan }).collect();
    Some(f(&reads, &types))
}

/// `kernel` on `reads`, for `len` items.
fn applied(kernel: &Kernel, reads: &[Read<'_>], len: usize, settings: rustymath::operation::Settings) -> Result<Buffer, rustymath::error::Error> {
    let mut out = Buffer::with_capacity(kernel.result_dtype(), len);
    kernel.map(reads, 0..len, settings, &mut out).map(|()| out)
}

/// A pervasive function applied to stored arguments on `agreement`'s frame, as `plan` plans it, through `p`'s formula (`resolve`) at the kind the plan computes in, with `computed`'s reruns. Each argument reads through its agreement mapping and converts to that kind as it reads. `None` where no kind gives the result, and for any failure under static kinds: `number` then computes it item by item.
pub(crate) fn map(plan: &Pervasive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Value> {
    let (p, numeric, len) = (plan.p, plan.numeric, agreement.len);
    let code = match left { Some(left) if p.info().dyad.code => Some(code_of(left)?), _ => None };
    let args: Vec<(&Value, &Mapping, usize)> = match left.filter(|_| code.is_none()) {
        Some(left) => vec![(left, &agreement.left, len), (right, &agreement.right, len)], None => vec![(right, &agreement.right, len)]
    };
    let at = plan.computes()?;
    let settings = numeric.settings_for(at);
    let (out, dtype) = with_reads(&args, Missing::Default, |reads, types| {
        computed(p, code, types, at, numeric, |kernel| applied(kernel, reads, len, settings)).ok().flatten()
    })??;
    let out = whole_results(p, plan.dyadic, dtype, out, numeric);
    let value = settled(Value::from_buffer_at(agreement.layout.clone(), out, numeric.fixed).ok()?, || Some(dtype), numeric);
    Some(halved(value, plan.halve))
}

/// `p` on two items, one or both characters, as a one-item call through `computed`: the character policy of `p`'s formula decides the result. An `ᵣ` number reads as the machine number it stands for, a whole one as an ordinary integer and a fraction as a float, and where the policy picks one argument, as `⌊` does between a number and a character, the result is that argument itself. `None` where `p` takes no characters, or none of these items.
pub(crate) fn character_item(p: Primitive, x: &Value, y: &Value) -> Option<Result<Value, crate::Error>> {
    if !matches!(x, Value::Character(_)) && !matches!(y, Value::Character(_)) { return None; }
    let numeric = Numeric::current();
    let settings = numeric.settings_for(DType::Character);
    let read = |v: &Value| match v {
        Value::Number(n) if n.is_unbounded() => Some(Value::Number(if n.is_integral() { Number::from_integer(n.integer().ok()?) } else { numeric.float(n.to_float().ok()?) })),
        v => Some(v.clone()),
    };
    let (a, b) = (read(x)?, read(y)?);
    let mut picked = None;
    let result = with_reads(&[(&a, &Mapping::Single, 1), (&b, &Mapping::Single, 1)], Missing::Default, |reads, types| {
        computed(p, None, types, DType::Character, numeric, |kernel| { picked = kernel.selected_input(); applied(kernel, reads, 1, settings) })
    })?;
    match result {
        Ok(Some(_)) if let Some(i) = picked => Some(Ok([x, y][i].clone())),
        Ok(Some((out, _))) => Some(Ok(Value::from_buffer(Vec::<usize>::new(), out).expect("one item fills a rank-0 array").at(0))),
        Ok(None) => None,
        Err(e) => Some(Err(e.into())),
    }
}

/// The common kind of compact arguments of kinds `left` and `right`, as `Numeric::kind` gives them, which a dyadic call on them computes in, except as `computes_in` says. Atoms promote as arrays do. An integer beside a float takes the float's width, and two widths of one kind take the wider.
fn joined(left: Option<DType>, right: Option<DType>, numeric: Numeric) -> Option<DType> { numeric.kind(left?).join(numeric.kind(right?), Widening::Arrays) }
/// The kind that a dyadic call of `p` on compact arguments of kinds `left` and `right` computes in. Where `p`'s left argument is a code that picks a function, as for `○`, that's the right argument's kind, as `Numeric::kind` gives it, and the code reads at it. Otherwise it's the kind that `joined` gives both.
pub(crate) fn computes_in(p: Primitive, left: Option<DType>, right: Option<DType>, numeric: Numeric) -> Option<DType> {
    if p.info().dyad.code { Some(numeric.kind(right?)) } else { joined(left, right, numeric) }
}

/// `p` applied to the items of `z` at `writes`, in turn and in place: `z[o]` becomes `z[o] p y`, where `y` is the write's item of `values`, or its only item. A repeated place applies `p` to the result of the one before. Items compute at the kind that `map` computes `z p values` in, and `z` converts to that kind first. `meets` is the rule that joins `z`'s kind with that kind. An assignment passes `Widening::Items`, so a call whose results would leave `z` mixed gives `None` and takes the assignment's general path. A sum passes `Widening::Arrays`. 16-bit floats accumulate in 32 bits and round once. Only `+ - × ÷ ⌈ ⌊` on compact numbers take this path. It gives the route it took, and `None`, with `z` unchanged, for any other call, and where an item needs the `Number` path, such as an integer result that doesn't fit 64 bits.
pub(crate) fn scatter(p: Primitive, z: &mut Value, writes: Writes<'_>, values: &Value, meets: Widening) -> Option<Route> {
    use {Arithmetic::*, Math::*};
    if !matches!(p, Primitive::Arithmetic(Plus | Minus | Times | Divide) | Primitive::Math(Ceiling | Floor)) || !matches!(z, Value::Array(_)) { return None; }
    if z.carries_here() || values.carries_here() { return None; }
    let weak = p.weaken_value(values, z);
    let values = weak.as_ref().unwrap_or(values);
    let plan = plan::pervasive_on(p, Some(&*z), values, Numeric::current()).ok()?;
    let Some(halve) = plan.halve else { return scatter_under(&plan, z, writes, values, meets) };
    let wide = copied_halves(&[Some(&*z), Some(values)], halve, |v| {
        let mut wide = v[0]?.clone();
        scatter_under(&plan, &mut wide, writes, v[1]?, meets).map(|_| wide)
    });
    wide.map(|wide| { *z = wide; Route::Copied })
}

/// How `scatter` added its values: in place in `z`'s own items, or on a copy of `z`, converted for 16-bit floats, that then replaces it.
#[derive(Clone, Copy)]
pub(crate) enum Route { InPlace, Copied }

impl Route {
    /// The route's name, as `•explain` reports it.
    pub(crate) fn name(self) -> &'static str { match self { Route::InPlace => "in place", Route::Copied => "on a copy" } }
}

/// The own code of `p` paired by `Pairing::At`: `values` combined into `base` at `writes`, in place. A singleton goes to every write, as a unit. `scatter` combines compact numbers where it can, as `Widening::Items` joins items. Otherwise one call of `p` covers writes at distinct positions. Repeated positions take rounds. Each round is one call on distinct positions, reading what earlier rounds wrote. A write at a fill is dropped. `•explain` records the path as an accumulation. It records none for offsets in one ascending run, a block. An error leaves `base` unchanged.
pub(crate) fn combine_at(p: Primitive, base: &mut Value, writes: Writes<'_>, values: &Value, span: &crate::Span) -> Result<(), crate::Error> {
    use crate::{error::Described, primitive::{Selection, Targets}};
    let block = matches!(writes, Writes::Offsets(o) if o.windows(2).all(|w| w[1] == w[0] + 1));
    let (count, size) = (writes.len(), base.len());
    let report = |path: &'static str| { if !block { crate::explain::accumulated(count, size, path) } };
    let unit = if values.is_singleton() {
        let item = values.at(0);
        Some(if item.is_atom() { item } else { item.enclose().described("invalid modified selection")? })
    } else { None };
    // The fallbacks pair a flat list of writes with flat values.
    let flat;
    let values = match &unit {
        Some(unit) => unit,
        None if values.shape().len() != 1 => { flat = values.clone().with_shape(vec![values.len()]).described("invalid modified selection")?; &flat }
        None => values,
    };
    if let Some(route) = scatter(p, base, writes, values, Widening::Items) { report(route.name()); return Ok(()); }
    let listed = writes.listed().into_owned();
    let all = Selection::offsets(listed);
    let kept = all.without_fills(values, span)?;
    let (selection, values) = match &kept { Some((kept, values)) => (kept, values), None => (&all, values) };
    let Targets::Offsets(offsets) = &selection.targets else { unreachable!("listed targets are offsets") };
    let Some((counts, rounds)) = rustymath::search::occurrences(offsets, size) else {
        report("one call");
        let items = selection.read(base, None, span)?;
        let results = p.call(Some(&items), values, span)?;
        return selection.write_into(base, &results, span);
    };
    report("rounds");
    // Round r holds the r-th write at each position. Its positions are distinct. One call computes the round from the items that earlier rounds wrote into a copy of `base`, which then replaces it.
    let order = crate::search::grouped_by(&counts, rounds, |c| c as usize)?;
    let mut current = base.clone();
    for targets in order.each() {
        let round = Selection::offsets(targets.iter().map(|&t| offsets[t as usize]).collect());
        let items = round.read(&current, None, span)?;
        let values =
            if unit.is_some() { values.clone() } else { Selection::offsets(targets.iter().map(|&t| t as usize).collect()).read(values, None, span)? };
        let results = p.call(Some(&items), &values, span)?;
        round.write_into(&mut current, &results, span)?;
    }
    *base = current;
    Ok(())
}

/// `scatter` as `plan` plans it, where no argument holds 16-bit floats: one call of rustymath's `operation::at` on `z`'s items, at the kind the plan computes in, with `computed`'s reruns. Converting `z` to that kind gives what writing each result into it gives only where `meets` joins the two kinds to that one. A call that picks an argument by its order class (`Pervasive::picking`), as `z.[p]⌊←∞` does, takes the general path, which keeps the picked items' kind.
fn scatter_under(plan: &Pervasive, z: &mut Value, writes: Writes<'_>, values: &Value, meets: Widening) -> Option<Route> {
    use rustymath::operation::{Positions, WriteRoot};
    let (p, numeric, computes) = (plan.p, plan.numeric, plan.computes()?);
    if z.dtype()?.join(computes, meets) != Some(computes) { return None; }
    if plan.picking().is_some() { return None; }
    let (len, n) = (z.len(), writes.len());
    // The writes' flat offsets, where a fill place (`Steps::FILL`) writes nothing.
    let listed = writes.listed();
    // A single value spreads over every write: the read of `values` has an item for each write.
    let map = if values.len() == 1 { Mapping::Single } else { Mapping::Linear(1) };
    let settings = numeric.settings_for(computes);
    // A kernel that can't fail an item runs once, so it writes into `z`'s own items where `z` holds them alone (`Value::take_buffer`).
    if z.dtype() == Some(computes) && plan.never_fails() {
        let written = with_reads(&[(values, &map, n)], Missing::Default, |reads, types| {
            let mut root = Some(z.take_buffer()?);
            Some(computed(p, None, &[computes, types[0]], computes, numeric, |kernel| {
                let mut out = Buffer::with_capacity(kernel.result_dtype(), len);
                let root = WriteRoot::Owned(root.take().expect("a kernel that can't fail an item runs once"));
                let ids = Positions::Offsets(&listed);
                rustymath::operation::at(Some(kernel), root, reads[0], ids, kernel.variant.checked, settings, &mut out).map(|()| out)
            }).ok().flatten().expect("a write by a kernel that can't fail an item succeeds"))
        }).flatten();
        if let Some((out, dtype)) = written {
            let written = Value::from_buffer_at(z.layout().clone(), out, numeric.fixed).ok().expect("an in-place write keeps its layout");
            *z = settled(written, || Some(dtype), numeric);
            return Some(Route::InPlace);
        }
    }
    let (out, dtype) = with_reads(&[(&*z, &Mapping::Linear(1), len), (values, &map, n)], Missing::Default, |reads, types| {
        computed(p, None, &types[..2], computes, numeric, |kernel| {
            let mut out = Buffer::with_capacity(kernel.result_dtype(), len);
            let ids = Positions::Offsets(&listed);
            rustymath::operation::at(Some(kernel), WriteRoot::Read(reads[0]), reads[1], ids, kernel.variant.checked, settings, &mut out).map(|()| out)
        }).ok().flatten()
    })??;
    *z = settled(Value::from_buffer_at(z.layout().clone(), out, numeric.fixed).ok()?, || Some(dtype), numeric);
    Some(Route::Copied)
}

/// The kind that `p` computes in on arguments stored as `kind` under `numeric`, where `constants` gives the arguments that are one real number: the first kind on rustymath's ladder (`DType::computes_in`) whose kernel table has `p`. Exact numbers compute exactly, through the integer table or the `Number` path, where `p`'s formula computes integers: its kernel at an integer kind stays at an integer kind, where `Kernel::try_resolve` would otherwise promote to a float (`Formula::computing_dtype`). Otherwise they compute as floats at `Numeric::exact_width`, as for `⍟`, `√`, monadic `*`, `π` and dyadic `○` at every code. Integer `÷` floors, at every kind. An integer kind is the narrowest from `kind` upward at which no result can fail (`total`), without reading `constants`. Booleans compute as Booleans only where `p` is `logical`, and otherwise as exact numbers do, at the narrowest integer kind that holds every result, from the one that `next_kind` gives, as `-` on Booleans computes at 16 bits. `None` where no kind on the way has `p`, as for `○` with no constant code.
pub(crate) fn computes(p: Primitive, dyadic: bool, constants: [Option<f64>; 2], kind: DType, numeric: Numeric) -> Option<DType> {
    let Some(DType::Integer(int)) = next_kind(DType::Boolean, numeric) else { unreachable!("a Boolean's next kind is an integer") };
    let stored = kind;
    let exact = || table_kernel(p, dyadic, DType::Integer(int), stored, constants, numeric).is_some_and(|k| k.variant.dtype.is_integral());
    let kind = numeric.kind(kind);
    if matches!(kind, DType::Integer(_)) && exact() { return Some(total(p, dyadic, stored, kind, numeric)); }
    let at = kind.computes_in(int, numeric.exact_width(kind), |d| match d {
        DType::Integer(_) => exact(), DType::Boolean if !logical(p, dyadic) => false, d => table_kernel(p, dyadic, d, stored, constants, numeric).is_some()
    })?;
    Some(if matches!(at, DType::Integer(_)) { total(p, dyadic, stored, at, numeric) } else { at })
}

/// Whether `p` computes on Booleans as Booleans: comparisons, `~`, `⊼`, `⊽`, bitwise functions, and dyadic `∧`, `∨`, `⌊` and `⌈`. Other functions read Booleans as the integers 0 and 1.
fn logical(p: Primitive, dyadic: bool) -> bool {
    booleans(p, dyadic) || matches!(p, Primitive::Bitwise(_)) || dyadic && (p.info().dyad.picks || matches!(p, Primitive::Math(Math::Lcm | Math::Gcd)))
}
/// `items`, which a fold or scan by `p` gives as they are because each of their lanes holds one item, at the kind that a longer lane gives. Booleans become the integers that `p` computes from them (`plan::Fold::result`), as a count by `+` does. Where integers wrap, under static kinds or at a fixed width, integers become the floats that `p` computes from them (`plan::Fold::floated`). Dynamic kinds keep ordinary integers as they are.
pub(crate) fn lone(p: Primitive, items: Value) -> Value {
    let numeric = Numeric::current();
    let Ok(plan) = plan::fold(p, FoldKind::REDUCE, None, Kind::value(&items), numeric) else { return items };
    match (plan.floated(), items.dtype(), plan.result()) {
        (Some(width), ..) => items.to_float_width(width).unwrap_or(items),
        (None, Some(DType::Boolean), Some(DType::Integer(width))) => items.to_integers(width, numeric).unwrap_or(items), _ => items
    }
}

/// Whether `p` is a `ᵇ` table that maps two 0 bits to 1. Its results from nonnegative integers are negative, with every bit above the items' own set. On Booleans it computes the logical function instead. That gives other results than the integers 0 and 1 give.
pub(crate) fn sets_high_bits(p: Primitive) -> bool { matches!(p, Primitive::Bitwise(table) if table & 1 != 0) }
pub(crate) fn seed_last(p: Primitive, right: &Value) -> bool {
    let folds = right.dtype().and_then(|kind| kernel_at(p, None, kind, 2, Numeric::current())).is_some_and(|k| k.fold().is_some());
    folds && !(sets_high_bits(p) && right.dtype() == Some(DType::Boolean))
}

/// The kind that an exact call tries after `kind`, where it has no kernel at `kind` or a result doesn't fit: the `int` width after Booleans under static kinds, and the next wider integer width under dynamic kinds.
fn next_kind(kind: DType, numeric: Numeric) -> Option<DType> {
    match (numeric.wrapping(), kind) { (true, DType::Boolean) => Some(DType::Integer(numeric.integer_width())), (false, kind) => kind.wider(), _ => None }
}
/// The narrowest integer kind, from `from` upward (`next_kind`), at which `p`'s formula on arguments stored as `stored` can't fail, as rustymath's validity facts at their types' ranges say (`settles`): `-` on Booleans or bytes computes at 16 bits, and `+` on bytes too. `from` where integers wrap, and where no width settles.
fn total(p: Primitive, dyadic: bool, stored: DType, from: DType, numeric: Numeric) -> DType {
    if numeric.wrapping() { return from; }
    std::iter::successors(Some(from), |&k| next_kind(k, numeric))
        .find(|&k| table_kernel(p, dyadic, k, stored, [None; 2], numeric).is_some_and(|kernel| settles(&kernel)))
        .unwrap_or(from)
}

/// The kind that an exact call of `p` on operands of storage kinds `left` and `right` tries first, where their kinds join to `dtype`: the kind that `Numeric::kind` computes `dtype` in, which static kinds widen to the `int` width. A comparison computes at `dtype` itself, which its plan gives (`Pervasive::computes`). Other functions whose integer results are the same at every width that holds their arguments compute at their storage width instead, and read the storage as it is: a function that gives Booleans (`booleans`), `⌈` and `⌊`, which give one of their arguments (`picks`), and a bitwise function, whose high bits copy its arguments' sign bits. This refines the plan's kind (`computes`), which sets the results' kind and what a deferred call reads: these functions give the same integers at every width that holds their arguments, and `settled` wraps the results back to the plan's width, so an eager call and its deferred form give the same results. The two kinds agree under dynamic kinds, and under a fixed width, where the operands' storage widths join to the fixed width.
fn first_kind(p: Primitive, dyadic: bool, left: Option<DType>, right: Option<DType>, dtype: DType, numeric: Numeric) -> DType {
    if matches!(p, Primitive::Compare(_)) { return dtype; }
    let free = booleans(p, dyadic) || dyadic && p.info().dyad.picks || matches!(p, Primitive::Bitwise(_));
    let stored = || if dyadic { left?.join(right?, Widening::Arrays) } else { right };
    match free.then(stored).flatten() { Some(kind @ DType::Integer(_)) => total(p, dyadic, kind, kind, numeric), _ => numeric.kind(dtype) }
}

/// `result` as a call under `numeric` gives it, where `kind` gives its arguments' kind, which only static kinds read. Under static kinds, folds, scans and inner products total integers at 64 bits. Where integers wrap, an integer result wraps to the call's width through `Value::wrapped_at`, as `whole` wraps an elementwise result: the fixed width, or under static kinds the width of the arguments' kind, where Booleans count as integers of the `int` width. Wrapping once at the end gives the same low bits as wrapping at each step of `+`, `-` and `×`. Under dynamic kinds, `result` is unchanged.
fn settled(result: Value, kind: impl FnOnce() -> Option<DType>, numeric: Numeric) -> Value {
    if !numeric.wrapping() { return result; }
    let width = numeric.fixed.unwrap_or_else(|| match kind().map(|k| numeric.kind(k)) { Some(DType::Integer(width)) => width, _ => numeric.int });
    result.wrapped_at(width, numeric).expect("an integer result wraps to its width")
}

/// The formula of `p` at kind `kind` under `numeric`, on `arity` arguments of that kind, with `code` for a function whose left argument is a code (`resolved`). `None` where `p` has no formula there.
pub(crate) fn kernel_at(p: Primitive, code: Option<i64>, kind: DType, arity: usize, numeric: Numeric) -> Option<Kernel> {
    resolved(p, code, variant(kind, numeric), &[kind; 2][..arity], numeric)
}
/// The formula of `p` at kind `table` under `numeric`, on arguments stored as `stored`, whose types' ranges bound the failures that it can raise (`settles`), where `constants` are the arguments that are one real number. A function whose left argument is a code reads the code from `constants` and takes one argument. `None` where `p` has no formula there.
pub(crate) fn table_kernel(p: Primitive, dyadic: bool, table: DType, stored: DType, constants: [Option<f64>; 2], numeric: Numeric) -> Option<Kernel> {
    let coded = dyadic && p.info().dyad.code;
    let code = if coded { Some(constants[0].and_then(circle_code)?) } else { None };
    resolved(p, code, variant(table, numeric), &[stored; 2][..if dyadic && !coded { 2 } else { 1 }], numeric)
}

/// The arguments of a call that are one real number, as the loops that run kernels give them (`Dyad::constants`).
pub(crate) fn constants(left: Option<&Value>, right: &Value) -> [Option<f64>; 2] { [left.and_then(Value::as_real), right.as_real()] }

/// The argument that `plan`'s call on `left` and `right` gives unchanged (`Pervasive::kept`), where the result has its shape: the other argument is an atom, or the kept one isn't. A weak number beside a strong one isn't kept: the call's result is strong, as `∞⌈1ₓ₁` gives a strong `∞`, an ordinary float.
pub(crate) fn unchanged<'a>(plan: &crate::plan::Pervasive, left: Option<&'a Value>, right: &'a Value) -> Option<&'a Value> {
    let (kept, other) = if plan.kept()? == 0 { (left?, Some(right)) } else { (right, left) };
    let strengthens = matches!((kept, other), (Value::Number(n), Some(Value::Number(o))) if n.is_weak() && !o.is_weak());
    (!strengthens && other.is_none_or(|o| o.is_atom() || !kept.is_atom())).then_some(kept)
}

/// The kind of every item of `p`'s result on compact `left` and `right` under the current settings, where one route decides it for all items at once. That route is the kernel that the call's plan picks (`Pervasive::kernel`). Where the route can't take some item, another route computes the whole call. That route's kind then holds for every item, including items that the first route could take. `None` under static kinds, where the arguments' kinds decide every result's kind. Also `None` where no route takes those kinds, because the `Number` path then computes each item alone.
pub(crate) fn expected(p: Primitive, left: Option<&Value>, right: &Value) -> Option<DType> {
    let numeric = Numeric::current();
    if numeric.is_static() { return None; }
    plan::pervasive_on(p, left, right, numeric).ok()?.kernel().map(|k| k.result_dtype())
}

/// `p` on the number `y`, and on `x` where it's dyadic: the one path for functions on numbers, through `p`'s formula as `map` computes arrays. Machine numbers compute at the kind `map` chooses, with its reruns (`computed`). Exact numbers, rationals and unbounded integers, compute through the formula's exact form. Where the formula has no exact form or gives no exact result, as for an irrational root, they compute at the widest float argument's width, as a machine integer beside it does, or at `numeric`'s float width beside no float.
pub(crate) fn number(p: Primitive, x: Option<&Number>, y: &Number, numeric: Numeric) -> Result<Number, crate::number::Failure> {
    use crate::number::Failure;
    let code = match x {
        Some(x) if p.info().dyad.code => Some(code_of(&Value::Number(x.clone())).ok_or(Failure::Domain("circle code must be a whole number"))?),
        _ => None,
    };
    let pair;
    let args: &[&Number] = match x.filter(|_| code.is_none()) { Some(x) => { pair = [x, y]; &pair } None => std::slice::from_ref(&y) };
    let exact = args.iter().any(|n| n.is_unbounded());
    // An unbounded (`ᵣ`) argument, beside other `ᵣ` numbers or machine integers, computes through the formula's exact form and keeps the result unbounded, at denominator 1 too. Machine integers alone never reach it. An `ᵣ` number isn't a machine integer, so the formula is the one for floats, which divides exactly where machine integers floor. Whole numbers take the integer formula only where the function has none for floats, as bitwise functions don't.
    if exact && args.iter().all(|n| n.is_exact()) {
        let kinds = [DType::Float(FloatWidth::F64), DType::Integer(Width::I64)];
        let whole = args.iter().all(|n| n.is_integral());
        if let Some(kernel) = kinds[..1 + usize::from(whole)].iter().find_map(|&kind| kernel_at(p, code, kind, args.len(), numeric).filter(Kernel::has_exact)) {
            let rationals: Vec<_> = args.iter().map(|n| n.exact_ref().expect("an exact number").into_owned()).collect();
            if let Some(n) = kernel.exact(&rationals).map_err(failure)? { return Ok(if kernel.result_dtype() == DType::Boolean { Number::from_bool(!num_traits::Zero::is_zero(&n)) } else { Number::rational(n) }); }
        }
    }
    let width = args.iter().filter_map(|n| match n.dtype() { Some(DType::Float(w)) => Some(w), _ => None }).max().unwrap_or(numeric.width);
    let types: Vec<DType> = args.iter().map(|n| n.dtype().unwrap_or(DType::Float(width))).collect();
    let none = || Failure::Domain("this function has no formula for these numbers");
    let dtype = if exact { DType::Float(width) } else {
        let constants = [x.and_then(|x| x.to_float().ok()), y.to_float().ok()];
        crate::plan::pervasive(p, x.map(crate::plan::Kind::number), crate::plan::Kind::number(y), constants, numeric).map_err(Failure::Domain)?.computes().ok_or_else(none)?
    };
    match computed(p, code, &types, dtype, numeric, |kernel| one(kernel, args, numeric.settings_for(dtype), numeric)) {
        Ok(result) => result.map(|(n, kind)| whole_number(p, args.len() == 2, kind, n, numeric)).ok_or_else(none),
        Err(e) => Err(failure(e)),
    }
}

/// `kernel` on one item of each of `args`, read at its computing kind, as the number that compact storage of its result kind holds (`Items::item`). Where the two items' order classes differ (`Number::order_class`) and the formula is order-only, it gives one of them, or a truth value, without promotion (`Kernel::ordered`): `3ₓ⌊∞` gives `3ₓ`, and `(10ᵣ*400ₓ)<∞` gives `$t`.
fn one(kernel: &Kernel, args: &[&Number], settings: rustymath::operation::Settings, numeric: Numeric) -> Result<Number, rustymath::error::Error> {
    use rustymath::error::{Error, Kind};
    let unread = || Error::new(Kind::Domain, "a number that the computing kind can't read");
    use rustymath::characters::{ByKinds, Policy};
    if let [x, y] = args && let classes @ [Some(a), Some(b)] = [x.order_class(), y.order_class()] && a != b
        && let Some(Policy::Kinds(side)) = kernel.clone().ordered(&classes, &[kernel.variant.dtype; 2]).input_policy() {
        return Ok(match side { ByKinds::Truth(t) => Number::from_bool(t), side => args[usize::from(side == ByKinds::Right)].clone() });
    }
    let integer = |n: i64, result| match result {
        DType::Boolean => Number::from_bool(n != 0), DType::Integer(w) if numeric.fixed.is_some() => Number::fixed(n, w), _ => Number::from_integer(n),
    };
    match (kernel.variant.dtype, kernel.result_dtype()) {
        (DType::Character, _) | (_, DType::Character) => Err(unread()),
        (DType::Boolean | DType::Integer(_), result) => {
            let read = |n: &&Number| -> Result<i64, Error> { match n.as_integer() { Some(i) => Ok(i), None => Ok(rustymath::operation::whole_numbers(&[n.to_float().map_err(|_| unread())?])?[0]) } };
            let x = args.iter().map(read).collect::<Result<Vec<_>, Error>>()?;
            Ok(integer(kernel.integers(&x)?, result))
        }
        (DType::Float(_), result) => {
            let x = args.iter().map(|n| n.to_float().map_err(|_| unread())).collect::<Result<Vec<_>, _>>()?;
            let v = kernel.real(&x, settings)?;
            Ok(match result {
                DType::Float(w) => Number::float(v, w), DType::Complex(w) => Number::complex(num_complex::Complex64::new(v, 0.0), w.into()),
                DType::Boolean => Number::from_bool(v != 0.0), _ => integer(v as i64, result),
            })
        }
        (DType::Complex(_), result) => {
            let x = args.iter().map(|n| n.to_complex().map_err(|_| unread())).collect::<Result<Vec<_>, _>>()?;
            let z = kernel.complex(&x, settings)?;
            Ok(match result {
                DType::Complex(w) => Number::complex(z, w.into()), DType::Float(w) => Number::float(z.re, w),
                DType::Boolean => Number::from_bool(z.re != 0.0), _ => integer(z.re as i64, result),
            })
        }
    }
}

/// A formula's failure as a function on numbers reports it: DOMAIN with the formula's message, or the limit, overflow or interrupt that it raised.
fn failure(e: rustymath::error::Error) -> crate::number::Failure {
    use {crate::number::Failure, std::borrow::Cow};
    match (e.kind, e.message) {
        (rustymath::error::Kind::Domain, Cow::Borrowed(message)) => Failure::Domain(message),
        (rustymath::error::Kind::Domain, Cow::Owned(_)) => Failure::Domain("numerical argument is outside the function's domain"),
        (kind, _) => Failure::Interrupt(kind),
    }
}


mod fold;
mod product;
mod scan;
pub(crate) use product::{inner, reads_widened};
pub(crate) use scan::{inverse_scan, scan};
pub(crate) use fold::{fold, fold_groups, picks, sliding_2d, truth_table};
