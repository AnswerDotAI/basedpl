//! Borrowed call metadata, resolved parameters and execution-only logical reads.
use super::*;
/// Declared metadata and resolved parameters, shared by live construction and refill.
pub(crate) trait CallBindings {
    fn len(&self) -> usize;
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>>;
    /// Whether position `position` holds a constant: a literal node (target part 4).
    fn constant(&self, position: usize) -> bool;
    fn parameter(&self, position: usize) -> Option<&Value>;
    fn function(&self, position: usize) -> Option<&Function> {
        match self.parameter(position)? { Value::Function(function) => Some(function), _ => None }
    }
}

pub(crate) fn resolved_parameter(value: &Value) -> Option<&Value> {
    match value { Value::Pending(pending) => pending.completed(), Value::Carried { .. } => None, _ => Some(value) }
}

struct Omitted<'a> { bindings: &'a dyn CallBindings, range: std::ops::Range<usize> }
impl Omitted<'_> {
    fn index(&self, position: usize) -> usize { position + if position >= self.range.start { self.range.len() } else { 0 } }
}
impl CallBindings for Omitted<'_> {
    fn len(&self) -> usize { self.bindings.len() - self.range.len() }
    fn parameter(&self, position: usize) -> Option<&Value> { self.bindings.parameter(self.index(position)) }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> { self.bindings.desc(self.index(position)) }
    fn constant(&self, position: usize) -> bool { self.bindings.constant(self.index(position)) }
    fn function(&self, position: usize) -> Option<&Function> { self.bindings.function(self.index(position)) }
}

/// Where each section of a call's list lies, by tree root and by list position: function values, a select's selector, arguments, options, fill, settings, implicit inputs and captures. The 1 home of these boundaries.
#[derive(Clone, Copy)]
pub(crate) struct CallLayout<'a> {
    operation: &'a Operation, trees: Option<&'a crate::array::Trees>, explicit: usize, explicit_roots: usize, captures: &'a crate::eval::CaptureBindings,
}

