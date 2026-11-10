//! Operation planning: an operation's result type and layout, worked out from its operands' kinds and the numeric settings without reading any items. A pervasive call's plan (`Pervasive`) makes the decisions that eager dispatch, fusion and typed programs (`program`) share: the call's settings, the kind it computes in, its rustymath formula and, under static kinds, each result's kind. It works out the settings in `prepared` and the rest through `pervasive::computes_in`, `computes` and `kernel`. `map` takes a call's layout from its operands' `Agreement`. A fold's plan is its function's plan on two items (`Fold`), and a product's is `g`'s plan, which `f`'s fold of its results follows (`Product`).

use crate::{Number, Value, agreement::{Agree, Agreement}, array::{Layout, Promotion, Widening}, number::{Halving, Numeric}, primitive::{FoldKind, Primitive}};
use rustymath::{agreement::{Frames, Mapping}, buffer::{DType, FloatWidth, Width}, domain::Range, operation::Kernel};

/// An operand's kind: its type, fixed width and weakness, without its items or shape. A call's promotion reads its operands' kinds, and collected items join theirs into one array's dtype (`Kind::join`), before any weak integer is checked for fit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Kind {
    pub(crate) dtype: Option<DType>,
    pub(crate) fixed: Option<Width>,
    pub(crate) weak: bool,
    /// The sign of a bound, the bare `∞` or `¯∞` (`Number::order_class`): 1 or ¯1, and 0 for every other kind.
    bound: i8,
    /// The actual minimum width needed for weak integer promotion, not a computed result's provisional dtype.
    minimum: Minimum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Minimum { #[default] Unneeded, Known(Width), Unknown }

/// The fixed width that parts give, built one part at a time, for `Kind::fixed` and `Kind::join`. `fixed` joins the parts' declared fixed widths, `required` the narrowest widths of their weak machine integers, and `ordinary` the widths that static kinds give their strong ordinary integers (`array::static_width`), which join a declared width but never make one. `other` marks a part that is none of these: a weak integer in the unbounded domain (`ᵣ`) has no narrowest width, so it counts as other.
#[derive(Default)]
struct Fixing { fixed: Option<Width>, required: Option<Width>, ordinary: Option<Width>, other: bool }
impl Fixing {
    #[inline]
    fn add(&mut self, part: Kind) {
        let join = |into: &mut Option<Width>, w: Width| *into = Some(into.map_or(w, |v| v.join(w)));
        if self.other { return; }
        let part = part.for_promotion();
        if let Some(w) = part.fixed { join(&mut self.fixed, w); }
        else if part.dtype == Some(DType::Boolean) { }
        else if let Minimum::Known(w) = part.minimum {
            join(&mut self.required, w);
        } else if let Some(DType::Integer(w)) = part.dtype && let Some(w) = crate::array::static_width(w) { join(&mut self.ordinary, w); }
        else { self.other = true; }
    }
    fn width(self) -> Option<Width> {
        self.fixed.filter(|_| !self.other).map(|width| [self.required, self.ordinary].into_iter().flatten().fold(width, Width::join))
    }
}
impl Kind {
    /// The kind of `n`. Under static kinds an ordinary integer takes the `int` width, at which arrays store it (`array::ordinary_width`).
    pub(crate) fn number(n: &Number) -> Self {
        let fixed = n.fixed_width();
        let bound = n.order_class().filter(|&c| c != 0).map_or(0, |c| c as i8);
        let mut kind = Self { bound, ..Self::stored(n.dtype(), fixed, n.is_weak()) };
        if fixed.is_none() && let Some(DType::Integer(w)) = kind.dtype && let Some(s) = crate::array::static_width(w) { kind.dtype = Some(DType::Integer(s)); }
        kind
    }
    /// The logical kind: a recording's declared kind, or a carrier's primal/cell kind. Physical storage queries (`Value::dtype`) deliberately report no dtype for carriers.
    #[inline]
    pub(crate) fn value(v: &Value) -> Self {
        if let Some(carrier) = v.carrier() { return carrier.traced().map_or_else(|| Self::value(carrier.template()), |t| t.desc().kind); }
        match v { Value::Number(n) => Self::number(n), _ => Self { dtype: v.dtype(), fixed: v.fixed_width(), ..Self::default() } }
    }
    /// The ordered domain of an operand of this kind (`Number::order_class`): 0 for exact numbers, the sign of a bound (`∞`, `¯∞`), and `None` for other kinds.
    pub(crate) fn order_class(&self) -> Option<i32> {
        if self.bound != 0 { Some(self.bound.into()) } else { self.dtype.is_some_and(DType::is_integral).then_some(0) }
    }
    #[inline]
    pub(crate) fn stored(dtype: Option<DType>, fixed: Option<Width>, weak: bool) -> Self {
        let minimum = match (dtype, fixed, weak) { (Some(DType::Integer(w)), None, true) => Minimum::Known(w), _ => Minimum::Unneeded };
        Self { dtype, fixed, weak, minimum, bound: 0 }
    }

    /// Kind facts usable by a promotion that needs a weak integer's actual minimum width.
    fn for_promotion(self) -> Self { if self.minimum == Minimum::Unknown { Self::default() } else { self } }
    /// The fixed width that `parts` give a call or a join: the widest of their fixed widths, where every other part is a Boolean or a weak machine integer. `None` where no part is fixed, or a part is something else, such as a number in the unbounded domain (`ᵣ`), which mixing with machine integers keeps. A weak integer that doesn't fit widens the width to the narrowest that holds it (weak-literal minimum-width promotion), as `1ₓ₁+256ₓ` computes at 16 bits, and a comparison such as `v<300ₓ` of bytes compares there.
    pub(crate) fn fixed(parts: impl IntoIterator<Item = Self>) -> Option<Width> {
        let mut fixing = Fixing::default();
        for part in parts { fixing.add(part); }
        fixing.width()
    }
    /// `parts` joined into one kind under `widening`, as collected items join into one array's dtype. A fixed width that `fixed` finds wins. Where two parts' types don't join, the kind is unknown (`Kind::default()`). Each part is read once.
    pub(crate) fn join(parts: impl IntoIterator<Item = Self>, widening: Widening) -> Self {
        let mut parts = parts.into_iter().map(Self::for_promotion);
        let Some(first) = parts.next() else { return Self::default() };
        // A lone part is its own join: its fixed width, if it has one, is the only width `Fixing` could find.
        let Some(second) = parts.next() else { return first };
        let mut fixing = Fixing::default();
        fixing.add(first);
        let mut joined = Some(first);
        for part in [second].into_iter().chain(parts) {
            fixing.add(part);
            joined = joined.and_then(|kind| kind.with(part, widening));
            if joined.is_none() && fixing.other { return Self::default(); }
        }
        if let Some(width) = fixing.width() { return Self::stored(Some(DType::Integer(width)), Some(width), false); }
        joined.unwrap_or_default()
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
    /// The float width that a weak number of this kind takes beside a strong operand of `strong`'s dtype, as NEP 50 gives it, where that differs from its own. `None` for a strong kind, and where the weak number keeps its kind.
    pub(crate) fn weak_width(self, strong: DType) -> Option<FloatWidth> {
        if !self.weak { return None; }
        match strong.common_weak(self.dtype?)?.real() { DType::Float(width) if Some(width) != self.float_width() => Some(width), _ => None }
    }
    /// The width of the floats that operands of `kinds` compute in under `numeric`, for a function whose results are approximate, such as `⌹`: machine integers and Booleans compute as floats of the widest width that their division gives (`Numeric::exact_width`). `None` where an operand isn't machine integers or Booleans. Numbers in the unbounded domain (`ᵣ`) then stay exact.
    pub(crate) fn approximate(kinds: impl IntoIterator<Item = Self, IntoIter: Clone>, numeric: Numeric) -> Option<FloatWidth> {
        let kinds = kinds.into_iter().map(Self::for_promotion);
        let t = numeric.for_kinds(kinds.clone());
        let mut widest = None;
        for kind in kinds {
            let Some(dtype) = kind.dtype.filter(|d| d.is_integral()) else { return None };
            widest = widest.max(Some(t.exact_width(dtype)));
        }
        widest
    }
    /// The kind that a weak operand of this kind takes beside an operand of kind `other`, as `primitive::weakened_beside` converts a weak number: the width that NEP 50 gives it beside strong floats, or the width of fixed integers beside it where a weak integer fits them (`Kind::fixed`). A strong kind, or one beside another weak kind, stays as it is. So does a weak integer that doesn't fit, which promotion widens the fixed width to hold.
    pub(crate) fn beside(mut self, other: Kind) -> Self {
        if other.fixed.is_some() { self = self.for_promotion(); }
        if !self.weak || other.weak { return self; }
        if let Some(w) = other.fixed && matches!(self.dtype, Some(DType::Integer(_))) {
            return if Self::fixed([self, other]) == Some(w) { Self::stored(Some(DType::Integer(w)), Some(w), false) } else { self };
        }
        match (other.dtype.and_then(|strong| self.weak_width(strong)), self.dtype) {
            (Some(width), Some(DType::Complex(_))) => Self { dtype: Some(DType::Complex(width.into())), ..self },
            (Some(width), _) => Self { dtype: Some(DType::Float(width)), ..self }, (None, _) => self
        }
    }
}

/// What a program knows of a value without its items: its kind, atom/array category, layout and coarsened item range. Strong atoms use their logical kind; weak integers retain the minimum-width class needed for fixed promotion, but no exact-value key. Arrays retain their recorded range, so they share a key where their items have one sign and power-of-two magnitude.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Desc { pub(crate) kind: Kind, pub(crate) atom: bool, pub(crate) layout: Layout, pub(crate) range: Range }

impl Desc {
    /// The description of `value`, or `None` for a value that a program can't hold (`describes`).
    pub(crate) fn of(value: &Value) -> Option<Self> {
        let kind = Self::kind_of(value)?;
        Some(Self { kind, atom: value.is_atom(), layout: value.layout().clone(), range: kind.dtype.map_or(Range::ANY, Range::of_dtype).meet(value.range()) })
    }

    pub(crate) fn held(value: &Value) -> Self {
        Self::of(value).unwrap_or_else(|| Self { kind: Kind::value(value), atom: value.is_atom(), layout: value.layout().clone(), range: Range::ANY })
    }
    /// What a plan reads of `value`, a held value that this describes: a plain atom itself, whose number can pick a kernel as a constant, and otherwise this description, whose facts stay fixed while the value's items are computed or carry a level. The trace plans each step with it (`trace::Arg::operand`), and verification plans again with it (`verify::planned`).
    pub(crate) fn operand<'a>(&'a self, value: &'a Value) -> Operand<'a> {
        if value.is_atom() && !value.carried() { Operand::Held(value) } else { Operand::Traced(self) }
    }
    /// Whether a program can hold `value`: numbers in compact storage that carry no running level's data and whose layout has no renderer.
    pub(crate) fn describes(value: &Value) -> bool { Self::kind_of(value).is_some() }
    /// The kind of `value`'s description, or `None` where a program can't hold it (`describes`).
    fn kind_of(value: &Value) -> Option<Kind> {
        if value.carrier().is_some() || value.renderer().is_some() { return None; }
        let kind = Kind::value(value);
        let dtype = kind.dtype.filter(|&d| d != DType::Character)?;
        let mut described = Kind::stored(Some(dtype), kind.fixed, kind.weak);
        if kind.weak { described.minimum = kind.minimum; described.bound = kind.bound; }
        Some(described)
    }
    /// The stored item type. Pending structural copies may hold characters; traced numeric operations use `describes`.
    pub(crate) fn dtype(&self) -> DType { self.kind.dtype.expect("a description has a type") }
    /// The shape of the described value.
    pub(crate) fn shape(&self) -> &[usize] { self.layout.shape() }
    /// Runtime result conformance: type, fixed width and atom/array category. Ordinary integer atoms may use any narrower physical width that fits the plan (`Number::dtype`), as a one-item count does. Weakness does not change the kind already chosen for this result; input capture guards compare canonical kinds separately (`Entry::reread`). Array dtypes match exactly (`Value::stored_at`). Carriers expose their logical/template kinds, including tape coefficients carrying outer duals. Layouts are rebound separately.
    pub(crate) fn matches(&self, value: &Value) -> bool {
        let kind = Kind::value(value);
        let dtype = match (kind.dtype, self.kind.dtype) {
            (Some(DType::Integer(w)), Some(DType::Integer(planned))) if value.is_atom() && kind.fixed.is_none() => w.join(planned) == planned,
            (dtype, planned) => dtype == planned,
        };
        value.is_atom() == self.atom && dtype && kind.fixed == self.kind.fixed
    }
    /// A zero of this description's type, weak where the description is: the template of a traced value.
    pub(crate) fn zero(&self) -> Value {
        let n = match self.dtype() {
            DType::Boolean => Number::from_bool(false), DType::Integer(w) if self.kind.fixed.is_some() => Number::fixed(0, w),
            DType::Integer(_) => Number::from_integer(0), DType::Float(w) => Number::float(0.0, w),
            DType::Complex(w) => Number::complex(num_complex::Complex64::new(0.0, 0.0), w.into()),
            DType::Character => return Value::Character(' '),
        };
        Value::Number(n.marked(self.kind.weak))
    }
}

/// The plan of one call of a pervasive primitive: the decisions that eager dispatch, fusion and typed programs share, made from the operands' kinds and the call's settings without reading any items.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pervasive {
    pub(crate) p: Primitive,
    pub(crate) dyadic: bool,
    /// The operands that are one real number: `○`'s code picks its formula, a constant narrows its operand's range (`ranges`), and `kept` reads which operand a constant leaves unchanged.
    pub(crate) constants: [Option<f64>; 2],
    /// The settings that the call computes under: the session's, with the width that fixed operands give (`Numeric::fixed`), widened where 16-bit floats compute in 32 bits.
    pub(crate) numeric: Numeric,
    /// How the call reads and rounds 16-bit floats (`Numeric::halving`).
    pub(crate) halve: Halving,
    /// The kind that the call computes in: the kind that the operands' kinds join to (`pervasive::computes_in`), or the one operand's kind. `None` where an operand isn't numbers.
    pub(crate) kind: Option<DType>,
    /// The operands' kinds, the left one first, as the call computes them: `None` for a monad's left operand, and for an operand that isn't numbers in compact storage.
    pub(crate) kinds: [Option<DType>; 2],
    /// Each operand's ordered domain, the left one first (`Kind::order_class`): 0 for exact numbers, the sign of a bound (`∞`, `¯∞`), and `None` for other operands. An order-only call between different classes picks without promotion (`picking`).
    pub(crate) classes: [Option<i32>; 2],
    /// The kind whose kernel table runs the call (`computes`), which `pervasive` works out once from the other facts. A comparison of plans that leaves out a constant leaves this out too.
    pub(crate) at: Option<DType>,
    /// The bound on each operand's items: its type's values (`Range::of_dtype`) within the range that its description or held value carries (`Operand::range`). The call's kernel is checked at these bounds, and at a held constant's own value (`ranges`).
    pub(crate) bounds: [Range; 2],
    /// Whether both operands are one value, as `⊼x`'s `x×x` reads them.
    pub(crate) same: bool,
    /// The call's kernel (`kernel`).
    pub(crate) resolved: Resolved,
}

/// A plan's kernel: the one that working out `computes` settled on (`pervasive::computes`), or else resolved from the plan's other facts on its first read (`Pervasive::kernel`). Plans whose other facts are equal have the same kernel, so it never tells two plans apart.
#[derive(Clone, Debug, Default)]
pub(crate) struct Resolved(std::sync::OnceLock<Option<Kernel>>);
impl PartialEq for Resolved { fn eq(&self, _: &Self) -> bool { true } }

impl Pervasive {
    /// The kind whose kernel table runs the call (`pervasive::computes`), or for a comparison of integers the kind that `compared` gives, as `pervasive` works it out once when it builds the plan (`computing`). `None` where no table on the way has `p`, as for `○` with no constant code.
    pub(crate) fn computes(&self) -> Option<DType> { self.at }
    /// `computes`, worked out from the plan's other facts, and the kernel that `pervasive::computes` settled on there, if any.
    fn computing(&self) -> Option<(DType, Option<Kernel>)> {
        if let Some(kind) = self.compared() { return Some((kind, None)); }
        let kind = self.kind?;
        self.with_ranges(|ranges| crate::pervasive::computes(self.p, self.dyadic, self.constants, kind, ranges, self.numeric))
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
        let (true, Halving::Off, Some(DType::Float(width))) = (self.dyadic, self.halve, self.kind) else { return None };
        (0..if residue { 1 } else { 2 }).find_map(|side| Some((side, whole_float(self.p, self.constants[side]?, width, self.kinds[1 - side]?, self.numeric)?)))
    }
    /// The formula that the call runs first, at the kind that `computes` gives, on operands stored as `kind`, whose ranges bound its failures (`pervasive::table_kernel`), kept from working out `computes`, or resolved on its first read (`resolving`). Where the operands' order classes differ, an order-only formula picks without promotion (`Kernel::ordered`).
    pub(crate) fn kernel(&self) -> Option<&Kernel> { self.resolved.0.get_or_init(|| self.resolving()).as_ref() }
    /// `kernel`, resolved from the plan's other facts.
    fn resolving(&self) -> Option<Kernel> {
        Some(self.ordered(crate::pervasive::table_kernel(self.p, self.dyadic, self.computes()?, self.kind?, self.constants, self.numeric)?))
    }
    /// `kernel` picking without promotion where the operands' order classes differ (`Kernel::ordered`).
    fn ordered(&self, kernel: Kernel) -> Kernel { match self.kinds { [Some(x), Some(y)] => kernel.ordered(&self.classes, &[x, y]), _ => kernel } }
    /// The range of each argument that the kernel takes, the left one first: a held constant's own value, except ¯0, and otherwise the operand's bound (`bounds`), as `Kernel::may_raise_on` and `Kernel::range_on` read them, so `x*2` settles on floats. Both for a dyadic call, and the right one alone for a monad and for a function whose left argument is a code, which the kernel doesn't take.
    fn with_ranges<R>(&self, f: impl FnOnce(&[Range]) -> R) -> R {
        let ranges = [0, 1].map(|k| match self.constants[k] { Some(c) if c != 0.0 || c.is_sign_positive() => Range::of(Some(c)), _ => self.bounds[k] });
        f(&ranges[usize::from(!self.dyadic || self.p.info().dyad.code)..])
    }
    /// The result bound from the kernel at its operands' ranges (`Kernel::range_on`), or the square of one range for a product of one value by itself.
    pub(crate) fn result_range(&self) -> Range { self.kernel().map_or(Range::ANY, |k| self.with_ranges(|ranges| k.range_on(ranges, self.same))) }
    /// Whether the call's kernel can't fail an item on these operands, as rustymath's validity facts say at the operands' ranges (`Kernel::may_raise_on`, at `ranges`). A deferred call and an in-place write need it.
    pub(crate) fn never_fails(&self) -> bool { self.kernel().is_some_and(|k| self.with_ranges(|ranges| !k.may_raise_on(ranges))) }
    /// The kind of every result, where the operands' kinds settle it: the call's kernel never fails on its operands (`never_fails`), and gives each result the kind that `result` gives. Under dynamic kinds, `√` of a negative real gives a complex number, so it settles only where its operand's range keeps it real. Programs, traces and block stages take only calls that settle, and folds and products only where each step settles (`folded`).
    pub(crate) fn settled(&self) -> Result<DType, &'static str> {
        if !self.never_fails() { return Err("a result whose type depends on values"); }
        // Under dynamic kinds, `⌊`, `⌈` and `×` give integers from floats only where each result is a whole number that a width holds (`pervasive::integral`).
        if !self.numeric.is_static() && self.computes().is_some_and(|at| crate::pervasive::integral(self.p, self.dyadic, at, self.numeric)) {
            return Err("a result whose type depends on values");
        }
        self.result().ok_or("an operand that isn't numbers")
    }
    /// Whether a block stage can run the call as a step of pending work: its kernel never fails, and gives the type that its result has (`blocks::computes`), so that no item changes the call's type.
    #[inline(never)]
    pub(crate) fn fuses(&self) -> bool { self.result().is_some_and(|out| crate::program::blocks::computes(self, out).is_some()) }
    /// The kind of every result where the operands' kinds decide it, as they do under static kinds: the kind of the operand or truth value that an order-only call picks (`picking`), and otherwise the kind that the call's kernel gives (`Kernel::result_dtype`), with integers at the integer width (`Numeric::integer_width`), as are the whole numbers that `⌊`, `⌈` and `×` give from floats under dynamic kinds (`pervasive::integral`). `None` where an operand isn't numbers that compact storage holds, as an exact fraction isn't, and where no kernel runs the call, as for `○` with no constant code.
    pub(crate) fn result(&self) -> Option<DType> {
        if let Some(k) = self.picking() { return Some(k.result_dtype()); }
        if self.kinds[1].is_none() || self.dyadic && self.kinds[0].is_none() { return None; }
        if crate::pervasive::booleans(self.p, self.dyadic) { return Some(DType::Boolean); }
        if self.kinds.into_iter().flatten().any(|k| !k.is_real() && !matches!(k, DType::Complex(_))) { return None; }
        let integers = DType::Integer(self.numeric.integer_width());
        if crate::pervasive::integral(self.p, self.dyadic, self.computes()?, self.numeric) { return Some(integers); }
        Some(match self.kernel()?.result_dtype() { DType::Integer(_) if self.numeric.wrapping() => integers, k => k })
    }
    /// The settled result under its layout, atom and weak-number flags.
    pub(crate) fn result_desc(&self, layout: Layout, atom: bool, weak: bool) -> Result<Desc, &'static str> {
        Ok(Desc { kind: computed(self.settled()?, self.numeric, weak), atom, layout, range: self.result_range() })
    }
    /// The operand that the call gives unchanged, as its kinds and constants show it, 0 for the left and 1 for the right: the operand that an order-only call picks by its order class (`picking`), monadic `+` on a real operand, `⌊` and `⌈` on integers, and a dyadic call whose constant leaves the other operand unchanged, as its kernel's formula says (`Kernel::kept`): `x+0`, `0+x`, `x-0`, `x×1`, `1×x`, `x÷1` and `x*1`. The result must have the operand's kind, so a float constant beside integers, which widens them, keeps nothing. Integers that don't wrap keep their kind through `+`, `⌊` and `⌈` whatever `result` says. A Boolean's floor is an integer. The exact `⌊` and `⌈` remove fixedness, as plain `vₓ` does. `x+0` gives ¯0 where `x` is ¯0, where the sum is 0: Jeremy accepted that difference on 30 October 2026. A constant of ¯0 counts as 0, so `x-¯0` keeps `x` too, with the same difference where `x` is ¯0. `pervasive::unchanged` adds the condition on shapes.
    pub(crate) fn kept(&self) -> Option<usize> {
        if let Some(k) = self.picking() { return k.selected_input(); }
        use crate::number::{Arithmetic::*, Math::*};
        use Primitive::{Arithmetic as A, Math as M};
        let integer = |k: usize| matches!(self.kinds[k], Some(DType::Integer(_)));
        let k = match (self.p, self.dyadic) {
            (A(Plus), false) if matches!(self.kinds[1], Some(DType::Float(_) | DType::Integer(_) | DType::Boolean)) => 1,
            (M(Floor | Ceiling), false) if integer(1) => 1, (_, true) if self.constants != [None; 2] => self.kernel()?.kept(&self.constants)?, _ => return None
        };
        (self.kinds[k].is_some() && self.result() == self.kinds[k] || !self.dyadic && integer(1) && !self.numeric.wrapping()).then_some(k)
    }
    /// The kernel where the operands' order classes differ and `p` is order-only, as for `⌊`, `⌈` and comparisons: it then gives one operand, or one truth value, without promotion (`Kernel::ordered`), as `3ₓ⌊∞` gives `3ₓ`.
    pub(crate) fn picking(&self) -> Option<&Kernel> {
        if !matches!(self.classes, [Some(x), Some(y)] if x != y) { return None; }
        self.kernel().filter(|k| matches!(k.input_policy(), Some(rustymath::characters::Policy::Kinds(_))))
    }
}

