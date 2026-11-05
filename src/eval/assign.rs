//! Assignment: plain, destructuring, modified and selective.

use super::tree::{Entity, Facts, Last, Layer, Split, Target, Term, To, narrow};
use super::*;
use crate::{array::Layout, keyed::Selector, primitive::{coordinate_fields, item_at}};
use rustymath::error::Kind;

struct ArrayBinding { place: Place, value: Value }

impl Session {
    /// Whether `nodes` is one name that can hold an array, or brackets of such names, as in `[a b]←`. `log` keeps what it reads about what names hold (`tree::Fact`).
    pub(super) fn assignment_names(&self, nodes: &[Node], log: &mut Facts) -> Result<bool, Error> {
        let [node] = nodes else { return Ok(false) };
        Ok(match &node.kind {
            NodeKind::Name(name, _) => {
                (self.bindings.current().is_some() && name.implicit().is_none())
                    || !matches!(self.path_category(*name, &[], log)?, Category::Function | Category::Operator | Category::DyadicOperator)
            }
            NodeKind::Group(inner) => self.assignment_names(inner, log)?,
            _ => crate::syntax::plain_target(nodes),
        })
    }

    /// The category of `name`'s binding, or of the field that `keys` reach from the array it holds, as its role gives it (`Session::path_role`, which logs the fact), read without evaluating anything. An array that holds a function, an unbound name and an absent `⍺` count as arrays.
    pub(super) fn path_category(&self, name: Name, keys: &[Arc<str>], log: &mut Facts) -> Result<Category, Error> {
        let role = self.path_role(name, keys, log)?;
        Ok(match role.0 { Category::Callable | Category::Absent => Category::Value, category => category })
    }

    /// Inside a dfn, `⍵` is a local name bound to the argument. The other argument and operand names can't be assigned.
    fn assignable(&self, name: Name) -> bool { name.implicit().is_none() || name == Name::OMEGA && self.bindings.current().is_some() }
    fn store(&self, name: Name, value: Binding, span: &Span) -> Result<(), Error> {
        if !self.assignable(name) { return Err(Error::new(ErrorKind::Syntax, "arguments and operands cannot be assigned here").at(span)); }
        self.store_at(self.bindings.here(name), value, span)
    }

    fn array_binding(&self, name: Name, span: &Span) -> Result<ArrayBinding, Error> {
        if !self.assignable(name) { return Err(Error::new(ErrorKind::Syntax, "arguments and operands cannot be assigned here").at(span)); }
        let Some((place, Binding::Value(value))) = self.bindings.lookup(name)? else {
            return Err(Error::new(ErrorKind::Value, "assignment target must be an existing array").at(span));
        };
        Ok(ArrayBinding { place, value })
    }

    fn update_array(&self, binding: &ArrayBinding, value: Value, span: &Span) -> Result<(), Error> {
        self.store_at(binding.place, Binding::from_element(value), span)
    }
    pub(super) fn store_at(&self, place: Place, value: Binding, span: &Span) -> Result<(), Error> {
        let value = match value { Binding::Function(FunctionBinding::Rebuild { .. }) => Function::from_value(value, span)?.into(), value => value };
        if value.environment() > place.frame() { return Err(Error::domain("value would export a local closure").at(span)); }
        if place.frame() != self.bindings.current() {
            if self.bindings.get(place)?.is_some_and(|old| Category::bound(&old) != Category::bound(&value)) {
                return Err(Error::new(
                    ErrorKind::Syntax, "an assignment can't change a name outside its function between an array, a function and an operator").at(span));
            }
        }
        self.writing(place, span)?;
        self.bindings.insert(place, value);
        Ok(())
    }

    /// Stores a plain assignment and returns its right argument.
    #[inline]
    pub(super) fn assigned(&self, target: &Target, value: Option<Binding>, local: Option<u32>, span: &Span) -> Result<Binding, Error> {
        let Some(value) = value else { return Err(Error::new(ErrorKind::Syntax, NO_TARGET).at(span)) };
        match local { Some(slot) => self.store_at(self.bindings.local(slot), value.clone(), span)?, None => self.assign(target, None, &value)? }
        Ok(value)
    }

    /// A modified assignment, kept apart from `assigned` so that a plain assignment passes no function: `f` updates the target with `value` as its right argument, or with no value, as `a(f)←` does, sets `a` to `f a`. Gives the assignment's value, which is no result for an update.
    pub(super) fn modified(&self, target: &Target, f: Function, value: Option<Binding>) -> Result<Binding, Error> {
        let Some(value) = value else {
            // The update runs as the dyadic form with `f∘⊣`, which ignores its right argument of 0.
            let f = atop(f, Function::primitive(Primitive::Identity(true)), &target.at)?;
            self.modify(target, &target.to, f, &Binding::Value(Value::integer(0)))?;
            return Ok(Binding::NoResult);
        };
        self.assign(target, Some(f), &value)?;
        Ok(value)
    }

