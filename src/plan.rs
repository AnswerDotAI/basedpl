//! Operation planning: an operation's result type and layout, worked out from its operands' kinds and the numeric settings without reading any items. A pervasive call's plan (`Pervasive`) makes the decisions that eager dispatch, fusion and typed programs (`program`) share: the call's settings, the kind it computes in, its kernel table and, under static kinds, each result's kind. It works out the settings in `prepared` and the rest through `pervasive::computes_in`, `computes` and `kernel`. `map` takes a call's layout from its operands' `Agreement`. A fold's plan is its function's plan on two items (`Fold`), and a product's is `g`'s plan, which `f`'s fold of its results follows (`Product`).

use crate::{
    Number, Value, agreement::{Agree, Agreement}, array::{Layout, Promotion, Widening}, number::Numeric, primitive::{FoldKind, Primitive}, program::Desc
};
use rustymath::{agreement::{Frames, Mapping}, buffer::{DType, FloatWidth, Width}};

/// An operand's kind: its type, fixed width and weakness, without its items or shape. A call's promotion reads its operands' kinds, and collected items join theirs into one array's dtype (`Kind::join`), before any weak integer is checked for fit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Kind {
    pub(crate) dtype: Option<DType>,
    pub(crate) fixed: Option<Width>,
    pub(crate) weak: bool,
    weak_integer: bool,
    /// The narrowest width that holds an integer's value, which `fixed` checks a weak integer against. An ordinary integer atom's `dtype` takes the `int` width instead under static kinds (`Kind::number`).
    narrowest: Option<Width>,
}