/// `r` widened to the sign and power-of-two magnitude that selects the same integer storage width: [0, 2ᵏ-1], [-2ᵏ, 0] or [-2ᵏ, 2ᵏ-1]. Storage and scan allocation use this bucket; mathematical range propagation keeps the formula's bounds.
pub(crate) fn coarse(r: Range) -> Range {
    if r.is_empty() { return Range::EMPTY; }
    let mag = [r.hi, -r.lo - 1.0].into_iter().filter(|v| v.is_finite()).fold(0.0, f64::max);
    let m = (mag.ceil() as u64).saturating_add(1).checked_next_power_of_two().map_or(f64::INFINITY, |m| m as f64);
    Range { lo: if r.lo >= 0.0 { 0.0 }
    else if r.lo.is_finite() { -m }
    else { r.lo }, hi: if r.hi <= 0.0 { 0.0 }
    else if r.hi.is_finite() { m - 1.0 }
    else { r.hi } }
}

/// The plan of pervasive primitive `p` on operands of kinds `left` and `right` under the session's settings `numeric`, where `constants` are the operands that are one real number and `bounds` bound the operands' items.
pub(crate) fn pervasive(p: Primitive, left: Option<Kind>, right: Kind, constants: [Option<f64>; 2], bounds: [Range; 2], numeric: Numeric) -> Pervasive {
    let (numeric, halve, left, right) = prepared(p, left, right, numeric);
    let kinds = [left.and_then(|left| left.dtype), right.dtype];
    let bounds = [0, 1].map(|k| kinds[k].map_or(Range::ANY, Range::of_dtype).meet(bounds[k]));
    let classes = [left.and_then(|k| k.order_class()), right.order_class()];
    let mut plan = Pervasive { p, dyadic: left.is_some(), constants, numeric, halve, kinds, classes, kind: computes_kind(p, left.is_some(), kinds, numeric),
        at: None, resolved: Resolved::default(), bounds, same: false };
    let (at, found) = plan.computing().map_or((None, None), |(at, kernel)| (Some(at), kernel));
    plan.at = at;
    if let Some(kernel) = found { plan.resolved = Resolved(std::sync::OnceLock::from(Some(plan.ordered(kernel)))); }
    plan
}