    /// Applies the compiled target, with a modified assignment's function where supplied.
    pub(super) fn assign(&self, target: &Target, modifier: Option<Function>, value: &Binding) -> Result<(), Error> {
        match modifier {
            Some(f) => self.modify(target, &target.to, f, value), None => self.assign_to(target, &target.to, value)
        }.map_err(|error| error.at(&target.span))
    }

    /// Shows `value` as explicit output (`⎕←`), laid out as output inside a function is where `function` holds. A replay of an output effect shows it so too (`program::Act::run`).
    pub(crate) fn explicit_output(&self, value: &crate::Value, function: bool) {
        self.execution.output(crate::OutputKind::Explicit, self.display_settings().explicit(value, function));
    }

    fn assign_to(&self, target: &Target, to: &To, value: &Binding) -> Result<(), Error> {
        match to {
            To::Name(name, span) => self.store(*name, value.clone(), span),
            To::Output(span) => match value {
                Binding::Value(a) => Operation::Effect(crate::primitive::Act::Output).call(&[a.clone()]).map(|_| ()).map_err(|error| error.at(span)),
                _ => Err(Error::domain("output requires a subject").at(span)),
            },
            To::Names(cells, span) => {
                let right = value.clone().into_value(span)?;
                for (cell, item) in Self::destructure(cells, &right, span)?.into_iter().rev() { self.assign_to(target, cell, &Binding::from_element(item))?; }
                Ok(())
            }
            To::Rule(name, derivative, span) => self.attach_rule(*name, *derivative, value, span),
            To::Selected(layer) => self.selected(layer, &mut vec![None; target.slots], None, value),
            To::Invalid(e) => Err(e.clone()),
        }
    }

    /// The name in a target that attaches a hand-written rule to the function it holds, with whether the rule is its derivative: `f∂` for the derivative, or `f⁻¹` for the inverse.
    pub(super) fn rule_target(&self, nodes: &[Node], log: &mut Facts) -> Result<Option<(Name, bool)>, Error> {
        let [Node { kind: NodeKind::Name(name, _), .. }, Node { kind: NodeKind::Operator(op), .. }] = nodes else { return Ok(None) };
        let derivative = match op { OperatorKind::Derivative => true, OperatorKind::Inverse => false, _ => return Ok(None) };
        Ok((self.path_category(*name, &[], log)? == Category::Function).then_some((*name, derivative)))
    }

    /// `f⁻¹←g` or `f∂←g`: the nearest binding of `f` gets `g` attached as its inverse or derivative.
    fn attach_rule(&self, name: Name, derivative: bool, value: &Binding, span: &Span) -> Result<(), Error> {
        if !self.assignable(name) { return Err(Error::new(ErrorKind::Syntax, "arguments and operands cannot be assigned here").at(span)); }
        let g = Function::from_value(value.clone(), span)?;
        let (place, binding) = self.bindings.lookup(name)?.expect("rule_target checked the binding");
        let f = Function::from_value(binding, span)?;
        let f = if derivative { f.with_derivative(g) } else { f.with_inverse(g) };
        self.store_at(place, f.into(), span)
    }

    /// Updates a modified assignment's target `to` with `modifier`, taking `value` as its right argument: names update the arrays they hold, and a selection writes through `write_modified`.
    fn modify(&self, target: &Target, to: &To, modifier: Function, value: &Binding) -> Result<(), Error> {
        match to {
            To::Selected(layer) => self.selected(layer, &mut vec![None; target.slots], Some(modifier), value), To::Invalid(e) => Err(e.clone()),
            names => self.modify_names(names, &value.clone().into_value(&target.at)?, &modifier)
        }
    }

    fn modify_names(&self, to: &To, right: &Value, modifier: &Function) -> Result<(), Error> {
        match to {
            To::Name(name, span) => {
                let binding = self.array_binding(*name, span)?;
                self.update_array(&binding, modifier.call_array(Some(&binding.value), right, span)?, span)
            }
            To::Names(cells, span) => { for (cell, item) in Self::destructure(cells, right, span)? { self.modify_names(cell, &item, modifier)?; } Ok(()) }
            To::Invalid(error) => Err(error.clone()),
            _ => unreachable!("a modified target is a name or a destructuring pattern"),
        }
    }