impl<'a> CallLayout<'a> {
    pub(crate) fn of<L: ?Sized>(origin: &'a Origin<L>) -> Self {
        Self { operation: &origin.operation, trees: origin.trees.as_deref(), explicit: origin.explicit as usize, explicit_roots: origin.explicit_roots as usize,
            captures: &origin.bindings }
    }
    /// Root `index` with its list positions.
    pub(crate) fn root(&self, index: Root) -> TreeRoot { TreeRoot::at(self.trees, index) }
    /// The list position where root `root` starts. The root after the explicit call starts at the explicit call's end.
    pub(crate) fn start(&self, root: Root) -> usize { if root.get() == self.explicit_roots { self.explicit } else { self.root(root).leaves.start } }
    /// The list positions of the roots `roots`.
    pub(crate) fn leaves(&self, roots: Roots) -> std::ops::Range<usize> { self.start(roots.start)..self.start(roots.end) }
    /// The explicit call's roots: function values, selector, arguments, options and fill.
    pub(crate) fn explicit_roots(&self) -> Roots { Roots::new(Root(0), Root(self.explicit_roots as u32)) }
    /// The explicit call's list positions.
    pub(crate) fn explicit(&self) -> std::ops::Range<usize> { 0..self.explicit }
    /// The function values' roots, first in the call.
    pub(crate) fn functions(&self) -> Roots { Roots::new(Root(0), Root(self.operation.functions() as u32)) }
    /// The first list position after the function values.
    pub(crate) fn data(&self) -> usize { self.start(self.functions().end) }
    /// The options root, and whether a fill root follows it.
    pub(crate) fn options(&self) -> Option<(Root, bool)> { options_root(self.operation, self.explicit_roots) }
    /// The argument roots: after the function values and a select's selector, before options and fill.
    pub(crate) fn arguments(&self) -> Roots {
        let end = if matches!(self.operation, Operation::Join { fill: true, .. }) { Root(self.explicit_roots as u32 - 1) }
            else { self.options().map_or(Root(self.explicit_roots as u32), |(root, _)| root) };
        Roots::new(self.functions().end.after(usize::from(matches!(self.operation, Operation::Select { .. }))), end)
    }
    /// The argument roots' list positions.
    pub(crate) fn argument_positions(&self) -> std::ops::Range<usize> { self.leaves(self.arguments()) }
    /// The fill root: a filled strand's last root, or the root after the options.
    pub(crate) fn fill(&self) -> Option<TreeRoot> {
        match self.operation {
            Operation::Join { fill: true, .. } => Some(self.root(Root(self.explicit_roots as u32 - 1))),
            _ => self.options().filter(|(_, fill)| *fill).map(|(root, _)| self.root(root.after(1))),
        }
    }
    /// The settings' list positions, in `Operation::settings` order.
    pub(crate) fn settings(&self) -> std::ops::Range<usize> { self.explicit..self.explicit + self.operation.settings().len() }
    pub(crate) fn setting(&self, setting: Setting) -> Option<usize> {
        self.operation.settings().iter().position(|&name| name == setting).map(|index| self.explicit + index)
    }
    fn implicit_inputs(&self) -> &'static [crate::program::ImplicitInput] { self.operation.implicit_inputs(self.captures) }
    /// The implicit inputs' list positions, after the settings.
    pub(crate) fn implicit(&self) -> std::ops::Range<usize> { let first = self.settings().end; first..first + self.implicit_inputs().len() }
    pub(crate) fn implicit_input(&self, input: crate::program::ImplicitInput) -> Option<usize> {
        self.implicit_inputs().iter().position(|&state| state == input).map(|index| self.implicit().start + index)
    }
    /// The first captured value's list position.
    pub(crate) fn captures(&self) -> usize { self.implicit().end }
    /// The implicit inputs' roots, after the settings' roots. Settings and implicit inputs are 1 leaf, so 1 root, each.
    pub(crate) fn implicit_roots(&self) -> Roots {
        let first = Root((self.explicit_roots + self.operation.settings().len()) as u32);
        Roots::new(first, first.after(self.implicit_inputs().len()))
    }
    /// The first captured value's root.
    pub(crate) fn capture_roots(&self) -> Root { self.implicit_roots().end }
    /// How the call reads position `position`: a setting as a value, an implicit input by its kind, an explicit operand by its role, a capture only as a description.
    pub(crate) fn read_kind(&self, operand_roles: &Roles, position: usize) -> Option<ReadKind> {
        if self.settings().contains(&position) { return Some(ReadKind::Value); }
        let implicit = self.implicit();
        if implicit.contains(&position) { return self.implicit_inputs()[position - implicit.start].read(); }
        let kind = operand_roles.kind(position);
        if kind == Some(ReadKind::Description) || position < self.explicit { kind } else { None }
    }
}

fn options_root(operation: &Operation, roots: usize) -> Option<(Root, bool)> {
    match operation {
        Operation::Read(ReadOp::Cells { operation, .. }) => options_root(operation, roots),
        Operation::Primitive { pairing: Pairing::Explicit { fill }, .. } | Operation::Fold { pairing: Pairing::Explicit { fill }, .. }
        | Operation::Product { pairing: Pairing::Explicit { fill }, .. } => Some((Root((roots - 1 - usize::from(*fill)) as u32), *fill)),
        _ => None,
    }
}

/// Borrowed facts of one resolved call. Describing and planning allocate no parallel operand lists.
#[derive(Clone, Copy)]
pub(crate) struct CallView<'a> {
    pub(crate) operation: &'a Operation, pub(crate) mode: Mode, pub(crate) roles: &'a Roles, explicit: usize,
    explicit_roots: usize, pub(crate) trees: Option<&'a std::sync::Arc<crate::array::Trees>>,
    pub(crate) captures: &'a std::sync::Arc<crate::eval::CaptureBindings>, bindings: &'a dyn CallBindings, pub(crate) site: Option<&'a Span>,
    context: Option<u32>
}