/// The settings and operand kinds that a call of `p` computes with: `numeric` with the fixed width that the operands give (`Numeric::for_kinds`). A function that takes no fixed width (`takes_fixed`) keeps `numeric`. Where 16-bit floats compute in 32 bits, the settings widen, the kinds read 32-bit floats, and the flag says whether float results round back to 16 bits.
fn prepared(p: Primitive, left: Option<Kind>, right: Kind, numeric: Numeric) -> (Numeric, Halving, Option<Kind>, Kind) {
    let (left, right) = match left { Some(left) => (Some(left.beside(right)), right.beside(left)), None => (None, right) };
    // A reduction drops its seed's kind; keep an unresolved promotion on both sides.
    if left.is_some_and(|k| k.dtype.is_none()) || right.dtype.is_none() {
        return (numeric, Halving::Off, left.map(|_| Kind::default()), Kind::default());
    }
    let numeric = if takes_fixed(p) { numeric.for_kinds(left.into_iter().chain([right])) } else { numeric };
    let (numeric, halve) = numeric.halving(left.into_iter().chain([right]).map(Kind::float_width));
    let read = |k: Kind| Kind { dtype: k.dtype.map(|d| halve.operand(d)), ..k };
    (numeric, halve, left.map(read), read(right))
}

/// The kind that a call of `p` on operands of kinds `kinds`, the left one first, computes in: the kind that a dyadic call's operands join to (`pervasive::computes_in`), or a monadic call's one operand's kind.
fn computes_kind(p: Primitive, dyadic: bool, kinds: [Option<DType>; 2], numeric: Numeric) -> Option<DType> {
    if dyadic { crate::pervasive::computes_in(p, kinds[0], kinds[1], numeric) } else { kinds[1] }
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

/// One source is pointwise identical on both sides only when the paired reads agree.
fn same_at(same: bool, agreed: Option<&Agreement>) -> bool {
    same && agreed.is_none_or(|a| a.left == a.right)
}

/// `pervasive` on the values' kinds, constants and ranges, under the supplied pairing or ordinary agreement.
pub(crate) fn pervasive_on(p: Primitive, left: Option<&Value>, right: &Value, numeric: Numeric, agreed: Option<&Agreement>) -> Pervasive {
    let same = same_at(left.is_some_and(|l| std::ptr::eq(l, right)), agreed);
    operand_plan(p, left.map(Operand::Held).as_ref(), &Operand::Held(right), same, numeric)
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
    /// The bound on the operand's items beyond its kind and constant (`Pervasive::bounds`): a traced value's description's, or a held value's recorded range (`Value::range`).
    fn range(&self) -> Range { match self { Operand::Traced(desc) => desc.range, Operand::Held(value) => value.range() } }
}

/// A planned pervasive call: its result's description, and its plan.
pub(crate) struct Mapped { pub(crate) desc: Desc, pub(crate) plan: Pervasive, pub(crate) agreement: Agreement }

impl Mapped {
    pub(crate) fn new(plan: Pervasive, agreement: Agreement, atom: bool, weak: bool) -> Result<Self, &'static str> {
        let desc = plan.result_desc(agreement.layout.clone(), atom, weak)?;
        Ok(Self { desc, plan, agreement })
    }
}

