//! Steps' formulas recorded as JIT operations, for the parts of a stage that compile (`recorded`).

use super::*;

/// Steps recorded into `kernel` at type `ty`. As `rustymath::recipe::Ops`, it records a formula's recipe, so a compiled part computes as rustymath's kernels do. `held` pairs each operand that the program holds with its number, which a recipe reads while recording (`Ops::constant`). A recipe step that no compiled kernel takes, such as a shift, declines the recording (`declined`), and the step doesn't lower. `tolerance` is the comparison tolerance at the step's type (`Numeric::tolerance_of`), which a recipe reads through `Ops::tolerance`, as an eager kernel reads it from its run `Settings`.
struct Recording<'a> {
    kernel: std::cell::RefCell<&'a mut jit::Kernel>, ty: jit::Ty, held: [Option<(jit::Value, f64)>; 2], declined: std::cell::Cell<bool>, tolerance: Tolerance
}

impl Recording<'_> {
    /// The value of a new step `op` of `x` and `y` that gives type `ty`.
    fn step(&self, op: jit::Op, ty: jit::Ty, x: jit::Value, y: Option<jit::Value>) -> jit::Value {
        let mut kernel = self.kernel.borrow_mut();
        kernel.steps.push(jit::Step { op, ty, x, y });
        jit::Value::Step(kernel.steps.len() - 1)
    }
    /// The value of a new step `op` of `x` and `y` at this type.
    fn at(&self, op: jit::Op, x: jit::Value, y: Option<jit::Value>) -> jit::Value { self.step(op, self.ty, x, y) }
    /// The kernel's constant input `x` at this type, added the first time it's read.
    fn constant(&self, x: f64) -> jit::Value { self.input(jit::Input::constant(self.ty, x)) }
    /// The kernel's input `input`, added the first time it's read.
    fn input(&self, input: jit::Input) -> jit::Value {
        let mut kernel = self.kernel.borrow_mut();
        let k = kernel.inputs.iter().position(|&i| i == input).unwrap_or_else(|| {
            kernel.inputs.push(input);
            kernel.inputs.len() - 1
        });
        jit::Value::Input(k)
    }
    /// The number that `v` holds where it's known while recording: an operand that the program holds, or a constant input.
    fn known(&self, v: jit::Value) -> Option<f64> {
        if let Some(h) = self.held.iter().flatten().find(|h| h.0 == v) { return Some(h.1); }
        let jit::Value::Input(k) = v else { return None };
        match self.kernel.borrow().inputs[k] { jit::Input::Constant(t, bits) => Some(jit::Input::decoded(t, bits)), _ => None }
    }
    /// `x` in place of a step that no compiled kernel takes, which declines the recording.
    fn decline(&self, x: jit::Value) -> jit::Value { self.declined.set(true); x }
    /// Whether the recording's values are integers or Booleans.
    fn integral(&self) -> bool { !matches!(self.ty, jit::Ty::F32 | jit::Ty::F64) }
    /// The Boolean function whose truth table is `t`, on `x`, through the kernel's `And`, `Or`, `Xor` and `Not`: for one argument, bit `i` is the result for `i`, and for two, bit `2x + y`. `None` for any table but those and their negations.
    fn table(&self, t: u8, x: &[jit::Value]) -> Option<jit::Value> {
        let b = |op, x, y| self.step(op, jit::Ty::Bool, x, y);
        let (op, not) = match (x.len(), t) {
            (1, 1) => return Some(b(jit::Op::Not, x[0], None)), (2, 8) => (jit::Op::And, false), (2, 14) => (jit::Op::Or, false),
            (2, 6) => (jit::Op::Xor, false), (2, 7) => (jit::Op::And, true), (2, 1) => (jit::Op::Or, true), (2, 9) => (jit::Op::Xor, true), _ => return None
        };
        let v = b(op, x[0], Some(x[1]));
        Some(if not { b(jit::Op::Not, v, None) } else { v })
    }
}