/// The fixed width that parts give, built one part at a time, for `Kind::fixed` and `Kind::join`. `fixed` joins the parts' fixed widths, and `required` the narrowest widths of their weak integers. `misfit` marks a weak integer with no narrowest width, and `other` a part that is neither fixed, a Boolean nor a weak integer.
#[derive(Default)]
struct Fixing { fixed: Option<Width>, required: Option<Width>, misfit: bool, other: bool }
impl Fixing {
    #[inline]
    fn add(&mut self, part: Kind) {
        if self.other { return; }
        if let Some(w) = part.fixed { self.fixed = Some(self.fixed.map_or(w, |v| v.join(w))); }
        else if part.dtype == Some(DType::Boolean) { }
        else if part.weak_integer {
            match part.narrowest { Some(w) => self.required = Some(self.required.map_or(w, |v| v.join(w))), None => self.misfit = true }
        } else { self.other = true; }
    }
    fn width(self) -> Result<Option<Width>, &'static str> {
        let Some(width) = self.fixed.filter(|_| !self.other) else { return Ok(None) };
        if self.misfit || self.required.is_some_and(|w| w.join(width) != width) { return Err("integer scalar doesn't fit the fixed width"); }
        Ok(Some(width))
    }
}
impl Kind {
    /// The kind of `n`. Under static kinds an ordinary integer takes the `int` width, at which arrays store it (`array::ordinary_width`).
    pub(crate) fn number(n: &Number) -> Self {
        let fixed = n.fixed_width();
        let mut kind = Self { weak_integer: n.is_weak() && n.is_integral(), ..Self::stored(n.dtype(), fixed, n.is_weak()) };
        if fixed.is_none() && let Some(s) = Self::static_fixed(kind.dtype) { kind.dtype = Some(DType::Integer(s)); }
        kind
    }
    #[inline]
    pub(crate) fn value(v: &Value) -> Self {
        match v {
            Value::Number(n) => Self::number(n),
            _ => {
                let dtype = v.dtype();
                // Only integer storage can be fixed (`Storage::Fixed`), so other dtypes skip `fixed_integer`.
                let fixed = if let Some(DType::Integer(w)) = dtype { Self::fixed_integer(v, w) } else { None };
                Self { dtype, fixed, ..Self::default() }
            }
        }
    }
    pub(crate) fn item(v: &Value) -> Self { if matches!(v, Value::Array(_)) { Self::default() } else { Self::value(v) } }
    /// Whether the kind's type holds no negative numbers, as Booleans and unsigned bytes don't.
    pub(crate) fn unsigned(&self) -> bool { matches!(self.dtype, Some(DType::Boolean | DType::Integer(Width::U8))) }
    #[inline]
    pub(crate) fn stored(dtype: Option<DType>, fixed: Option<Width>, weak: bool) -> Self {
        let narrowest = match dtype { Some(DType::Integer(w)) => Some(w), _ => None };
        Self { dtype, fixed: fixed.or_else(|| if weak { None } else { Self::static_fixed(dtype) }), weak,
            weak_integer: weak && dtype.is_some_and(DType::is_integral), narrowest }
    }
    /// The fixed width of a strong ordinary integer of `dtype` under static kinds (`array::static_width`), which is a fixed integer of the width it computes at. `None` for other dtypes and under dynamic kinds. It stays out of line, the test of the setting included, so that `value` stays small enough to inline into the loops that read many kinds.
    #[inline(never)]
    fn static_fixed(dtype: Option<DType>) -> Option<Width> { match dtype { Some(DType::Integer(w)) => crate::array::static_width(w), _ => None } }
    /// The fixed width of `v`, an array of integers of width `w`: its own, or the width static kinds give it (`array::static_width`). It stays out of line, so that `value` stays small enough to inline into the loops that read many kinds.
    #[inline(never)]
    fn fixed_integer(v: &Value, w: Width) -> Option<Width> { v.fixed_width().or_else(|| crate::array::static_width(w)) }
    /// The fixed width that `parts` give a call or a join: the widest of their fixed widths, where every other part is a Boolean or a weak integer. `None` where no part is fixed, or a part is something else. An error where a weak integer doesn't fit the width.
    pub(crate) fn fixed(parts: impl IntoIterator<Item = Self>) -> Result<Option<Width>, &'static str> {
        let mut fixing = Fixing::default();
        for part in parts { fixing.add(part); }
        fixing.width()
    }
    /// The DOMAIN error that `fixed` gives where a weak integer among `values` doesn't fit the width of the fixed integers beside it, for a function that joins `values` into one array.
    pub(crate) fn fits(values: [&Value; 2]) -> Result<(), crate::Error> { Self::fixed(values.map(Self::value)).map(drop).map_err(crate::Error::domain) }
    /// `parts` joined into one kind under `widening`, as collected items join into one array's dtype. A fixed width that `fixed` finds wins. Where two parts' types don't join, the kind is unknown (`Kind::default()`). Each part is read once.
    pub(crate) fn join(parts: impl IntoIterator<Item = Self>, widening: Widening) -> Result<Self, &'static str> {
        let mut parts = parts.into_iter();
        let Some(first) = parts.next() else { return Ok(Self::default()) };
        // A lone part is its own join: its fixed width, if it has one, is the only width `Fixing` could find.
        let Some(second) = parts.next() else { return Ok(first) };
        let mut fixing = Fixing::default();
        fixing.add(first);
        let mut joined = Some(first);
        for part in [second].into_iter().chain(parts) {
            fixing.add(part);
            joined = joined.and_then(|kind| kind.with(part, widening));
            if joined.is_none() && fixing.other { return Ok(Self::default()); }
        }
        if let Some(width) = fixing.width()? { return Ok(Self::stored(Some(DType::Integer(width)), Some(width), false)); }
        Ok(joined.unwrap_or_default())
    }
    /// This kind joined with `part` under `widening`, one step of `join`. A strong float or complex type beside a weak type takes NEP 50's common type (`DType::common_weak`). `None` where the types don't join.
    fn with(self, part: Self, widening: Widening) -> Option<Self> {
        let (a, b) = (self.dtype?, part.dtype?);
        let mut dtype = a.join(b, widening)?;
        if self.weak != part.weak {
            let (strong, weak) = if self.weak { (b, a) } else { (a, b) };
            if strong.exact() == Some(false) { dtype = strong.common_weak(weak)?; }
        }
        Some(Self::stored(Some(dtype), None, self.weak && part.weak))
    }
    /// The width of the floats that this kind holds, or of the parts of its complex numbers.
    pub(crate) fn float_width(self) -> Option<FloatWidth> { crate::array::float_width_of(self.dtype?) }
    /// This kind with 16-bit floats as 32-bit floats, as a call that computes 16-bit floats in 32 bits reads them (`pervasive::halves`).
    fn single(self) -> Self {
        if self.dtype == Some(DType::Float(FloatWidth::F16)) { Self { dtype: Some(DType::Float(FloatWidth::F32)), ..self } } else { self }
    }
    /// The float width that a weak number of this kind takes beside a strong operand of `strong`'s dtype, as NEP 50 gives it, where that differs from its own. `None` for a strong kind, and where the weak number keeps its kind.
    pub(crate) fn weak_width(self, strong: DType) -> Option<FloatWidth> {
        if !self.weak { return None; }
        match strong.common_weak(self.dtype?)?.real() { DType::Float(width) if Some(width) != self.float_width() => Some(width), _ => None }
    }
    /// The width of the floats that operands of `kinds` compute in under `numeric`, for a function whose results are approximate, such as `⌹` or polar parts. Where integers wrap, under static kinds or with a fixed integer among them, integers and Booleans compute as floats of the widest width that their division gives (`Numeric::exact_width`). `None` where integers don't wrap and where an operand isn't integers or Booleans. Exact operands then stay exact.
    pub(crate) fn approximate(kinds: impl IntoIterator<Item = Self, IntoIter: Clone>, numeric: Numeric) -> Result<Option<FloatWidth>, &'static str> {
        let kinds = kinds.into_iter();
        let t = numeric.for_kinds(kinds.clone())?;
        let mut widest = None;
        for kind in kinds {
            let Some(dtype) = kind.dtype.filter(|d| t.wrapping() && d.is_integral()) else { return Ok(None) };
            widest = widest.max(Some(t.exact_width(dtype)));
        }
        Ok(widest)
    }
    /// This kind as a weak operand of `p` takes it beside an operand of kind `other`: the width that NEP 50 gives a weak number beside strong floats, as `Primitive::weaken_value` converts one. A strong kind, or one beside another weak kind, stays as it is. So does a weak integer beside fixed integers, for promotion to check that it fits (`Kind::fixed`), and an exact number that `p` keeps exact (`pervasive::keeps_exact`).
    pub(crate) fn beside(self, p: Primitive, other: Kind, numeric: Numeric) -> Self {
        if !self.weak || other.weak || other.fixed.is_some() && matches!(self.dtype, Some(DType::Integer(_))) { return self; }
        if self.dtype.is_some_and(DType::is_integral) && crate::pervasive::keeps_exact(p, numeric) { return self; }
        match (other.dtype.and_then(|strong| self.weak_width(strong)), self.dtype) {
            (Some(width), Some(DType::Complex(_))) => Self { dtype: Some(DType::Complex(width.into())), ..self },
            (Some(width), _) => Self { dtype: Some(DType::Float(width)), ..self }, (None, _) => self
        }
    }
}