/// The operand facts of a pervasive call, before settling its result.
fn operand_plan(p: Primitive, left: Option<&Operand>, right: &Operand, same: bool, numeric: Numeric) -> Pervasive {
    let bounds = [left.map_or(Range::ANY, Operand::range), right.range()];
    Pervasive { same, ..pervasive(p, left.map(Operand::kind), right.kind(), [left.and_then(Operand::constant), right.constant()], bounds, numeric) }
}


fn map_plan(p: Primitive, left: Option<&Operand>, right: &Operand, same: bool, numeric: Numeric) -> Result<Pervasive, &'static str> {
    if !p.scalar(left.is_some()) { return Err("a call that isn't pervasive"); }
    let plan = operand_plan(p, left, right, same, numeric);
    // 16-bit floats compute at 32 bits and round back (`pervasive::map`), which programs don't model yet.
    if plan.halve != Halving::Off { return Err("a 16-bit float"); }
    Ok(plan)
}

/// A pervasive call's output, with geometry from its operands and numeric facts from its plan.
fn mapped(plan: Pervasive, left: Option<&Operand>, right: &Operand, agreed: Option<&Agreement>) -> Result<Mapped, &'static str> {
    let agreement = match agreed {
        Some(agreement) => agreement.clone(),
        None => Agreement::new(&left.map_or_else(Layout::default, Operand::layout), &right.layout()).map_err(|_| "shapes that don't agree")?,
    };
    let atom = right.atom() && left.is_none_or(Operand::atom);
    let weak = atom && right.weak() && left.is_none_or(Operand::weak);
    Mapped::new(plan, agreement, atom, weak)
}