/// A recipe step that no compiled kernel takes declines the recording (`Recording::decline`).
impl<F: rustymath::Float> rustymath::recipe::Ops<F> for Recording<'_> {
    type V = jit::Value;
    type B = jit::Value;
    type I = jit::Value;
    type W = jit::Value;
    fn width(&self) -> FloatWidth { if self.ty == jit::Ty::F32 { FloatWidth::F32 } else { FloatWidth::F64 } }
    fn fused(&self) -> bool { rustymath::FUSED }
    fn integral(&self) -> bool { Recording::integral(self) }
    fn constant(&self, v: jit::Value) -> Option<F> { self.known(v).map(F::narrow) }
    fn tolerance(&self, _: F) -> jit::Value { self.constant(self.tolerance.value()) }
    fn widen(&self, x: jit::Value) -> jit::Value { x }
    fn wide_number(&self, x: f64) -> jit::Value { self.constant(x) }
    fn wide_multiply_add(&self, x: jit::Value, y: jit::Value, z: jit::Value) -> jit::Value { <Self as rustymath::recipe::Ops<F>>::multiply_add(self, x, y, z) }
    fn narrow(&self, x: jit::Value) -> jit::Value { x }
    fn integer(&self, x: i64) -> jit::Value { let c = self.constant(x as f64); if self.integral() { c } else { self.decline(c) } }
    fn integer_op(&self, op: rustymath::recipe::Integer, x: jit::Value, y: jit::Value) -> jit::Value {
        use rustymath::recipe::Integer;
        let op = match op {
            Integer::Add => jit::Op::Add, Integer::Subtract => jit::Op::Subtract, Integer::And => jit::Op::And, Integer::Or => jit::Op::Or,
            Integer::Xor => jit::Op::Xor, _ => return self.decline(x)
        };
        if self.integral() { self.at(op, x, Some(y)) } else { self.decline(x) }
    }
    fn integer_compare(&self, op: rustymath::recipe::Cmp, x: jit::Value, y: jit::Value) -> jit::Value {
        if self.integral() { <Self as rustymath::recipe::Ops<F>>::compare(self, op, x, y) } else { self.decline(x) }
    }
    fn integer_select(&self, m: jit::Value, x: jit::Value, y: jit::Value) -> jit::Value {
        if self.integral() { self.at(jit::Op::Select(m), x, Some(y)) } else { self.decline(x) }
    }
    fn to_integer(&self, x: jit::Value) -> jit::Value { if self.integral() { x } else { self.decline(x) } }
    fn from_integer(&self, x: jit::Value) -> jit::Value { if self.integral() { x } else { self.decline(x) } }
    fn to_bits(&self, x: jit::Value) -> jit::Value { if self.integral() { x } else { self.decline(x) } }
    fn from_bits(&self, x: jit::Value) -> jit::Value { if self.integral() { x } else { self.decline(x) } }
    fn overflow(&self, _: rustymath::recipe::Binary, x: jit::Value, _: jit::Value) -> jit::Value { self.decline(x) }
    fn number(&self, x: F) -> jit::Value { self.constant(x.wide()) }
    fn unary(&self, op: rustymath::recipe::Unary, x: jit::Value) -> jit::Value {
        use rustymath::recipe::Unary;
        let op = match op {
            Unary::Negate => jit::Op::Negate, Unary::Magnitude => jit::Op::Magnitude, Unary::Sqrt => jit::Op::Sqrt, Unary::Floor => jit::Op::Floor,
            _ => return self.decline(x)
        };
        self.at(op, x, None)
    }
    fn binary(&self, op: rustymath::recipe::Binary, x: jit::Value, y: jit::Value) -> jit::Value {
        use rustymath::recipe::Binary;
        let op = match op {
            Binary::Add => jit::Op::Add, Binary::Subtract => jit::Op::Subtract, Binary::Multiply => jit::Op::Multiply, Binary::Divide => jit::Op::Divide,
            Binary::Larger => jit::Op::Maximum, Binary::Smaller => jit::Op::Minimum, Binary::BitAnd if self.integral() => jit::Op::And,
            _ => return self.decline(x)
        };
        self.at(op, x, Some(y))
    }
    fn compare(&self, c: rustymath::recipe::Cmp, x: jit::Value, y: jit::Value) -> jit::Value {
        use rustymath::recipe::Cmp;
        let c = match c { Cmp::Less => jit::Cmp::Lt, Cmp::LessEqual => jit::Cmp::Le, Cmp::Equal => jit::Cmp::Eq };
        self.step(jit::Op::Compare(c), jit::Ty::Bool, x, Some(y))
    }
    fn multiply_add(&self, x: jit::Value, y: jit::Value, z: jit::Value) -> jit::Value {
        if !rustymath::FUSED { return self.at(jit::Op::Add, self.at(jit::Op::Multiply, x, Some(y)), Some(z)); }
        self.at(jit::Op::MultiplyAdd(z), x, Some(y))
    }
    fn and(&self, x: jit::Value, y: jit::Value) -> jit::Value { self.step(jit::Op::And, jit::Ty::Bool, x, Some(y)) }
    fn or(&self, x: jit::Value, y: jit::Value) -> jit::Value { self.step(jit::Op::Or, jit::Ty::Bool, x, Some(y)) }
    fn not(&self, x: jit::Value) -> jit::Value { self.step(jit::Op::Not, jit::Ty::Bool, x, None) }
    fn select(&self, m: jit::Value, x: jit::Value, y: jit::Value) -> jit::Value { self.at(jit::Op::Select(m), x, Some(y)) }
}