/// The plan of one call of a pervasive primitive: the decisions that eager dispatch, fusion and typed programs share, made from the operands' kinds and the call's settings without reading any items.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pervasive {
    pub(crate) p: Primitive,
    pub(crate) dyadic: bool,
    /// The operands that are one real number, which pick some kernels, as `x*2` picks the square (`pervasive::kernel`).
    pub(crate) constants: [Option<f64>; 2],
    /// The settings that the call computes under: the session's, with the width that fixed operands give (`Numeric::fixed`), widened where 16-bit floats compute in 32 bits.
    pub(crate) numeric: Numeric,
    /// Where a 16-bit float takes part, whether float results round back to 16 bits. The call then computes on 32-bit copies of its 16-bit floats (`pervasive::halves`).
    pub(crate) halve: Option<bool>,
    /// The kind that the call computes in: the kind that the operands' kinds join to (`pervasive::computes_in`), or the one operand's kind. `None` where an operand isn't numbers.
    pub(crate) kind: Option<DType>,
    /// The operands' kinds, the left one first, as the call computes them: `None` for a monad's left operand, and for an operand that isn't numbers in compact storage.
    pub(crate) kinds: [Option<DType>; 2],
    /// Whether each operand is never negative or ¯0: by its type (`Kind::unsigned`), as a held constant that is at least +0, or by the fact that its description or pending array carries (`Operand::nonnegative`).
    pub(crate) nonnegative: [bool; 2],
    /// Whether both operands are one value, as `⊼x`'s `x×x` reads them.
    pub(crate) same: bool,
}

impl Pervasive {
    /// This plan with `same`, and with each operand's carried range fact (`Operand::nonnegative`) joined to the one that `pervasive` gave from its kind and constant.
    pub(crate) fn with_operands(self, same: bool, left: Option<&Operand>, right: &Operand) -> Self {
        Self { same, nonnegative: [self.nonnegative[0] || left.is_some_and(Operand::nonnegative), self.nonnegative[1] || right.nonnegative()], ..self }
    }
    /// The kind whose kernel table runs the call (`pervasive::computes`), or for a comparison of integers the kind that `compared` gives. `None` where no table on the way has `p`, as for `○` with no constant code.
    pub(crate) fn computes(&self) -> Option<DType> {
        if let Some(kind) = self.compared() { return Some(kind); }
        crate::pervasive::computes(self.p, self.dyadic, || self.constants, self.kind?, self.numeric)
    }
    /// The kind that a comparison of integers computes at, in eager dispatch and in block stages alike: the kind that its operands' kinds join to, with an operand that is one integer at the narrowest width that holds it (`narrowed`). Every width that holds both operands gives the same Booleans, so static kinds add no `int` width here, as `Numeric::kind` would. `None` for other calls, and where the operands don't join to integers.
    fn compared(&self) -> Option<DType> {
        if !self.dyadic || !matches!(self.p, Primitive::Compare(_)) { return None; }
        let narrowed = self.narrowed();
        let side = |k: usize| match narrowed { Some((s, n)) if s == k => Some(DType::Integer(Width::narrowest(n, n))), _ => self.kinds[k] };
        side(0)?.join(side(1)?, Widening::Arrays).filter(|k| matches!(k, DType::Integer(_)))
    }
    /// The side of a comparison's operand that is one integer, the left one first, and that integer: an integer, or a whole float that stands for one beside integers (`whole_float`).
    pub(crate) fn narrowed(&self) -> Option<(usize, i64)> {
        if !self.dyadic || !matches!(self.p, Primitive::Compare(_)) { return None; }
        self.whole_float().or_else(|| {
            (0..2).find_map(|k| match (self.kinds[k], self.constants[k]) { (Some(DType::Integer(_)), Some(c)) => Some((k, c as i64)), _ => None })
        })
    }
    /// The side of the operand that is a whole float standing for an integer, the left one first, and that integer, where the call is a comparison or residue between that float and integers that gives the same results on integers (`whole_float`). A residue's whole float is its divisor, the left operand.
    pub(crate) fn whole_float(&self) -> Option<(usize, i64)> {
        let residue = self.p == Primitive::Math(crate::number::Math::Magnitude);
        if !residue && !matches!(self.p, Primitive::Compare(_)) { return None; }
        let (true, None, Some(DType::Float(width))) = (self.dyadic, self.halve, self.kind) else { return None };
        (0..if residue { 1 } else { 2 }).find_map(|side| Some((side, whole_float(self.p, self.constants[side]?, width, self.kinds[1 - side]?, self.numeric)?)))
    }
    /// The kernel that the call runs first, from the table of kind `computes`, with what its operands' facts give (`rustymath::facts`). A float product of one value by itself, as `⊼x` is, is never negative or ¯0.
    pub(crate) fn kernel(&self) -> Option<crate::pervasive::Kernel> {
        let k = crate::pervasive::table_kernel(self.p, self.dyadic, self.computes()?, self.constants, self.nonnegative, self.numeric)?;
        let square = self.same && self.p == Primitive::Arithmetic(crate::number::Arithmetic::Times) && matches!(k.kind, DType::Float(_));
        Some(crate::pervasive::Kernel { nonnegative: k.nonnegative || square, ..k })
    }
    /// The kind of every result, where the operands' kinds settle it: the call's kernel gives each result the kind that `result` gives (`Kernel::total`). Under dynamic kinds, `+` on ordinary integers widens a result that doesn't fit, and `√` of a negative real gives a complex number, so neither settles. Programs, traces and block stages take only calls that settle, and folds and products only where each step settles (`folded`).
    pub(crate) fn settled(&self) -> Result<DType, &'static str> {
        if !self.kernel().is_some_and(|k| k.total) { return Err("a result whose type depends on values"); }
        self.result().ok_or("an operand that isn't numbers")
    }
    /// Whether a block stage can run the call as a step of pending work: its kernel never fails, and gives the type that its result has (`blocks::computes`), so that no item changes the call's type.
    #[inline(never)]
    pub(crate) fn fuses(&self) -> bool { self.result().is_some_and(|out| crate::program::blocks::computes(self, out).is_some()) }
    /// The kind of every result where the operands' kinds decide it, as they do under static kinds (`result_kind`).
    pub(crate) fn result(&self) -> Option<DType> { result_kind(self.p, self.dyadic, self.kinds, self.numeric, || self.computes()) }
    /// The operand that the call gives unchanged, as its kinds and constants show it, 0 for the left and 1 for the right: monadic `+` on a real operand, `⌊` and `⌈` on integers, and a dyadic call whose constant leaves the other operand unchanged, as the facts of its kernel's object say (`pervasive::kept_by`): `x+0`, `0+x`, `x-0`, `x×1`, `1×x`, `x÷1` and `x*1`. The result must have the operand's kind, so a float constant beside integers, which widens them, keeps nothing. Integers that don't wrap keep their kind through `+`, `⌊` and `⌈` whatever `result` says. A Boolean's floor is an integer. The exact `⌊` and `⌈` remove fixedness, as plain `vₓ` does. `x+0` gives ¯0 where `x` is ¯0, where the sum is 0: Jeremy accepted that difference on 30 October 2026. A constant of ¯0 counts as 0, so `x-¯0` keeps `x` too, with the same difference where `x` is ¯0. `pervasive::unchanged` adds the condition on shapes.
    pub(crate) fn kept(&self) -> Option<usize> {
        use crate::number::{Arithmetic::*, Math::*};
        use Primitive::{Arithmetic as A, Math as M};
        let integer = |k: usize| matches!(self.kinds[k], Some(DType::Integer(_)));
        let k = match (self.p, self.dyadic) {
            (A(Plus), false) if matches!(self.kinds[1], Some(DType::Float(_) | DType::Integer(_) | DType::Boolean)) => 1,
            (M(Floor | Ceiling), false) if integer(1) => 1,
            (_, true) if self.constants != [None; 2] => crate::pervasive::kept_by(self.p, self.constants)?,
            _ => return None,
        };
        (self.kinds[k].is_some() && self.result() == self.kinds[k] || !self.dyadic && integer(1) && !self.numeric.wrapping()).then_some(k)
    }
}