/// Plan the original operands under either their ordinary agreement or an already resolved pairing.
pub(crate) fn map(p: Primitive, left: Option<&Operand>, right: &Operand, same: bool, numeric: Numeric, agreed: Option<&Agreement>) -> Result<Mapped, &'static str> {
    let plan = map_plan(p, left, right, same_at(same, agreed), numeric)?;
    mapped(plan, left, right, agreed)
}

/// Checks a declared plan against its operands before settling its result. Constants compare by their bits, so that a NaN constant matches itself, only where a held operand still exposes them. Normalization can hide an observed scalar constant, as in `(⊂3)+4`; the declared plan owns that observation and its derived computing kind (`at`).
pub(crate) fn check_map(plan: &Pervasive, left: Option<&Operand>, right: &Operand) -> Result<Desc, &'static str> {
    let inferred = map_plan(plan.p, left, right, plan.same, plan.numeric)?;
    let held = [left.is_some_and(|o| matches!(o, Operand::Held(_))), matches!(right, Operand::Held(_))];
    let constants = |p: &Pervasive| [0, 1].map(|k| p.constants[k].filter(|_| held[k] && plan.constants[k].is_some()).map(f64::to_bits));
    let without_constants = |p: &Pervasive| Pervasive { constants: [None; 2], at: None, ..p.clone() };
    if without_constants(&inferred) != without_constants(plan) || constants(&inferred) != constants(plan) {
        return Err("a plan other than the one that its operands give");
    }
    mapped(plan.clone(), left, right, None).map(|mapped| mapped.desc)
}