/// Records into `kernel` the steps of a pervasive step of plan `plan` at type `at` on `x` and, for a dyad, `y`, by the formula that the step's kernel runs (`pervasive::formula`), so a compiled part computes as the kernel does. `held` gives each operand's number where the program holds it. A recipe reads it while recording, as a power's exponent, a divisor or a comparison's one number. The last recorded step gives the result: a test's Booleans, or items of type `at`. A step whose type `made` differs from `at` lowers only for a test, since a kernel converts other results by rules that a compiled step doesn't follow. `None` where the step doesn't lower: its formula has no recipe here, or the recipe takes a step that no compiled kernel takes, as an exact float residue does.
pub(super) fn recorded(
    kernel: &mut jit::Kernel, plan: &Pervasive, at: DType, made: jit::Ty, x: jit::Value, y: Option<jit::Value>, held: [Option<f64>; 2]) -> Option<()> {
    let ty = jit::Ty::of(at)?;
    let o = Recording { kernel: std::cell::RefCell::new(kernel), ty, held: [held[0].map(|c| (x, c)), y.zip(held[1])], declined: std::cell::Cell::new(false),
        tolerance: plan.numeric.tolerance_of(at) };
    let (args, arity, integral) = ([x, y.unwrap_or(x)], 1 + usize::from(y.is_some()), o.integral());
    let (mathish, t) = (plan.numeric.mathish_at(at), o.tolerance);
    let op = pervasive::formula(plan.p, arity, mathish, integral, t)?;
    let result = match ty {
        jit::Ty::F32 => op.visit(arity, Emitted::<f32> { o: &o, args, at: std::marker::PhantomData }),
        _ => op.visit(arity, Emitted::<f64> { o: &o, args, at: std::marker::PhantomData }),
    }?;
    let steps = o.kernel.borrow().steps.len();
    let gives = o.kernel.borrow().steps.last()?.ty;
    (!o.declined.get() && result == jit::Value::Step(steps - 1) && (made == ty || gives == jit::Ty::Bool)).then_some(())
}

/// A step's recipe through `o` at float type `G`, on `args`: on Booleans, the formula's truth table (`Recording::table`); for a test, against the one number that the program holds (`Formula::emit_against`) or on its arguments (`Formula::emit_test`); otherwise the formula's own recipe (`Formula::emit`).
struct Emitted<'a, 'b, G> { o: &'a Recording<'b>, args: [jit::Value; 2], at: std::marker::PhantomData<G> }

impl<G: rustymath::Float> rustymath::facts::Visit for Emitted<'_, '_, G> {
    type Out = Option<jit::Value>;
    fn visit<K: rustymath::facts::Formula<N>, const N: usize>(self, k: K) -> Option<jit::Value> {
        let (o, args) = (self.o, std::array::from_fn(|i| self.args[i]));
        if o.ty == jit::Ty::Bool && let Some(v) = k.truth_table().and_then(|t| o.table(t, &args)) { return Some(v); }
        if !K::BOOLEAN { return k.emit::<G, _>(o, args); }
        let against = match o.held { [_, Some((_, c))] => Some((c, self.args[0], true)), [Some((_, c)), None] => Some((c, self.args[1], false)), _ => None };
        if N == 2 && let Some((c, v, left)) = against && let Some(b) = k.emit_against::<G, _>(o, G::narrow(c), v, left) { return Some(b); }
        k.emit_test::<G, _>(o, args)
    }
}

/// Whether a compiled kernel converts items of type `from` to `to` as a step whose tolerance is `tolerance` reads them (`Buffer::floored`): to any type but Booleans, and from a float to an integer only where the tolerance is exact, since the kernel floors exactly.
pub(super) fn converts(from: DType, to: DType, tolerance: Tolerance) -> bool {
    from == to || (to != DType::Boolean && (matches!(to, DType::Float(_)) || !matches!(from, DType::Float(_)) || tolerance.is_exact()))
}