/// Whether the number `c` is never negative or ¯0, as a range fact needs (`rustymath::facts`).
pub(crate) fn nonnegative_number(c: f64) -> bool { c > 0.0 || c == 0.0 && c.is_sign_positive() }

/// The plan of pervasive primitive `p` on operands of kinds `left` and `right` under the session's settings `numeric`, where `constants` are the operands that are one real number. The message for a weak integer that doesn't fit the width of the fixed integers beside it (`Kind::fixed`).
pub(crate) fn pervasive(p: Primitive, left: Option<Kind>, right: Kind, constants: [Option<f64>; 2], numeric: Numeric) -> Result<Pervasive, &'static str> {
    let (numeric, halve, left, right) = prepared(p, left, right, numeric)?;
    let kinds = [left.and_then(|left| left.dtype), right.dtype];
    // An operand counts where its type holds no negative numbers, or where it's a held constant that is at least +0.
    let nonnegative = [left.is_some_and(|k| k.unsigned()) || constants[0].is_some_and(nonnegative_number), right.unsigned() || constants[1].is_some_and(nonnegative_number)];
    Ok(Pervasive { p, dyadic: left.is_some(), constants, numeric, halve, kinds, kind: computes_kind(p, left.is_some(), kinds, numeric), nonnegative, same: false })
}

/// The settings and operand kinds that a call of `p` computes with: `numeric` with the fixed width that the operands give (`Numeric::for_kinds`), or the message for a weak integer that doesn't fit that width. A function that takes no fixed width (`takes_fixed`) keeps `numeric`. Where 16-bit floats compute in 32 bits, the settings widen, the kinds read 32-bit floats, and the flag says whether float results round back to 16 bits.
fn prepared(p: Primitive, left: Option<Kind>, right: Kind, numeric: Numeric) -> Result<(Numeric, Option<bool>, Option<Kind>, Kind), &'static str> {
    let numeric = if takes_fixed(p) { numeric.for_kinds(left.into_iter().chain([right]))? } else { numeric };
    let (numeric, halve) = numeric.halving(left.into_iter().chain([right]).map(Kind::float_width));
    Ok((numeric, halve, left.map(Kind::single), right.single()))
}

/// The kind that a call of `p` on operands of kinds `kinds`, the left one first, computes in: the kind that a dyadic call's operands join to (`pervasive::computes_in`), or a monadic call's one operand's kind.
fn computes_kind(p: Primitive, dyadic: bool, kinds: [Option<DType>; 2], numeric: Numeric) -> Option<DType> {
    if dyadic { crate::pervasive::computes_in(p, kinds[0], kinds[1], numeric) } else { kinds[1] }
}

/// The kind of every result of `p` on operands of kinds `kinds`, the left one first, under `numeric`, where the kinds decide it, as they do under static kinds (section 2.4 of `meta/stage0-design.md`). A function that gives Booleans gives them. Beside a complex operand, results are complex, except for a magnitude, which is real. Beside a float operand, they're floats of the widest width. Integers and Booleans give the kind of the table that computes them, which `computes` gives (`pervasive::computes`): the floats of an approximate function, Booleans where the Boolean table has a kernel for the call, as it has for `∧`, `∨`, `⌈` and `⌊`, and otherwise integers of the integer width. Cis gives complex numbers where these rules give floats. `None` where an operand isn't numbers that compact storage holds, as an exact fraction isn't.
#[inline]
pub(crate) fn result_kind(p: Primitive, dyadic: bool, kinds: [Option<DType>; 2], numeric: Numeric, computes: impl FnOnce() -> Option<DType>) -> Option<DType> {
    use crate::number::Math::{self, Circle, Magnitude};
    if kinds[1].is_none() || dyadic && kinds[0].is_none() { return None; }
    let each = || kinds.into_iter().flatten();
    let monad = |m: Math| !dyadic && p == Primitive::Math(m);
    if p.facts(dyadic).booleans { return Some(DType::Boolean); }
    let width = each().filter_map(crate::array::float_width_of).max();
    if each().any(|k| matches!(k, DType::Complex(_))) {
        let w = width?;
        return Some(if monad(Magnitude) { DType::Float(w) } else { DType::Complex(w.into()) });
    }
    if width.is_none() && !each().all(DType::is_integral) { return None; }
    Some(match width.map_or_else(computes, |w| Some(DType::Float(w)))? {
        DType::Float(w) if monad(Circle) => DType::Complex(w.into()), k @ (DType::Float(_) | DType::Complex(_) | DType::Boolean) => k,
        _ => DType::Integer(numeric.integer_width())
    })
}