/// The description of what `p` gives where it combines items that `items` describes into the base that `base` describes, as `Pairing::At` pairs them (`pervasive::combine_at`): the base's layout, and `p`'s result on one item of each joined with the base's kind as `Widening::Items` joins items. An error for an atom base, where `p` doesn't plan on the two, or where the join would leave the base mixed, which programs can't hold.
pub(crate) fn at(p: Primitive, base: &Desc, items: &Desc, numeric: Numeric) -> Result<Desc, &'static str> {
    if base.atom { return Err("an accumulation into an atom"); }
    let item = |d: &Desc| Desc { atom: true, layout: Layout::default(), ..d.clone() };
    let combined = map(p, Some(&Operand::Traced(&item(base))), &Operand::Traced(&item(items)), false, numeric, None)?.desc.kind;
    let kind = Kind::join([base.kind, combined], Widening::Items);
    let Some(dtype) = kind.dtype else { return Err("a write that leaves its target mixed"); };
    Ok(Desc { kind, atom: false, layout: base.layout.clone(), range: Range::of_dtype(dtype) })
}

/// The plan of a fold or scan by `p`: `kind`, the settings that the items and any seed give together (`prepared`), and the kinds of each step's operands, the running result's first, as those settings compute them. A scan's first step takes the seed, so a scan computes in the kind that the seed's and the items' kinds join to. A fold applies its seed after it folds the items. Where 16-bit floats compute in 32 bits, `halve` says whether float results round back to 16 bits.
pub(crate) struct Fold { pub(crate) kind: FoldKind, p: Primitive, pub(crate) numeric: Numeric, pub(crate) halve: Halving, kinds: [Option<DType>; 2] }

/// The plan of a fold or scan by `p` of items of kind `items`, with a seed of kind `seed`, under the session's settings `numeric`. A weak seed converts beside the items as `primitive::weakened_beside` converts one (`Kind::beside`).
pub(crate) fn fold(p: Primitive, kind: FoldKind, seed: Option<Kind>, items: Kind, numeric: Numeric) -> Fold {
    let (numeric, halve, seed, items) = prepared(p, seed, items, numeric);
    let left = if kind.scan { seed.unwrap_or(items) } else { items };
    Fold { kind, p, numeric, halve, kinds: [left.dtype, items.dtype] }
}