impl CallView<'_> {
    fn new<'a, L: ?Sized>(origin: &'a Origin<L>, bindings: &'a dyn CallBindings) -> CallView<'a> {
        CallView { operation: &origin.operation, mode: origin.mode, site: origin.site.as_ref(), context: origin.context,
            roles: &origin.roles, explicit: origin.explicit as usize, explicit_roots: origin.explicit_roots as usize, trees: origin.trees.as_ref(),
            captures: &origin.bindings, bindings }
    }
    fn projected<'a>(&'a self, operation: &'a Operation, bindings: &'a dyn CallBindings) -> CallView<'a> {
        CallView { operation, bindings, ..*self }
    }
    pub(crate) fn origin<L>(&self, list: L) -> crate::program::Origin<L> {
        crate::program::Origin { operation: self.operation.clone(), mode: self.mode, roles: self.roles.clone(),
            explicit: self.explicit as u32, explicit_roots: self.explicit_roots as u32, trees: self.trees.cloned(), site: self.site.cloned(),
            context: self.context, bindings: self.captures.clone(), list }
    }
    pub(crate) fn layout(&self) -> CallLayout<'_> {
        CallLayout { operation: self.operation, trees: self.trees.map(|trees| &**trees), explicit: self.explicit, explicit_roots: self.explicit_roots,
            captures: self.captures }
    }
    pub(crate) fn kernel(&self, variant: rustymath::facts::Variant) -> crate::error::Result<Option<rustymath::operation::Kernel>> {
        Operation::kernel(self, variant)
    }
    pub(crate) fn row(&self) -> Option<&'static Row> {
        match self.operation {
            Operation::Primitive { primitive, .. } => Some(primitive.info()),
            Operation::System | Operation::Draw { .. } | Operation::Effect(_) => Some(crate::system::native_row()), _ => None
        }
    }
    pub(crate) fn axis_root(&self) -> Option<TreeRoot> {
        let layout = self.layout();
        layout.options().map(|(root, _)| layout.root(root))
            .filter(|root| !self.root_layout(root.index).is_some_and(|layout| layout.shape() == [0] && layout.keys(0).is_some()))
    }
    pub(crate) fn data_positions(&self) -> impl Iterator<Item = usize> + '_ {
        let layout = self.layout();
        layout.leaves(layout.arguments()).chain(layout.fill().into_iter().flat_map(|root| root.leaves)).filter(move |&position| {
            matches!(self.roles.kind(position), None | Some(ReadKind::Kind))
                && (!matches!(self.operation, Operation::At { .. }) || !layout.root(Root(1)).leaves.contains(&position))
        })
    }
    pub(crate) fn operand_kind(&self) -> crate::error::Result<Option<crate::number::Kind>> {
        let mut joined: Option<crate::number::Kind> = None;
        for position in self.data_positions() {
            let Some(desc) = self.desc(position) else { return Ok(None) };
            if desc.kind.dtype() == rustymath::buffer::DType::Character { continue; }
            joined = Some(match joined { None => desc.kind, Some(kind) => kind.join(desc.kind).ok_or_else(|| Error::domain("numeric kinds do not agree"))? });
        }
        Ok(joined)
    }
    pub(crate) fn numeric(&self) -> crate::error::Result<Numeric> {
        let mut numeric = crate::settings::captured(self)?;
        numeric.fixed = self.operand_kind()?.and_then(crate::number::Kind::fixed_width);
        Ok(numeric)
    }
    pub(crate) fn variant(&self, dtype: rustymath::buffer::DType) -> crate::error::Result<rustymath::facts::Variant> {
        let numeric = Numeric { mathish: self.mode.mathish, kinds: if self.mode.dynamic { Kinds::Dynamic } else { Kinds::Static },
            fixed: self.operand_kind()?.and_then(crate::number::Kind::fixed_width), ..Numeric::DEFAULT };
        Ok(rustymath::facts::Variant { dtype, checked: self.mode.dynamic, wrapping: dtype.is_integral(),
            mathish: numeric.mathish_at(crate::array::float_width_of(dtype)) })
    }
    pub(crate) fn kernel_for(
        &self, primitive: Primitive, dyadic: bool, variant: Variant) -> Result<Option<Kernel>, Error> {
        let mut numeric = self.numeric()?;
        numeric.tolerance = crate::array::float_width_of(variant.dtype).map_or(tol::Tolerance::EXACT, |width| numeric.tolerance_at(width));
        let code = self.parameter(self.layout().data())?.as_deref().and_then(Value::as_number).and_then(|number| number.integer().ok());
        let mut types = [variant.dtype; 3];
        let mut ranges = [Range::ANY; 3];
        let direct = matches!(self.operation, Operation::Primitive { .. } | Operation::Product { .. });
        let mut count = 0;
        let layout = self.layout();
        let arguments = layout.leaves(layout.arguments());
        for position in self.data_positions().filter(|position| arguments.contains(position)) {
            let slot = types.get_mut(count).ok_or_else(|| Error::domain("too many numeric operands"))?;
            *slot = match self.desc(position) {
                Some(desc) => { if direct { ranges[count] = desc.bounds(); } desc.kind.dtype() }
                None => return Ok(None),
            };
            count += 1;
        }
        primitive.kernel(dyadic, &ObjectArgs { numeric, code, variant, types: &types[..count], ranges: &ranges[..count] })
    }
    pub(crate) fn len(&self) -> usize { self.bindings.len() }
    fn parameter_allowed(&self, position: usize) -> bool {
        let layout = self.layout();
        position < layout.data()
            || self.bindings.constant(position)
            || layout.read_kind(self.roles, position).is_some_and(ReadKind::is_parameter)
    }
    pub(crate) fn parameter_at(&self, position: usize) -> Result<Cow<'_, Value>, Error> {
        if !self.parameter_allowed(position) { return Err(Error::internal()); }
        self.bindings.parameter(position).map(Cow::Borrowed).or_internal()
    }
    pub(crate) fn root_parameter(&self, root: Root) -> Result<Cow<'_, Value>, Error> {
        let root = self.layout().root(root);
        match self.trees {
            Some(trees) => trees.read_root(&root, |position| self.parameter_at(position).map(Cow::into_owned)).map(Cow::Owned),
            None => self.parameter_at(root.leaves.start),
        }
    }
    pub(crate) fn root_desc(&self, root: Root) -> Option<Cow<'_, Desc>> {
        let root = self.layout().root(root);
        match self.trees { Some(trees) => trees.describe_root(&root, |position| self.desc(position)), None => self.desc(root.leaves.start) }
    }
    pub(crate) fn root_atom(&self, root: Root) -> Option<bool> {
        let at = self.layout().root(root);
        if self.trees.is_some_and(|trees| !matches!(trees.root_at(&at), Tree::Leaf)) { return Some(false); }
        self.root_desc(root).map(|desc| desc.atom)
    }
    pub(crate) fn root_layout(&self, root: Root) -> Option<Cow<'_, Layout>> {
        let at = self.layout().root(root);
        if let Some(trees) = self.trees && let Tree::Array { layout, .. } = trees.root_at(&at) { return Some(Cow::Borrowed(layout)); }
        self.root_desc(root).map(|desc| match desc { Cow::Borrowed(desc) => Cow::Borrowed(&desc.layout), Cow::Owned(desc) => Cow::Owned(desc.layout) })
    }
    pub(crate) fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> { self.bindings.desc(position) }
    /// Whether position `position` holds a constant: a literal node (target part 4).
    pub(crate) fn constant(&self, position: usize) -> bool { self.bindings.constant(position) }
    pub(crate) fn is_function(&self, position: usize) -> bool { self.bindings.function(position).is_some() }
    pub(crate) fn function(&self, position: usize) -> Option<&Function> {
        if self.parameter_allowed(position) { self.bindings.function(position) } else { None }
    }
    pub(crate) fn product_map(&self) -> Option<&Function> { self.function(self.operation.product_functions()?.map) }
    pub(crate) fn product_fold(&self) -> Option<&Function> { self.function(self.operation.product_functions()?.fold?) }
    pub(crate) fn source_roots(&self) -> Result<Vec<TreeRoot>, Error> { Operation::source_roots(self) }
    pub(crate) fn mathematical_positions(&self) -> Result<Vec<usize>, Error> {
        let fill = (!matches!(self.operation, Operation::ReadRelation { .. })).then(|| self.layout().fill()).flatten();
        let mut positions: Vec<_> = self.source_roots()?.into_iter().chain(fill).flat_map(|root| root.leaves).collect();
        positions.sort_unstable();
        positions.dedup();
        Ok(positions)
    }
    /// The value of the parameter at list position `position`, where its role reads it at the call.
    pub(crate) fn parameter(&self, position: usize) -> Result<Option<Cow<'_, Value>>, Error> {
        match self.roles.kind(position) {
            Some(read) if read.is_parameter() => self.parameter_at(position).map(Some),
            _ => Ok(None),
        }
    }
    pub(crate) fn with_cells<T>(&self, apply: impl FnOnce(&CallView<'_>, usize) -> Result<T, Error>) -> Result<T, Error> {
        let Operation::Read(ReadOp::Cells { operation, frame, inputs }) = self.operation else { return apply(self, 0) };
        let bindings = CellBindings { bindings: self.bindings, frame: *frame, inputs };
        apply(&self.projected(operation, &bindings), *frame)
    }
    pub(crate) fn with_relation<T>(
        &self, apply: impl FnOnce(&CallView<'_>, Relation, &[TreeRoot], &std::ops::Range<usize>) -> crate::error::Result<T>
    ) -> crate::error::Result<T> {
        use crate::array::Trees;
        let call = self;
        let Operation::ReadRelation { operation, relation, .. } = call.operation else { unreachable!("a read relation") };
        if matches!(relation, Relation::Input(_) | Relation::Validate) {
            let original = CallView { operation, ..*call };
            let original_roles = Operation::roles(&original)?;
            return apply(&CallView { roles: &original_roles, ..original }, *relation, &[], &(call.explicit..call.explicit));
        }
        let count = if matches!(relation, Relation::Back(_)) { 2 } else { 1 };
        let first = call.explicit_roots.checked_sub(count).or_internal()?;
        let trees = call.trees.cloned().unwrap_or_else(|| std::sync::Arc::new(Trees::leaves(call.len())));
        let removed: Vec<_> = trees.roots().skip(first).take(count).collect();
        if removed.len() != count { return Err(Error::internal()); }
        let range = removed[0].leaves.start..removed[count - 1].leaves.end;
        let kept: Vec<_> = trees.roots().filter(|root| root.index.get() < first || root.index.get() >= call.explicit_roots).collect();
        let original_trees = std::sync::Arc::new(trees.project_roots(&kept));
        let bindings = Omitted { bindings: call.bindings, range: range.clone() };
        let original = CallView { operation, mode: call.mode, roles: call.roles, explicit: call.explicit - range.len(), explicit_roots: first,
            trees: Some(&original_trees), captures: call.captures, bindings: &bindings, site: call.site, context: call.context };
        let original_roles = Operation::roles(&original)?;
        let original = CallView { roles: &original_roles, ..original };
        apply(&original, *relation, &removed, &range)
    }
}

pub(crate) struct RunView<'a> { call: CallView<'a>, bindings: &'a crate::program::BoundCall<'a> }
impl<'a> std::ops::Deref for RunView<'a> {
    type Target = CallView<'a>;
    fn deref(&self) -> &Self::Target { &self.call }
}
impl<'a> RunView<'a> {
    pub(crate) fn new(call: CallView<'a>, bindings: &'a crate::program::BoundCall<'a>) -> Self { Self { call, bindings } }
    pub(crate) fn source(&self, position: usize) -> Result<Cow<'_, Value>, Error> { self.bindings.source(position) }
    pub(crate) fn value(&self, position: usize, read: ReadKind) -> Result<Cow<'_, Value>, Error> { self.bindings.value(position, read) }
    pub(crate) fn implicit(&self, input: crate::program::ImplicitInput) -> Option<(usize, &Value)> {
        let position = self.layout().implicit_input(input)?;
        Some((position, self.bindings.parameter(position)?))
    }
    pub(crate) fn root_value(&self, root: Root, read: ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        let layout = self.layout();
        let root = layout.root(root);
        let Some(trees) = self.trees else { return self.value(root.leaves.start, read) };
        let value = trees.read_root(&root, |position| {
            if layout.read_kind(self.roles, position) == Some(ReadKind::Description) {
                self.source(position).map(Cow::into_owned)
            } else { self.value(position, read).map(Cow::into_owned) }
        })?;
        value.read(read).map(Cow::Owned)
    }
    pub(crate) fn logical_origin(&self) -> crate::error::Result<Origin<Vec<Value>>> {
        let values = (0..self.len()).map(|position| self.source(position).map(Cow::into_owned)).collect::<crate::error::Result<Vec<_>>>()?;
        Ok(self.origin(values))
    }
}
struct CellBindings<'a> { bindings: &'a dyn CallBindings, frame: usize, inputs: &'a [CellInput] }
impl CallBindings for CellBindings<'_> {
    fn len(&self) -> usize { self.bindings.len() }
    fn parameter(&self, position: usize) -> Option<&Value> { self.bindings.parameter(position) }
    fn function(&self, position: usize) -> Option<&Function> { self.bindings.function(position) }
    fn constant(&self, position: usize) -> bool { self.bindings.constant(position) }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        let desc = self.bindings.desc(position)?;
        let Ok(input) = self.inputs.binary_search_by_key(&position, |input| input.position as usize) else { return Some(desc) };
        let mut desc = desc.into_owned();
        if self.frame > desc.shape().len() { return None; }
        desc.layout = desc.layout.axes(self.frame..desc.shape().len());
        desc.atom = self.inputs[input].atom;
        desc.kind.weak = self.inputs[input].weak;
        Some(Cow::Owned(desc))
    }
}

pub(crate) struct PreparedCall<'a> { origin: &'a Origin<&'a [Value]>, values: Cow<'a, [Value]> }
impl PreparedCall<'_> {
    pub(crate) fn view(&self) -> CallView<'_> { CallView::new(self.origin, &self.values) }
    pub(crate) fn origin(&self) -> Origin<&[Value]> { self.origin.with_list(self.values.as_ref()) }
}
impl<'a> Origin<crate::program::BoundCall<'a>> {
    pub(crate) fn call_view(&self) -> CallView<'_> { CallView::new(self, &self.list) }
}
impl Operation {
    pub(crate) fn prepare<'a>(origin: &'a Origin<&'a [Value]>) -> Result<PreparedCall<'a>, Error> {
        let mut values = Cow::Borrowed(origin.list);
        for (position, read) in origin.parameter_reads(origin.list.len()) {
            let value = values[position].read(read)?;
            values.to_mut()[position] = value;
        }
        Ok(PreparedCall { origin, values })
    }
    pub(crate) fn roles_of<L: CallBindings>(origin: &Origin<L>) -> Result<Roles, Error> {
        Self::roles(&CallView::new(origin, &origin.list))
    }
}