/// The integer that the whole float `c` stands for beside integers of kind `ints`, where `p`, a comparison or a residue, computes in floats of `width` under `numeric` and gives the same results computed on integers. `c` must be exactly whole and below rustymath's `tolerant::integer_limit` at the width and the call's tolerance (`tolerant::integer_of`). A residue, and so each step of a residue scan, also needs `c` and the greatest magnitude of an integer of `ints` below half that limit, where a residue of one by the other is exact. `None` otherwise, and the call computes in floats.
pub(crate) fn whole_float(p: Primitive, c: f64, width: FloatWidth, ints: DType, numeric: Numeric) -> Option<i64> {
    use rustymath::{Float as _, tolerant::{integer_limit, integer_of}};
    let most = match ints {
        DType::Boolean => 1.0, DType::Integer(w) => rustymath::with_width!(w, T => (T::MIN as f64).abs().max(T::MAX as f64)), _ => return None
    };
    let t = numeric.tolerance_at(width);
    let (n, limit) = rustymath::with_float_width!(width, F => (integer_of(t, F::narrow(c))?, integer_limit::<F>(t)));
    (p != Primitive::Math(crate::number::Math::Magnitude) || 2.0 * most.max(n.unsigned_abs() as f64) < limit).then_some(n)
}

/// The kind of every result of `p` on the numbers `x` and `y` under `numeric`, by the rule that arrays of their kinds follow (`result_kind`), without building the call's plan. Static kinds give each scalar call this kind. `numeric` must already hold the numbers' fixed width, and 16-bit floats must already compute in 32 bits, as `Primitive::number` arranges before it calls `statically`.
#[inline]
pub(crate) fn atom_result(p: Primitive, x: Option<&Number>, y: &Number, numeric: Numeric) -> Option<DType> {
    let (dyadic, kinds) = (x.is_some(), [x.and_then(Number::dtype), y.dtype()]);
    let computes =
        || crate::pervasive::computes(p, dyadic, || [x.and_then(|x| x.to_float().ok()), y.to_float().ok()], computes_kind(p, dyadic, kinds, numeric)?, numeric);
    result_kind(p, dyadic, kinds, numeric, computes)
}

/// `pervasive`'s plan for `p` on the values `left` and `right`: their kinds, with the ones that are one real number as constants, the range facts of pending arrays (`Operand::nonnegative`), and whether they're one value.
pub(crate) fn pervasive_on(p: Primitive, left: Option<&Value>, right: &Value, numeric: Numeric) -> Result<Pervasive, &'static str> {
    let plan = pervasive(p, left.map(Kind::value), Kind::value(right), crate::pervasive::constants(left, right), numeric)?;
    Ok(plan.with_operands(left.is_some_and(|l| std::ptr::eq(l, right)), left.map(Operand::Held).as_ref(), &Operand::Held(right)))
}

/// An operand of a planned call: a traced value, known only by its description, or a value that the program holds.
pub(crate) enum Operand<'a> { Traced(&'a Desc), Held(&'a Value) }

impl Operand<'_> {
    /// The operand's kind: a traced value's description's, or a held value's.
    fn kind(&self) -> Kind { match self { Operand::Traced(desc) => desc.kind, Operand::Held(value) => Kind::value(value) } }
    /// The operand's layout: a traced value's description's, or a held value's layout.
    fn layout(&self) -> Layout { match self { Operand::Traced(desc) => desc.layout.clone(), Operand::Held(value) => value.layout().clone() } }
    /// The operand's number, where it's a held real number. A kernel table can pick a kernel by it, as `x*2` picks the square. A traced number's value isn't known.
    fn constant(&self) -> Option<f64> { match self { Operand::Held(value) => value.as_real(), Operand::Traced(_) => None } }
    fn atom(&self) -> bool { match self { Operand::Traced(desc) => desc.atom, Operand::Held(value) => value.is_atom() } }
    fn weak(&self) -> bool {
        match self { Operand::Traced(desc) => desc.kind.weak, Operand::Held(value) => matches!(value, Value::Number(n) if n.is_weak()) }
    }
    /// The range fact that the operand carries beyond its kind and constant (`Pervasive::nonnegative`): a traced value's description's, or a held pending array's.
    fn nonnegative(&self) -> bool { match self { Operand::Traced(desc) => desc.nonnegative, Operand::Held(value) => value.pending().is_some_and(|p| p.nonnegative) } }
}

/// A planned pervasive call: its result's description, and its plan.
pub(crate) struct Mapped { pub(crate) desc: Desc, pub(crate) plan: Pervasive }