impl Fold {
    /// The kind that each step computes in (`pervasive::computes_in`).
    pub(crate) fn computes_in(&self) -> Option<DType> { crate::pervasive::computes_in(self.p, self.kinds[0], self.kinds[1], self.numeric) }
    /// The float width that integer items convert to before the fold or scan, where integers wrap, under static kinds or at a fixed width, and `p`'s table gives floats for them, as for `⍟`. Every item of the result then takes the call's kind, a lane's first included, and reading the items as floats first gives that through the float kernels. Where integers don't wrap, an item that no call computes, such as a scan's first, keeps its kind, so it's `None`.
    pub(crate) fn floated(&self) -> Option<FloatWidth> {
        if !self.numeric.wrapping() { return None; }
        let kind = self.computes_in().filter(|k| k.is_integral())?;
        match crate::pervasive::computes(self.p, true, [None; 2], kind, &[Range::ANY; 2], self.numeric)?.0 { DType::Float(width) => Some(width), _ => None }
    }
    /// The kind of the fold's result, as a lane of two or more items gives it without a seed: `p`'s running results where each step settles (`folded`), and otherwise `p`'s result on the first two items as their kinds decide it (`Pervasive::result`), before any widening that values cause under dynamic kinds.
    pub(crate) fn result(&self) -> Option<DType> {
        let [Some(left), Some(items)] = self.kinds else { return None };
        folded(self.p, left, items, self.numeric).ok().or_else(|| step(self.p, left, items, self.numeric).result())
    }
    /// The kind of an unseeded lane of one item. No binary call runs; only the fold's Boolean and static-float promotions apply.
    pub(crate) fn lone_kind(&self, input: Kind) -> Kind {
        match (self.floated(), input.dtype, self.result()) {
            (Some(width), ..) => computed(DType::Float(width), self.numeric, false),
            (None, Some(DType::Boolean), Some(dtype @ DType::Integer(_))) => computed(dtype, self.numeric, false), _ => input
        }
    }
    /// The description of the fold along `axis`, or along the fold's default axis, of a value that `right` describes, with an atom seed that `seed` describes, and the axis. Its kind is that of `p`'s running results (`folded`), joined with the first item's kind for a scan with no seed, and then `p`'s result beside a fold's seed. Its range bounds the running results (`range`).
    pub(crate) fn desc(&self, axis: Option<usize>, seed: Option<&Desc>, right: &Desc) -> Result<(Desc, usize), &'static str> {
        let rank = right.shape().len();
        if rank == 0 { return Err("a fold of an atom or a scalar"); }
        if seed.is_some_and(|s| !s.atom) { return Err("a seed that isn't an atom"); }
        let axis = axis.unwrap_or(self.kind.default_axis(rank));
        if right.shape()[axis] == 0 { return Err("an empty lane"); }
        if seed.is_none() && right.shape()[axis] == 1 {
            let layout = right.layout.folded(self.kind.scan, axis);
            return Ok((Desc { kind: self.lone_kind(right.kind), atom: layout.shape().is_empty(), layout, ..right.clone() }, axis));
        }
        let (numeric, [Some(left), Some(items)]) = (self.numeric, self.kinds) else { return Err("items that aren't numbers") };
        let running = folded(self.p, left, items, numeric)?;
        let dtype = match (self.kind.scan, seed) {
            (true, None) => numeric.kind(items.join(running, Widening::Arrays).ok_or("a scan whose kinds don't join")?),
            (false, Some(seed)) => pervasive(self.p, Some(computed(running, numeric, false)), seed.kind, [None; 2], [Range::ANY; 2], numeric).settled()?,
            _ => running,
        };
        let dtype = self.halve.result(dtype);
        let layout = right.layout.folded(self.kind.scan, axis);
        Ok((Desc { kind: computed(dtype, numeric, false), atom: layout.shape().is_empty(), layout,
                range: self.range(right.range, seed.map(|s| s.range), right.shape()[axis]) }, axis))
    }
    /// The bound on running results over lanes of up to `lane` items within `items`, from `seed` (`Range::folded`). The lane count bounds the result where `p` has an associative native reduction.
    pub(crate) fn range(&self, items: Range, seed: Option<Range>, lane: usize) -> Range {
        let [Some(left), Some(kind)] = self.kinds else { return Range::ANY };
        let step = step(self.p, left, kind, self.numeric);
        let lane = step.kernel().and_then(|k| k.fold()).map(|_| lane);
        Range::folded(items, seed, lane, |r| step.kernel().map_or(Range::ANY, |k| k.range(&r)))
    }
}

/// Whether `p` computes at the fixed width of its fixed integer operands. Comparisons and the exact functions don't.
pub(crate) fn takes_fixed(p: Primitive) -> bool { !matches!(p, Primitive::Exact(_) | Primitive::Compare(_)) }

/// The kind of a result of `dtype` from a call under `numeric`, weak if `weak` is. An integer result is fixed where `numeric` has a fixed width.
fn computed(dtype: DType, numeric: Numeric, weak: bool) -> Kind {
    let mut kind = Kind::stored(Some(dtype), numeric.fixed.filter(|_| matches!(dtype, DType::Integer(_))), weak);
    // An integer Number can narrow after execution; its result dtype does not prove its minimum width.
    if matches!(kind.minimum, Minimum::Known(_)) { kind.minimum = Minimum::Unknown; }
    kind
}

/// The plan of one step of a fold by `p` under `numeric`, on a running result of kind `left` and an item of kind `items`.
fn step(p: Primitive, left: DType, items: DType, numeric: Numeric) -> Pervasive {
    pervasive(p, Some(computed(left, numeric, false)), computed(items, numeric, false), [None; 2], [Range::ANY; 2], numeric)
}

/// The kind of a fold's running results under `numeric`: `p`'s result on `left`, its first step's left operand, and an item of kind `items`, where the kinds settle each step's result (`Pervasive::settled`). Each later step takes that result and an item, which must give it again. A fold of Booleans that rustymath runs as a truth fold, such as a count by `+` (`pervasive::truth_fold`), settles even where a step's kernel alone wouldn't, because its results stay within the number of items.
fn folded(p: Primitive, left: DType, items: DType, numeric: Numeric) -> Result<DType, &'static str> {
    let truth = left == DType::Boolean && items == DType::Boolean
        && crate::pervasive::table_kernel(p, true, DType::Boolean, DType::Boolean, [None; 2], numeric).is_some_and(|k| k.fold().is_some());
    // A fold's running integers take the integer width, as each step's narrowest width would widen along the lane; `Fold::range` bounds the stored result instead.
    let width = |k: DType| if matches!(k, DType::Integer(_)) { DType::Integer(numeric.integer_width()) } else { k };
    let step = |left| {
        let plan = step(p, left, items, numeric);
        if truth { plan.result().map(width).ok_or("an operand that isn't numbers") } else { plan.settled().map(width) }
    };
    let first = step(left)?;
    if step(first)? == first { Ok(first) } else { Err("a fold whose kind changes along it") }
}

/// The plan of a product `f.g`: `g`'s plan on the operands, which the fold by `f` of `g`'s results follows (`Product::folds`). rustymath's product picks a native kernel for the pair where it has one (`operation::product`).
pub(crate) struct Product { pub(crate) f: Primitive, pub(crate) g: Pervasive }

/// The plan of the product `f.g` on operands of kinds `left` and `right` under the session's settings `numeric`.
pub(crate) fn product(f: Primitive, g: Primitive, left: Kind, right: Kind, numeric: Numeric) -> Product {
    Product { f, g: pervasive(g, Some(left), right, [None; 2], [Range::ANY; 2], numeric) }
}

/// `product`'s plan for `f.g` on the values `left` and `right`, under the session's settings.
pub(crate) fn product_on(f: Primitive, g: Primitive, left: &Value, right: &Value) -> Product {
    product(f, g, Kind::value(left), Kind::value(right), Numeric::current())
}