    /// The cells of a destructuring target, each with the nodes it assigns and, in a pattern, the key it reads. A pattern cell `key:target` reads the item with that key, spelled: a name stands for itself and a string literal for its text, never for a value. `key:` assigns the name `key`.
    pub(super) fn targets<'a>(&self, cells: &'a [Vec<Node>], record: bool, log: &mut Facts) -> Result<Vec<(Option<Arc<str>>, &'a [Node])>, Error> {
        if !record { return Ok(cells.iter().map(|cell| (None, &cell[..])).collect()); }
        cells.iter().map(|cell| {
            let colon = crate::syntax::key_colon(cell)
                .ok_or_else(|| Error::new(ErrorKind::Syntax, "each cell of a pattern has a key, as in [a:] or [key:a]").at(&cell[0].span))?;
            let key_error = || Error::new(ErrorKind::Syntax, "a pattern's key is a name, or a string before a target").at(&cell[0].span);
            let key: Arc<str> = match &cell[..colon] {
                [Node { kind: NodeKind::Name(name, _), .. }] => name.text().into(),
                [Node { kind: NodeKind::Literal(text), .. }] if colon + 1 < cell.len() => crate::keyed::name(text).ok_or_else(key_error)?,
                _ => return Err(key_error()),
            };
            let target = crate::syntax::pattern_target(cell, colon);
            if !(matches!(target, [Node { kind: NodeKind::Name(_, _), .. }]) || self.assignment_names(target, log)?) {
                return Err(Error::new(ErrorKind::Syntax, "a pattern assigns to a name or to brackets of names").at(&target[0].span));
            }
            Ok((Some(key), target))
        }).collect()
    }

    /// The item of `right` that each target in `cells` takes: by key in a pattern, and otherwise by position, whatever keys `right` has. One item fills every positional target. A vector's part is its item, and any other array's part is a major cell (`Cells::get`).
    fn destructure<'a>(cells: &'a [(Option<Arc<str>>, To, Span)], right: &Value, span: &Span) -> Result<Vec<(&'a To, Value)>, Error> {
        let right = right.exposed()?;
        let positional = cells.iter().all(|(key, ..)| key.is_none());
        if positional && right.is_singleton() {
            let item = item_at(&right, &[0], None)?;
            return Ok(cells.iter().map(|(_, target, _)| (target, item.clone())).collect());
        }
        let keys = if positional { None } else {
            Some(right.keys(0).ok_or_else(|| Error::domain("a pattern reads items by key, and these items have none").at(span))?)
        };
        let items = right.cells(right.shape().len().saturating_sub(1)).described("array exceeds limits").at(span)?;
        if positional && items.len() != cells.len() { return Err(Error::new(Kind::Length, "destructuring needs one item for each name").at(span)); }
        cells.iter().enumerate().map(|(i, (key, target, at))| {
            let i = match keys {
                Some(keys) => {
                    let key = key.as_ref().expect("every cell of a pattern has a key");
                    keys.position(key).ok_or_else(|| Error::new(ErrorKind::Value, format!("no item has the key {key}")).at(at))?
                }
                None => i,
            };
            let part = if items.shape().is_empty() { item_at(&right, &[i], None) } else { items.get(i) };
            Ok((target, part.described("array exceeds limits").at(span)?))
        }).collect()
    }

    /// Builds missing keyed containers. Each index expression runs once.
    fn extend(&self, layer: &Layer, run: &mut [Option<Value>], descend: bool) -> Result<(), Error> {
        let Some(split) = layer.split.as_ref().filter(|split| split.keyed) else { return Ok(()) };
        let value = self.index(split, run)?.read(crate::ReadKind::Value)?;
        if !Selector::may_hold_names(&value) { return Ok(()); }
        let pick = split.p == Primitive::Mix;
        let items = if pick { coordinate_fields(&value).collect() } else { vec![value.clone()] };
        if !items.iter().filter(|value| Selector::may_hold_names(value)).flat_map(coordinate_fields).any(|s| matches!(Selector::of(&s), Ok(Some(_)))) { return Ok(()); }
        let container = &split.container;
        self.extend(container, run, true)?;
        let mut target = self.read(container, run)?;
        let changed = if pick { crate::primitive::extend_path(&items, &mut target, descend, true)? }
        else { crate::primitive::extend_indices(&items[0], &mut target, descend, true)? };
        if changed { self.selected(container, run, None, &Binding::Value(target))?; }
        Ok(())
    }

    /// The `←` of an assignment on the path from a selection target to its root array, as in `(U←T).[k]←v`. The root is the last node at each level.
    pub(super) fn root_assignment(nodes: &[Node]) -> Option<Span> {
        if let Some(node) = nodes.iter().find(|n| matches!(n.kind, NodeKind::Assign)) { return Some(node.span.clone()); }
        match &nodes.last()?.kind { NodeKind::Group(inner) | NodeKind::Run(inner) => Self::root_assignment(inner), _ => None }
    }

    /// Walks the selected graph back to the root and builds its At update.
    fn selected(&self, layer: &Layer, run: &mut [Option<Value>], modifier: Option<Function>, value: &Binding) -> Result<(), Error> {
        let right = value.clone().into_value(&layer.at)?;
        if modifier.is_none() { self.extend(layer, run, false)?; }
        let binding = self.root(layer)?;
        if modifier.is_none() && layer.prefix.is_none() && layer.split.is_none() && matches!(layer.last, Last::Name(..)) {
            return self.update_array(&binding, right, &layer.span);
        }
        let scope = crate::selection::Scope::new(&binding.value)?;
        let target = self.fold(layer, run, scope.root.clone())?;
        let walk = scope.select(&target)?.ok_or_else(|| Error::domain("cannot assign to an item that isn't from the original").at(&layer.span))?;
        let updated = match modifier {
            None => walk.write(right, None)?,
            Some(f) => {
                let arguments = [&target, &right].map(|value| Desc::of(value).map(|desc| Desc::new(desc.kind, true, Layout::EMPTY)));
                let origin = f.origin(&[target.clone(), right.clone()])?;
                match f.primitive_body(&origin.reborrow(), &arguments)? {
                    Some(primitive) if primitive.info().pervasive(2) => walk.write(right, Some(primitive))?,
                    _ => walk.modified(right, &f, &layer.span)?,
                }
            }
        };
        self.update_array(&binding, updated, &layer.span)
    }

    /// The binding of the array that `layer` ends in, its root.
    fn root(&self, layer: &Layer) -> Result<ArrayBinding, Error> {
        match &layer.last {
            Last::Name(name, span) => self.array_binding(*name, span),
            Last::Container => self.root(&layer.split.as_ref().expect("a split holds the container").container), Last::Layer(inner) => self.root(inner),
            Last::Invalid(e) => Err(e.clone())
        }
    }

    /// What `layer`'s code gives from `root`, the root array's value: through a split that the run takes, where `p` with the index value applies to the container's value (`Split::unit`), and otherwise the code before its last node, run on what the last node gives.
    fn fold(&self, layer: &Layer, run: &mut [Option<Value>], root: Value) -> Result<Value, Error> {
        if let Some(split) = layer.split.as_ref().filter(|split| split.unit && split.taken(run)) {
            let value = self.fold(&split.container, run, root)?;
            let index = self.index(split, run)?;
            return Function::primitive(split.p).call_array(Some(&index), &value, &split.p_at);
        }
        let value = match &layer.last {
            Last::Name(..) => root, Last::Container => self.fold(&layer.split.as_ref().expect("a split holds the container").container, run, root)?,
            Last::Layer(inner) => self.fold(inner, run, root)?, Last::Invalid(_) => unreachable!("`root` raises the error before the walk")
        };
        let Some(prefix) = &layer.prefix else { return Ok(value) };
        let seed = Entity::new(Term::Binding(Binding::Value(value)), narrow(&layer.last_at.range), false, None);
        let Step::Done(bound) = prefix.as_ref().map_err(Clone::clone)?.run_seeded(self, seed)? else { unreachable!("a target's code isn't in tail position") };
        bound.value.into_value(&layer.last_at)
    }

    /// What `layer` gives as an expression: its code run on its root array's value.
    fn read(&self, layer: &Layer, run: &mut [Option<Value>]) -> Result<Value, Error> { let root = self.root(layer)?.value; self.fold(layer, run, root) }

    /// The value of `split`'s index, evaluated once in a run, one level deeper, and kept at its slot.
    fn index(&self, split: &Split, run: &mut [Option<Value>]) -> Result<Value, Error> {
        if let Some(value) = &run[split.slot] { return Ok(value.clone()); }
        let index = split.index.as_ref().map_err(Clone::clone)?;
        self.descend(&split.at)?;
        let ran = index.run(self, false);
        self.ascend();
        let Step::Done(bound) = ran? else { unreachable!("an index isn't in tail position") };
        let value = bound.array(&split.at)?;
        run[split.slot] = Some(value.clone());
        Ok(value)
    }
}