/// The plan of pervasive primitive `p` on `left` and `right` under `numeric`, where `same` says whether both are one value (`Pervasive::same`), with its result's description, or the reason that a program can't hold the call.
pub(crate) fn map(p: Primitive, left: Option<&Operand>, right: &Operand, same: bool, numeric: Numeric) -> Result<Mapped, &'static str> {
    if !p.scalar(left.is_some()) { return Err("a call that isn't pervasive"); }
    // A traced weak number beside fixed integers converts to their width only if its value fits, which planning can't see.
    let fixed = |o: &Operand| matches!(o, Operand::Traced(d) if d.kind.fixed.is_some()) || matches!(o, Operand::Held(v) if v.fixed_width().is_some());
    if left.is_some_and(|l| matches!(l, Operand::Traced(d) if d.kind.weak) && fixed(right) || matches!(right, Operand::Traced(d) if d.kind.weak) && fixed(l)) {
        return Err("a weak number argument beside fixed integers");
    }
    let (x, y) = (left.map(Operand::kind), right.kind());
    // A weak operand beside a strong one takes the width that NEP 50 gives it, as `Primitive::weakened` converts a weak number.
    let (x, y) = match x { Some(x) if x.weak != y.weak => (Some(x.beside(p, y, numeric)), y.beside(p, x, numeric)), _ => (x, y) };
    let plan = pervasive(p, x, y, [left.and_then(Operand::constant), right.constant()], numeric).map_err(|_| "a weak integer that doesn't fit")?;
    let plan = plan.with_operands(same, left, right);
    // 16-bit floats compute at 32 bits and round back (`pervasive::map`), which programs don't model yet.
    if plan.halve.is_some() { return Err("a 16-bit float"); }
    let dtype = plan.settled()?;
    let agreement = Agreement::new(&left.map_or_else(Layout::default, Operand::layout), &right.layout()).map_err(|_| "shapes that don't agree")?;
    let atom = right.atom() && left.is_none_or(Operand::atom);
    let weak = atom && right.weak() && left.is_none_or(Operand::weak);
    Ok(Mapped { desc: Desc { kind: computed(dtype, plan.numeric, weak), atom, layout: agreement.layout, nonnegative: plan.kernel().is_some_and(|k| k.nonnegative) }, plan })
}

/// The description of what `p` gives where it combines items that `items` describes into the base that `base` describes, as `Pairing::At` pairs them (`pervasive::combine_at`): the base's layout, and `p`'s result on one item of each joined with the base's kind under the rule for writes into an existing array (`Widening::of_writes`). An error for an atom base, where `p` doesn't plan on the two, or where the join would leave the base mixed, which programs can't hold.
pub(crate) fn at(p: Primitive, base: &Desc, items: &Desc, numeric: Numeric) -> Result<Desc, &'static str> {
    if base.atom { return Err("an accumulation into an atom"); }
    let item = |d: &Desc| Desc { atom: true, layout: Layout::default(), ..d.clone() };
    let combined = map(p, Some(&Operand::Traced(&item(base))), &Operand::Traced(&item(items)), false, numeric)?.desc.kind;
    let kind = Kind::join([base.kind, combined], Widening::of_writes())?;
    if kind.dtype.is_none() { return Err("a write that leaves its target mixed"); }
    Ok(Desc { kind, atom: false, layout: base.layout.clone(), nonnegative: false })
}

/// The plan of a fold or scan by `p`: `kind`, the settings that the items and any seed give together (`prepared`), and the kinds of each step's operands, the running result's first, as those settings compute them. A scan's first step takes the seed, so a scan computes in the kind that the seed's and the items' kinds join to. A fold applies its seed after it folds the items. Where 16-bit floats compute in 32 bits, `halve` says whether float results round back to 16 bits.
pub(crate) struct Fold { pub(crate) kind: FoldKind, p: Primitive, pub(crate) numeric: Numeric, pub(crate) halve: Option<bool>, kinds: [Option<DType>; 2] }

/// The plan of a fold or scan by `p` of items of kind `items`, with a seed of kind `seed`, under the session's settings `numeric`, or the message for a weak integer that doesn't fit the width of the fixed integers beside it. A weak seed takes the width that NEP 50 gives it beside the items, as `Primitive::weaken_value` converts one.
pub(crate) fn fold(p: Primitive, kind: FoldKind, seed: Option<Kind>, items: Kind, numeric: Numeric) -> Result<Fold, &'static str> {
    let (numeric, halve, seed, items) = prepared(p, seed.map(|s| s.beside(p, items, numeric)), items, numeric)?;
    let left = if kind.scan { seed.unwrap_or(items) } else { items };
    Ok(Fold { kind, p, numeric, halve, kinds: [left.dtype, items.dtype] })
}

impl Fold {
    /// The kind that each step computes in (`pervasive::computes_in`).
    pub(crate) fn computes_in(&self) -> Option<DType> { crate::pervasive::computes_in(self.p, self.kinds[0], self.kinds[1], self.numeric) }
    /// The float width that integer items convert to before the fold or scan, where integers wrap, under static kinds or at a fixed width, and `p`'s table gives floats for them, as for `⍟` and `÷`. Every item of the result then takes the call's kind, a lane's first included, and reading the items as floats first gives that through the float kernels. Where integers don't wrap, an item that no call computes, such as a scan's first, keeps its kind, so it's `None`.
    pub(crate) fn floated(&self) -> Option<FloatWidth> {
        if !self.numeric.wrapping() { return None; }
        let kind = self.computes_in().filter(|k| k.is_integral())?;
        match crate::pervasive::computes(self.p, true, || [None; 2], kind, self.numeric)? { DType::Float(width) => Some(width), _ => None }
    }
    /// The kind of the fold's result, as a lane of two or more items gives it without a seed: `p`'s running results where each step settles (`folded`), and otherwise `p`'s result on the first two items as their kinds decide it (`Pervasive::result`), before any widening that values cause under dynamic kinds.
    pub(crate) fn result(&self) -> Option<DType> {
        let [Some(left), Some(items)] = self.kinds else { return None };
        folded(self.p, left, items, self.numeric).ok().or_else(|| step(self.p, left, items, self.numeric).ok()?.result())
    }
    /// The description of the fold along `axis`, or along the fold's default axis, of a value that `right` describes, with an atom seed that `seed` describes, and the axis. Its kind is that of `p`'s running results (`folded`), joined with the first item's kind for a scan with no seed, and then `p`'s result beside a fold's seed. It's never negative or ¯0 where the items and any seed aren't and `p`'s steps keep that (`rustymath::facts`), since no lane here is empty.
    pub(crate) fn desc(&self, axis: Option<usize>, seed: Option<&Desc>, right: &Desc) -> Result<(Desc, usize), &'static str> {
        let rank = right.shape().len();
        if rank == 0 { return Err("a fold of an atom or a scalar"); }
        if seed.is_some_and(|s| !s.atom) { return Err("a seed that isn't an atom"); }
        let axis = axis.unwrap_or(self.kind.default_axis(rank));
        if right.shape()[axis] == 0 { return Err("an empty lane"); }
        let (numeric, [Some(left), Some(items)]) = (self.numeric, self.kinds) else { return Err("items that aren't numbers") };
        let running = folded(self.p, left, items, numeric)?;
        let dtype = match (self.kind.scan, seed) {
            (true, None) => numeric.kind(items.join(running, Widening::Arrays).ok_or("a scan whose kinds don't join")?),
            (false, Some(seed)) => pervasive(self.p, Some(computed(running, numeric, false)), seed.kind, [None; 2], numeric)?.settled()?, _ => running
        };
        let dtype = halved(dtype, self.halve);
        let layout = right.layout.folded(self.kind.scan, axis);
        let keeps = || step(self.p, left, items, numeric).is_ok_and(|s| Pervasive { nonnegative: [true; 2], ..s }.kernel().is_some_and(|k| k.nonnegative));
        let nonnegative = right.nonnegative && seed.is_none_or(|s| s.nonnegative) && keeps();
        Ok((Desc { kind: computed(dtype, numeric, false), atom: layout.shape().is_empty(), layout, nonnegative }, axis))
    }
}