impl Product {
    /// The settings that the fold by `f` of `g`'s results computes under: `g`'s, without the fixed width where `f` takes none or `g` gives Booleans.
    pub(crate) fn folds(&self) -> Numeric {
        Numeric { fixed: self.g.numeric.fixed.filter(|_| takes_fixed(self.f) && !crate::pervasive::booleans(self.g.p, true)), ..self.g.numeric }
    }
    /// The description of the product of values that `left` and `right` describe. Its kind is that of `f`'s fold of `g`'s results (`folded`), where the kinds settle `g`'s results and every step of the fold (`Pervasive::settled`).
    pub(crate) fn desc(&self, left: &Desc, right: &Desc) -> Result<Desc, &'static str> {
        let (Some(&k), Some(&j)) = (left.shape().last(), right.shape().first()) else { return Err("a product with an atom or a scalar") };
        if k != j || k == 0 { return Err("lengths that don't agree, or are empty"); }
        let layout = left.layout.contracted(&right.layout);
        let g = self.g.settled()?;
        let folds = self.folds();
        let result = self.g.halve.result(folded(self.f, g, g, folds)?);
        Ok(Desc { kind: computed(result, folds, false), atom: layout.shape().is_empty(), layout, range: Range::of_dtype(result) })
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
    Ok(Desc { kind: Kind::stored(source.kind.dtype, source.kind.fixed, false), atom: false, layout: layout.clone(), range: source.range })
}


pub(crate) fn structural(form: &crate::primitive::structural::Form, params: &[Value], inputs: &[Desc], fill: Option<&Desc>)
    -> Result<(Desc, rustymath::agreement::ReadPlan), crate::Error> {
    let layouts: Vec<&Layout> = inputs.iter().map(|d| &d.layout).collect();
    let (layout, plan) = crate::primitive::structural::plan(form, params, &layouts)?;
    let first = &inputs[0];
    let parts = || inputs.iter().chain(fill);
    let kind = crate::array::items_kind(parts().map(|d| (d.layout.count(), d.kind)), crate::array::Widening::Arrays).unwrap_or(first.kind);
    let atom = form.keeps_atoms() && layout.shape().is_empty() && parts().all(|d| d.atom);
    // Provenance cannot tighten a result whose every possible source/fill has unconstrained bounds.
    let range = if parts().all(|d| d.range == Range::ANY) { Range::ANY } else {
        match plan.positional(layout.shape()) {
            Some((source, _)) if fill.is_none() => inputs[source].range,
            _ => parts().fold(Range::of(Some(0.0)), |range, d| range.join(d.range)),
        }
    };
    let kind = Kind::stored(kind.dtype, kind.fixed, false);
    Ok((Desc { kind, atom, layout, range }, plan))
}

/// The description of a copy of one item of a value that `source` describes, as `Value::at` reads one: an atom of the source's type and fixedness.
pub(crate) fn item(source: &Desc) -> Result<Desc, &'static str> { Ok(Desc { atom: true, ..copy(source, &Layout::default())? }) }

/// The description of the mask that a program's picks give (`Step::Picks`): Booleans in the layout of `of`, whose real numbers `⌈` or `⌊` picks from along axis `axis`.
pub(crate) fn picks(of: &Desc, axis: usize) -> Result<Desc, &'static str> {
    if axis >= of.shape().len() { return Err("an axis that the items lack"); }
    if !matches!(of.kind.dtype, Some(DType::Boolean | DType::Integer(_) | DType::Float(_))) { return Err("picks from items that aren't real numbers"); }
    Ok(Desc { kind: Kind::stored(Some(DType::Boolean), None, false), range: Range::of_dtype(DType::Boolean), ..copy(of, &of.layout)? })
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
    let kind = Kind::join(cases, Widening::Arrays);
    let dtype = kind.dtype.filter(|&d| d != DType::Character).ok_or("cases that aren't numbers")?;
    if cases.iter().any(|c| !c.weak && c.dtype != Some(dtype)) { return Err("cases of different kinds"); }
    Ok(Select { desc: Desc { kind: Kind::stored(Some(dtype), kind.fixed, false), atom: false, layout, range: Range::of_dtype(dtype).meet(yes.range().join(no.range())) }, yes: yes_map,
        no: no_map })
}

/// The description of `source` with every number converted to `to`, as `Value::converted` converts it for width marks, `•float` and fixed marks: floats of a width, where complex numbers keep complex parts of that width, or fixed integers of a width. The result is strong.
pub(crate) fn convert(source: &Desc, to: DType) -> Result<Desc, &'static str> {
    let (dtype, fixed) = match (source.dtype(), to) {
        (DType::Character, _) => return Err("characters"), (DType::Complex(_), DType::Float(width)) => (DType::Complex(width.into()), None),
        (DType::Complex(_), _) => return Err("complex numbers that convert to integers"), (_, DType::Float(_)) => (to, None),
        (_, DType::Integer(width)) => (to, Some(width)), _ => return Err("a conversion to a type that no mark gives")
    };
    Ok(Desc { kind: Kind::stored(Some(dtype), fixed, false), range: Range::of_dtype(dtype), ..source.clone() })
}

/// The description of a fold step of `p` along `axis` over `items`, with `seed` where it has one, and the axis it folds along (`fold`, then `Fold::desc`): the one plan that recording, fusion and verification give a `Step::Fold`.
pub(crate) fn fold_desc(
    p: Primitive, kind: FoldKind, axis: Option<usize>, seed: Option<&Desc>, items: &Desc, numeric: Numeric) -> Result<(Desc, usize), &'static str> {
    fold(p, kind, seed.map(|s| s.kind), items.kind, numeric).desc(axis, seed, items)
}

/// The description of a product step `f.g` of `left` and `right`, `stacked` where it multiplies stacks of matrices (`product`, then `Product::desc` or `Product::stacked_desc`).
pub(crate) fn product_desc(f: Primitive, g: Primitive, left: &Desc, right: &Desc, stacked: bool, numeric: Numeric) -> Result<Desc, &'static str> {
    let plan = product(f, g, left.kind, right.kind, numeric);
    if stacked { plan.stacked_desc(left, right) } else { plan.desc(left, right) }
}