/// Whether `p` computes at the fixed width of its fixed integer operands. Comparisons and the exact functions don't.
pub(crate) fn takes_fixed(p: Primitive) -> bool { !matches!(p, Primitive::Exact(_) | Primitive::Compare(_)) }

/// The kind of a result of `dtype` from a call under `numeric`, weak if `weak` is. An integer result is fixed where `numeric` has a fixed width.
fn computed(dtype: DType, numeric: Numeric, weak: bool) -> Kind {
    Kind::stored(Some(dtype), numeric.fixed.filter(|_| matches!(dtype, DType::Integer(_))), weak)
}

/// The plan of one step of a fold by `p` under `numeric`, on a running result of kind `left` and an item of kind `items`.
fn step(p: Primitive, left: DType, items: DType, numeric: Numeric) -> Result<Pervasive, &'static str> {
    pervasive(p, Some(computed(left, numeric, false)), computed(items, numeric, false), [None; 2], numeric)
}

/// The kind of a fold's running results under `numeric`: `p`'s result on `left`, its first step's left operand, and an item of kind `items`, where the kinds settle each step's result (`Pervasive::settled`). Each later step takes that result and an item, which must give it again. A fold of Booleans that rustymath runs as a truth fold, such as a count by `+` (`pervasive::truth_fold`), settles even where a step's kernel alone wouldn't, because its results stay within the number of items.
fn folded(p: Primitive, left: DType, items: DType, numeric: Numeric) -> Result<DType, &'static str> {
    let truth = left == DType::Boolean && items == DType::Boolean && crate::pervasive::truth_fold(p).is_some();
    let step = |left| {
        let plan = step(p, left, items, numeric)?;
        if truth { plan.result().ok_or("an operand that isn't numbers") } else { plan.settled() }
    };
    let first = step(left)?;
    if step(first)? == first { Ok(first) } else { Err("a fold whose kind changes along it") }
}

/// `dtype`, rounded back to 16 bits where `halve` says that a call's 32-bit float results do (`pervasive::halved`).
fn halved(dtype: DType, halve: Option<bool>) -> DType {
    if halve == Some(true) && dtype == DType::Float(FloatWidth::F32) { DType::Float(FloatWidth::F16) } else { dtype }
}

/// The plan of a product `f.g`: `g`'s plan on the operands, which the fold by `f` of `g`'s results follows (`Product::folds`), and the rustymath kernel that `f.g` names at `g`'s kind (`pervasive::known_product`).
pub(crate) struct Product { pub(crate) f: Primitive, pub(crate) g: Pervasive, pub(crate) kernel: Option<rustymath::linalg::Product> }

/// The plan of the product `f.g` on operands of kinds `left` and `right` under the session's settings `numeric`, or the message for a weak integer that doesn't fit the width of the fixed integers beside it.
pub(crate) fn product(f: Primitive, g: Primitive, left: Kind, right: Kind, numeric: Numeric) -> Result<Product, &'static str> {
    let g = pervasive(g, Some(left), right, [None; 2], numeric)?;
    Ok(Product { f, kernel: crate::pervasive::known_product(f, g.p, g.kind == Some(DType::Boolean)), g })
}

/// `product`'s plan for `f.g` on the values `left` and `right`, under the session's settings.
pub(crate) fn product_on(f: Primitive, g: Primitive, left: &Value, right: &Value) -> Result<Product, &'static str> {
    product(f, g, Kind::value(left), Kind::value(right), Numeric::current())
}

impl Product {
    /// The settings that the fold by `f` of `g`'s results computes under: `g`'s, without the fixed width where `f` takes none or `g` gives Booleans.
    pub(crate) fn folds(&self) -> Numeric {
        Numeric { fixed: self.g.numeric.fixed.filter(|_| takes_fixed(self.f) && !self.g.p.facts(true).booleans), ..self.g.numeric }
    }
    /// The description of the product of values that `left` and `right` describe. Its kind is that of `f`'s fold of `g`'s results (`folded`), where the kinds settle `g`'s results and every step of the fold (`Pervasive::settled`).
    pub(crate) fn desc(&self, left: &Desc, right: &Desc) -> Result<Desc, &'static str> {
        let (Some(&k), Some(&j)) = (left.shape().last(), right.shape().first()) else { return Err("a product with an atom or a scalar") };
        if k != j || k == 0 { return Err("lengths that don't agree, or are empty"); }
        let layout = left.layout.contracted(&right.layout);
        let g = self.g.settled()?;
        let folds = self.folds();
        let result = halved(folded(self.f, g, g, folds)?, self.g.halve);
        Ok(Desc { kind: computed(result, folds, false), atom: layout.shape().is_empty(), layout, nonnegative: false })
    }
    /// The description of `f.g⍤2` on the stacks of matrices that `left` and `right` describe: the frame that pairing their frames gives, as Rank pairs them (`Frames`), around the product of one matrix of each. Stacks with names or keys don't plan.
    pub(crate) fn stacked_desc(&self, left: &Desc, right: &Desc) -> Result<Desc, &'static str> {
        if !left.layout.plain() || !right.layout.plain() { return Err("a stack of matrices with names or keys"); }
        let frames = Frames::new(&[(&*left.layout, 2), (&*right.layout, 2)]).map_err(|_| "stacks whose frames don't pair")?;
        let matrix = |d: &Desc| Desc { layout: Layout::from(d.shape()[d.shape().len() - 2..].to_vec()), ..d.clone() };
        let product = self.desc(&matrix(left), &matrix(right))?;
        Ok(Desc { layout: Layout::from([frames.layout.shape(), product.shape()].concat()), atom: false, ..product })
    }
}

/// The description of a copy or a view of a value that `source` describes, laid out as `layout`: the source's type, fixedness and range fact, in an array of `layout`. A layout with a renderer gives an error, because programs hold none.
pub(crate) fn copy(source: &Desc, layout: &Layout) -> Result<Desc, &'static str> {
    if layout.renderer().is_some() { return Err("a layout with a renderer"); }
    Ok(Desc { kind: Kind::stored(source.kind.dtype, source.kind.fixed, false), atom: false, layout: layout.clone(), nonnegative: source.nonnegative })
}

/// The description of a copy of one item of a value that `source` describes, as `Value::at` reads one: an atom of the source's type and fixedness.
pub(crate) fn item(source: &Desc) -> Result<Desc, &'static str> { Ok(Desc { atom: true, ..copy(source, &Layout::default())? }) }

/// The description of the cotangent that a copy of overlapping windows adds back to its source (`linear::overlap_added`), from the windows' cotangent that `windows` describes, laid out as the source's `layout`: plain 32- or 64-bit floats, as a copy of them is.
pub(crate) fn overlap(windows: &Desc, layout: &Layout) -> Result<Desc, &'static str> {
    if !matches!(windows.kind.dtype, Some(DType::Float(FloatWidth::F32 | FloatWidth::F64))) || windows.kind.fixed.is_some() {
        return Err("a cotangent that isn't plain 32- or 64-bit floats");
    }
    copy(windows, layout)
}

/// The description of the mask that a program's picks give (`Step::Picks`): Booleans in the layout of `of`, whose real numbers `⌈` or `⌊` picks from along axis `axis`.
pub(crate) fn picks(of: &Desc, axis: usize) -> Result<Desc, &'static str> {
    if axis >= of.shape().len() { return Err("an axis that the items lack"); }
    if !matches!(of.kind.dtype, Some(DType::Boolean | DType::Integer(_) | DType::Float(_))) { return Err("picks from items that aren't real numbers"); }
    Ok(Desc { kind: Kind::stored(Some(DType::Boolean), None, false), ..copy(of, &of.layout)? })
}

/// A planned masked select: its result's description, and the map through which each case reads from the result's positions. The mask reads them in order. A weak atom case takes the result's type, which its executor converts it to.
pub(crate) struct Select { pub(crate) desc: Desc, pub(crate) yes: Mapping, pub(crate) no: Mapping }

/// The plan of a select that takes `yes`'s items where `mask` is true and `no`'s where it's false, the masked lowering of `⍚` and `@[p]`. The mask is an array of Booleans. Each case is an atom, which serves every position, or an array of the mask's layout. The cases' kinds join to the result's type (`Kind::join`): a weak atom case takes the other case's kind, and two strong cases must have one kind.
pub(crate) fn select(mask: &Operand, yes: &Operand, no: &Operand) -> Result<Select, &'static str> {
    if mask.atom() || mask.kind().dtype != Some(DType::Boolean) { return Err("a mask that isn't an array of Booleans"); }
    let layout = mask.layout();
    let map = |case: &Operand| {
        if case.atom() { Ok(Mapping::Single) } else if *case.layout() == *layout { Ok(Mapping::Linear(1)) } else { Err("a case whose layout isn't the mask's") }
    };
    let (yes_map, no_map) = (map(yes)?, map(no)?);
    let cases = [yes.kind(), no.kind()];
    let kind = Kind::join(cases, Widening::Arrays)?;
    let dtype = kind.dtype.filter(|&d| d != DType::Character).ok_or("cases that aren't numbers")?;
    if cases.iter().any(|c| !c.weak && c.dtype != Some(dtype)) { return Err("cases of different kinds"); }
    Ok(Select { desc: Desc { kind: Kind::stored(Some(dtype), kind.fixed, false), atom: false, layout, nonnegative: false }, yes: yes_map, no: no_map })
}

/// The description of `source` with every number converted to `to`, as `Value::converted` converts it for width marks, `•float` and fixed marks: floats of a width, where complex numbers keep complex parts of that width, or fixed integers of a width. The result is strong.
pub(crate) fn convert(source: &Desc, to: DType) -> Result<Desc, &'static str> {
    let (dtype, fixed) = match (source.dtype(), to) {
        (DType::Character, _) => return Err("characters"), (DType::Complex(_), DType::Float(width)) => (DType::Complex(width.into()), None),
        (DType::Complex(_), _) => return Err("complex numbers that convert to integers"), (_, DType::Float(_)) => (to, None),
        (_, DType::Integer(width)) => (to, Some(width)), _ => return Err("a conversion to a type that no mark gives")
    };
    Ok(Desc { kind: Kind::stored(Some(dtype), fixed, false), nonnegative: false, ..source.clone() })
}
