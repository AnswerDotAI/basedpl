//! Assignment: plain, destructuring, modified and selective.

use rustymath::error::Kind;
use super::*;

impl Session {
    /// Whether `nodes` is one name that can hold an array, or brackets of such names, as in `[a b]←`.
    fn assignment_names(&self, nodes: &[Node]) -> bool {
        let [node] = nodes else { return false };
        match &node.kind {
            NodeKind::Name(name) => {
                (self.current.is_some() && !implicit_name(name)) || !matches!(self.lookup(name), Some(Binding::Function(_) | Binding::Operator(_)))
            }
            NodeKind::Group(inner) => self.assignment_names(inner),
            // Inside brackets every name is a target, whatever it holds. A pattern's cells each have a key, which `targets` checks.
            NodeKind::ArrayLiteral { cells, form: ListForm::Items | ListForm::Rows, record } => cells.iter().all(|c| {
                if *record { crate::syntax::key_colon(c).is_some() } else { matches!(&c[..], [Node { kind: NodeKind::Name(_), .. }]) || self.assignment_names(c) }
            }),
            _ => false,
        }
    }

    /// Whether a node holds an array.
    pub(super) fn holds_array(&self, node: &Node) -> bool { matches!(self.node_category(node), Category::Value) }

    pub(super) fn node_category(&self, node: &Node) -> Category {
        use Category::*;
        match &node.kind {
            NodeKind::Function(_) => Function,
            NodeKind::Operator(op) => Category::of(&Binding::Operator(self::OperatorNode::Primitive(*op))),
            NodeKind::Name(name) => self.lookup(name).map_or(Value, Category::of),
            NodeKind::System(_) => Function,
            NodeKind::Dfn(d) => match d.kind {
                DefinitionKind::Function => Function, DefinitionKind::MonadicOperator => Operator, DefinitionKind::DyadicOperator => DyadicOperator
            },
            // A group or run has the category of its expression, so `(M←-)` and `(⌽@ 1⌷)` are functions.
            NodeKind::Group(_) | NodeKind::Run(_) => self.assignment_operand(std::slice::from_ref(node), 1).map_or(NoResult, |(_, c)| c),
            _ => Value,
        }
    }

    fn assignment_operand(&self, nodes: &[Node], end: usize) -> Result<(usize, Category), Error> {
        use Category::*;
        if end == 0 { return Err(nodes[0].span.error(ErrorKind::Syntax, "missing assignment operand")); }
        let mut start = end - 1;
        let mut category = match &nodes[start].kind {
            NodeKind::Group(inner) | NodeKind::Run(inner) => {
                if inner.is_empty() { return Err(nodes[start].span.error(ErrorKind::Syntax, "empty assignment operand")); }
                // Rewrite the group's own dot access first, so that `(x.a).b` classifies `(x.a)` as a value.
                let inner = self.members(inner);
                self.assignment_operand(&inner, inner.len())?.1
            }
            _ => self.node_category(&nodes[start]),
        };
        if matches!(category, Operator) && start > 0 {
            let superscript = match &nodes[start].kind { NodeKind::Operator(OperatorKind::Super(s)) => Some(*s), _ => None };
            let (operand, operand_category) = self.assignment_operand(nodes, start)?;
            start = operand;
            // `ᵘ` always gives an array, and the other superscripts give one on an array.
            category = match superscript { Some(Superscript::Unit) => Value, Some(_) if matches!(operand_category, Value) => Value, _ => Function };
        }
        if start > 0 && matches!(&nodes[start - 1].kind, NodeKind::Run(inner) if self.ends_in_dyadic_operator(inner)) {
            // A run that ends in a dyadic operator holds its left operand, and takes this operand as its right one, as in `(⌽@ 1⌷)`.
            start -= 1;
            category = Function;
        } else if start > 0 && matches!(Rule::get(self.node_category(&nodes[start - 1]), category), Rule::BindRight) {
            start = self.assignment_operand(nodes, start - 1)?.0;
            category = Function;
        }
        Ok((start, category))
    }

    /// The target of `←` is the run before it: an array, followed by the function of a modified assignment such as `x+←1`.
    pub(super) fn assignment_start(&self, nodes: &[Node]) -> Result<usize, Error> {
        if self.rule_target(nodes).is_some() { return Ok(0); }
        let end = nodes.len();
        let (operand, category) = self.assignment_operand(nodes, end)?;
        // Inside a dfn a name before `←` is a target, unless an array comes directly before it. Then the name's value decides, as at top level: a function makes a modified assignment, as in `a(f)←3` and `(a)f←3`.
        let modified =
            operand > 0 && matches!(category, Category::Function) && (self.holds_array(&nodes[operand - 1]) || !self.assignment_names(&nodes[operand..]));
        let start = if modified { operand - 1 } else { end - 1 };
        if start > 0 && self.holds_array(&nodes[start]) && self.holds_array(&nodes[start - 1]) {
            let span = Span { source: nodes[start].span.source.clone(), range: nodes[start - 1].span.range.start..nodes[start].span.range.end };
            return Err(span.error(ErrorKind::Syntax, STRAND_TARGET));
        }
        Ok(start)
    }

    /// Inside a dfn, `⍵` is a local name bound to the argument. The other argument and operand names can't be assigned.
    fn assignable(&self, name: &str) -> bool { !implicit_name(name) || name == "⍵" && self.current.is_some() }
    fn store(&mut self, name: &str, value: Binding, span: &Span) -> Result<(), Error> {
        if !self.assignable(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        self.insert_name(self.current, name, value);
        Ok(())
    }

    fn array_binding(&self, name: &str, span: &Span) -> Result<ArrayBinding, Error> {
        if !self.assignable(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let Some((owner, Binding::Value(value))) = self.binding(name) else {
            return Err(span.error(ErrorKind::Value, "assignment target must be an existing array"));
        };
        Ok(ArrayBinding { name: name.to_owned(), owner, value: value.clone() })
    }

    fn update_array(&mut self, binding: &ArrayBinding, value: Value, span: &Span) -> Result<(), Error> {
        if value.environment() > binding.owner { return Err(span.domain_error("array would export a local closure")); }
        if self.differentiating() { self.record_write(binding.owner, &binding.name); }
        self.insert_name(binding.owner, &binding.name, Binding::from_element(value));
        Ok(())
    }

    /// Assigns `value` to `target`. `fusion_report` reports an assignment that computed a pending value, as display and destructuring do, as `←`.
    pub(super) fn assign(&mut self, target: &[Node], value: &Binding) -> Result<(), Error> {
        let watch = match value { Binding::Value(v) => self.watch(v), _ => None };
        self.assign_to(target, value)?;
        self.report_assigned(&target[0].span, watch);
        Ok(())
    }

    /// The count of items that pending arrays have computed, taken before an assignment that would compute `value`, a pending array, where `fusion_report` would report it.
    fn watch(&self, value: &Value) -> Option<usize> { ((self.explaining || self.display.fusion) && value.pending().is_some()).then(crate::explain::computed_so_far) }

    /// Reports, as `←`, the items that pending arrays computed since `watch` counted them.
    fn report_assigned(&mut self, span: &Span, watch: Option<usize>) {
        let Some(before) = watch else { return };
        let items = crate::explain::computed_so_far() - before;
        if items > 0 { fusion_report(&mut self.at(span), "fusion ends", "←", items); }
    }

    fn assign_to(&mut self, target: &[Node], value: &Binding) -> Result<(), Error> {
        let span = &target[0].span;
        if let [target] = target {
            match &target.kind {
                NodeKind::Name(name) => { return self.store(name, value.clone(), span); }
                NodeKind::System(_) => { self.resolve(target)?; return Err(span.error(ErrorKind::Syntax, "system names are read-only")); }
                NodeKind::Output => {
                    return match value {
                        Binding::Value(a) => {
                            self.execution.output(crate::OutputKind::Explicit, self.display_settings().explicit(a, self.current.is_some()));
                            Ok(())
                        }
                        _ => return Err(target.span.domain_error("output requires a subject")),
                    }
                }
                NodeKind::Group(nodes) => {
                    if self.assignment_names(nodes) { return self.assign_to(nodes, value); }
                    return self.assign_selected(nodes, None, value);
                }
                NodeKind::ArrayLiteral { cells, record, .. } if self.assignment_names(std::slice::from_ref(target)) => {
                    let right = value.clone().into_value(span)?;
                    let pairs = Self::destructure(self.targets(cells, *record)?, &right, span)?;
                    for (cell, item) in pairs.into_iter().rev() { self.assign_to(cell, &Binding::from_element(item))?; }
                    return Ok(());
                }
                _ => (),
            }
        }
        if let Some((name, derivative)) = self.rule_target(target) { return self.attach_rule(name, derivative, value, span); }
        let Some((array, modifier)) = self.modified_target(target)? else { return Err(span.error(ErrorKind::Syntax, "assignment needs a name or selection")); };
        self.modify(array, modifier, value)
    }

    /// The name in a target that attaches a hand-written rule to the function it holds, with whether the rule is its derivative: `f∂` for the derivative, or `f⁻¹` for the inverse.
    fn rule_target<'a>(&self, nodes: &'a [Node]) -> Option<(&'a str, bool)> {
        let [Node { kind: NodeKind::Name(name), .. }, Node { kind: NodeKind::Operator(op), .. }] = nodes else { return None };
        let derivative = match op { OperatorKind::Differentiate => true, OperatorKind::Super(Superscript::Power(-1)) => false, _ => return None };
        matches!(self.lookup(name), Some(Binding::Function(_))).then_some((name.as_str(), derivative))
    }

    /// `f⁻¹←g` or `f∂←g`: the nearest binding of `f` gets `g` attached as its inverse or derivative.
    fn attach_rule(&mut self, name: &str, derivative: bool, value: &Binding, span: &Span) -> Result<(), Error> {
        if !self.assignable(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let Binding::Function(g) = value else { return Err(span.domain_error("an inverse or derivative must be a function")) };
        let Some((owner, Binding::Function(f))) = self.binding(name) else { unreachable!("`rule_target` checked the binding") };
        if g.environment() > owner { return Err(span.domain_error("function would export a local closure")); }
        let f = if derivative { f.with_derivative(g.clone()) } else { f.with_inverse(g.clone()) };
        self.insert_name(owner, name, Binding::Function(f));
        Ok(())
    }

    /// Splits a modified assignment's target into its array part and its function, as `a` and `f` in `a(f)←`.
    fn modified_target<'a>(&mut self, target: &'a [Node]) -> Result<Option<(&'a [Node], Function)>, Error> {
        let (operand, category) = self.assignment_operand(target, target.len())?;
        if operand == 0 || !matches!(category, Category::Function) { return Ok(None); }
        Ok(self.modifier(&target[operand..])?.map(|f| (&target[..operand], f)))
    }

    /// Updates the array part of a modified assignment's target with `modifier`, taking `value` as its right argument.
    fn modify(&mut self, array: &[Node], modifier: Function, value: &Binding) -> Result<(), Error> {
        let span = &array[0].span;
        if self.assignment_names(array) { return self.modify_names(array, &value.clone().into_value(span)?, &modifier); }
        match array {
            [Node { kind: NodeKind::Group(nodes), .. }] => self.assign_selected(nodes, Some(modifier), value),
            _ => Err(span.error(ErrorKind::Syntax, "assignment needs a name or selection")),
        }
    }

    /// `a(f)←` with no value sets `a` to `f a`, and gives whether `target` has that form. It runs as the dyadic form with `f∘⊣`, which ignores its right argument of 0.
    pub(super) fn modify_monadic(&mut self, target: &[Node]) -> Result<bool, Error> {
        let Some((array, f)) = self.modified_target(target)? else { return Ok(false) };
        let f = atop(f, Function::primitive(Primitive::Identity(true)), &target[0].span)?;
        self.modify(array, f, &Binding::Value(Value::integer(0)))?;
        Ok(true)
    }

    fn modify_names(&mut self, nodes: &[Node], right: &Value, modifier: &Function) -> Result<(), Error> {
        let [node] = nodes else { unreachable!() };
        match &node.kind {
            NodeKind::Group(inner) => self.modify_names(inner, right, modifier),
            NodeKind::Name(name) => {
                let binding = self.array_binding(name, &node.span)?;
                // Joining along the leading axis grows the array in place.
                if let Some(p @ (Primitive::Ravel | Primitive::CatenateFirst)) = modifier.as_primitive() {
                    let first = matches!(p, Primitive::CatenateFirst);
                    if let Some((cells, names)) = crate::primitive::append_plan(&binding.value, right, first, &self.at(&node.span))? {
                        if cells.environment() > binding.owner { return Err(node.span.domain_error("array would export a local closure")); }
                        return self.write_kept(binding, &node.span, |value, _| {
                            value.append(&cells, names.as_deref()).error_at(&node.span, "catenate has duplicate axis keys")
                        });
                    }
                }
                let updated = modifier.call_array(Some(&binding.value), right, &mut self.at(&node.span))?;
                self.update_array(&binding, updated, &node.span)
            }
            NodeKind::ArrayLiteral { cells, record, .. } => {
                for (cell, item) in Self::destructure(self.targets(cells, *record)?, right, &node.span)? { self.modify_names(cell, &item, modifier)?; }
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    /// The cells of a destructuring target, each with the nodes it assigns and, in a pattern, the key it reads. A pattern cell `key:target` reads the item with that key, spelled: a name stands for itself and a string literal for its text, never for a value. `key:` assigns the name `key`.
    fn targets<'a>(&self, cells: &'a [Vec<Node>], record: bool) -> Result<Vec<(Option<Arc<str>>, &'a [Node])>, Error> {
        if !record { return Ok(cells.iter().map(|cell| (None, &cell[..])).collect()); }
        cells.iter().map(|cell| {
            let colon = crate::syntax::key_colon(cell).ok_or_else(|| cell[0].span.error(ErrorKind::Syntax, "each cell of a pattern has a key, as in [a:] or [key:a]"))?;
            let key_error = || cell[0].span.error(ErrorKind::Syntax, "a pattern's key is a name, or a string before a target");
            let key: Arc<str> = match &cell[..colon] {
                [Node { kind: NodeKind::Name(name), .. }] => name.as_str().into(),
                [Node { kind: NodeKind::Literal(text), .. }] if colon + 1 < cell.len() => crate::keyed::name(text).ok_or_else(key_error)?,
                _ => return Err(key_error()),
            };
            let target = if colon + 1 == cell.len() { &cell[..colon] } else { &cell[colon + 1..] };
            if !(matches!(target, [Node { kind: NodeKind::Name(_), .. }]) || self.assignment_names(target)) {
                return Err(target[0].span.error(ErrorKind::Syntax, "a pattern assigns to a name or to brackets of names"));
            }
            Ok((Some(key), target))
        }).collect()
    }

    /// The item of `right` that each target takes: by key in a pattern, and otherwise by position, whatever keys `right` has. One item fills every positional target.
    fn destructure<'a>(targets: Vec<(Option<Arc<str>>, &'a [Node])>, right: &Value, span: &Span) -> Result<Vec<(&'a [Node], Value)>, Error> {
        let parts = || -> Result<Vec<Value>, Error> {
            if right.shape().len() == 1 { Ok(right.elements().collect()) } else { right.major_cells().error_at(span, "array exceeds limits") }
        };
        if targets.iter().all(|(key, _)| key.is_none()) {
            if right.is_singleton() { return Ok(targets.into_iter().map(|(_, target)| (target, right.at(0))).collect()); }
            let parts = parts()?;
            if parts.len() != targets.len() { return Err(span.error(Kind::Length, "destructuring needs one item for each name")); }
            return Ok(targets.into_iter().zip(parts).map(|((_, target), part)| (target, part)).collect());
        }
        let keys = right.keys(0).ok_or_else(|| span.domain_error("a pattern reads items by key, and these items have none"))?;
        let parts = parts()?;
        targets.into_iter().map(|(key, target)| {
            let key = key.expect("every cell of a pattern has a key");
            let i = keys.position(&key).ok_or_else(|| target[0].span.error(ErrorKind::Value, format!("no item has the key {key}")))?;
            Ok((target, parts[i].clone()))
        }).collect()
    }

    fn modifier(&mut self, nodes: &[Node]) -> Result<Option<Function>, Error> {
        if nodes.is_empty() { return Ok(None); }
        match self.bind(nodes)?.value {
            Binding::Function(f) => Ok(Some(f)), _ => Err(nodes[0].span.error(ErrorKind::Syntax, "modified assignment needs a function"))
        }
    }

    fn extend_selected(&mut self, nodes: &mut Vec<Node>, descend: bool) -> Result<(), Error> {
        if let [Node { kind: NodeKind::Group(inner), .. }] = nodes.as_mut_slice() { return self.extend_selected(inner, descend); }
        // A key before `⊃`, `⌷` or `.[]` may name an entry that the container lacks. It can be a list of keys.
        let Some(i) = nodes.iter().position(|n| matches!(n.kind, NodeKind::Function(Primitive::Mix | Primitive::Index | Primitive::Bracket))) else { return Ok(()) };
        if i == 0 || i + 1 == nodes.len() || !nodes[..i].iter().all(crate::syntax::key_node) { return Ok(()); }
        let span = nodes[0].span.clone();
        let value = self.array_result(&nodes[..i])?;
        // `⌷` and `.[]` select in one level. A path's items go down level by level.
        let items = if matches!(nodes[i].kind, NodeKind::Function(Primitive::Mix)) { crate::primitive::coordinate_fields(&value).collect() } else { vec![value.clone()] };
        // Replace the keys with their value, so that they are evaluated once.
        nodes.splice(..i, [Node { kind: NodeKind::Literal(value), span: span.clone() }]);
        if !items.iter().flat_map(crate::primitive::coordinate_fields).any(|s| matches!(crate::keyed::Selector::of(&s), Ok(Some(_)))) { return Ok(()); }
        let mut container = nodes.split_off(2);
        self.extend_selected(&mut container, true)?;
        if let Some((binding, path)) = self.direct_item(&mut container)? {
            let mut target = crate::primitive::item_at(&binding.value, &path);
            let lacks = crate::primitive::extend_path(&items, &mut target, descend, false, &self.at(&span))?;
            // Holding the item would make the write copy it.
            drop(target);
            if lacks {
                self.write_kept(binding, &span, |value, cx| crate::primitive::extend_path(&items, value.item_mut(&path, false), descend, true, cx).map(drop))?;
            }
        } else {
            let mut target = self.array_result(&container)?;
            if crate::primitive::extend_path(&items, &mut target, descend, true, &self.at(&span))? {
                self.assign_selected(&container, None, &Binding::Value(target))?;
            }
        }
        nodes.extend(container);
        Ok(())
    }

    /// The `←` of an assignment on the path from a selection target to its root array, as in `(U←T).[k]←v`. The root is the last node at each level.
    fn root_assignment(&self, nodes: &[Node]) -> Option<Span> {
        let nodes = self.members(nodes);
        if let Some(node) = nodes.iter().find(|n| matches!(n.kind, NodeKind::Assign)) { return Some(node.span.clone()); }
        match &nodes.last()?.kind { NodeKind::Group(inner) | NodeKind::Run(inner) => self.root_assignment(inner), _ => None }
    }

    fn assign_selected(&mut self, nodes: &[Node], modifier: Option<Function>, value: &Binding) -> Result<(), Error> {
        if let Some(span) = self.root_assignment(nodes) { return Err(span.error(ErrorKind::Syntax, INVALID_SELECTION)); }
        let right = &value.clone().into_value(&nodes[0].span)?;
        let mut nodes = self.members(nodes).into_owned();
        if modifier.is_none() { self.extend_selected(&mut nodes, false)?; }
        let span = nodes[0].span.clone();
        let (binding, selection, values) = match self.direct_target(&mut nodes)? {
            Some((binding, selection)) => (binding, selection, right.clone()),
            None => {
                let (binding, labels, selected) = self.selection_expression(&nodes)?;
                let (selection, values) = labels.replacements(&selected.labels, right, selected.kind, &span)?;
                (binding, selection, values)
            }
        };
        match modifier {
            Some(f) => self.write_modified(binding, &selection, &f, &values, &span), None => self.write_selection(binding, &selection, &values, &span)
        }
    }

    /// Writes `values` at `selection` into the array that `binding` kept, then stores it under the name.
    fn write_selection(&mut self, binding: ArrayBinding, selection: &Selection, values: &Value, span: &Span) -> Result<(), Error> {
        // The items already in the array passed this check when they were stored.
        if values.environment() > binding.owner { return Err(span.domain_error("array would export a local closure")); }
        self.write_kept(binding, span, |value, context| selection.write_into(value, values, context))
    }

    /// Runs `write` on the array that `binding` kept, then stores it under the name. The name's current binding goes first, so the write happens in place unless something else holds the array. `write` makes every check before it changes anything, so an error leaves the name as it was. `fusion_report` reports a write that computes a pending array, as `←`.
    fn write_kept(
        &mut self, binding: ArrayBinding, span: &Span, write: impl FnOnce(&mut Value, &mut crate::execution::Context<'_>) -> Result<(), Error>
    ) -> Result<(), Error> {
        let ArrayBinding { name, owner, mut value } = binding;
        let watch = self.watch(&value);
        // The kept array can be written in place only when the name no longer holds it too.
        let current = self.remove_name(owner, &name);
        let same = matches!(&current, Some(Binding::Value(v)) if v.storage_id() == value.storage_id());
        let current = current.filter(|_| !same);
        if let Err(e) = write(&mut value, &mut self.at(span)) {
            if let Some(restored) = if same { Some(Binding::Value(value)) } else { current } { self.insert_name(owner, &name, restored); }
            return Err(e);
        }
        self.report_assigned(span, watch);
        self.insert_name(owner, &name, Binding::from_element(value));
        Ok(())
    }

    /// A target whose text gives its positions: `I⌷`, `.[I]` or a path `L⊃` of a name or of such a target, as dot access writes them. It gives the binding and the selection without the labels that other targets need. Each index is evaluated once and its node becomes its value, so a target that needs labels after all evaluates nothing twice.
    fn direct_target(&mut self, nodes: &mut [Node]) -> Result<Option<(ArrayBinding, Selection)>, Error> {
        let span = nodes[0].span.clone();
        match nodes {
            [Node { kind: NodeKind::Group(inner) | NodeKind::Run(inner), .. }] => self.direct_target(inner),
            [left, Node { kind: NodeKind::Function(p @ (Primitive::Index | Primitive::Bracket | Primitive::Mix)), .. }, rest @ ..] if self.holds_array(left) => {
                let Some((binding, path)) = self.direct_item(rest)? else { return Ok(None) };
                let left = self.evaluate_once(left)?;
                let item = crate::primitive::item_at(&binding.value, &path);
                let context = self.at(&span);
                let selection = match p {
                    Primitive::Mix => crate::primitive::pick_selection(&left, &item, &context)?,
                    _ => crate::primitive::squad_selection(&left, &item, matches!(p, Primitive::Bracket), &context)?,
                };
                Ok(Some((binding, selection.within(&path))))
            }
            _ => Ok(None),
        }
    }

    /// The array that `nodes` names, or the one whole item that a direct target reaches: its binding and its path.
    fn direct_item(&mut self, nodes: &mut [Node]) -> Result<Option<(ArrayBinding, Vec<usize>)>, Error> {
        match nodes {
            [Node { kind: NodeKind::Name(name), span }] => Ok(Some((self.array_binding(name, span)?, vec![]))),
            [Node { kind: NodeKind::Group(inner) | NodeKind::Run(inner), .. }] => self.direct_item(inner),
            _ => Ok(self.direct_target(nodes)?.and_then(|(binding, selection)| match (selection.frame, selection.targets) {
                (ResultFrame::Direct, Targets::Offsets(offsets)) => Some((binding, offsets)),
                (ResultFrame::Direct, Targets::Paths(mut paths)) if paths.len() == 1 => Some((binding, paths.remove(0))), _ => None
            })),
        }
    }

    /// The value of `node`, which then becomes a literal, so that nothing evaluates it again.
    fn evaluate_once(&mut self, node: &mut Node) -> Result<Value, Error> {
        let value = self.array_result(std::slice::from_ref(node))?;
        node.kind = NodeKind::Literal(value.clone());
        Ok(value)
    }

    fn selection_expression(&mut self, nodes: &[Node]) -> Result<(ArrayBinding, crate::selection::Labels, Marked), Error> {
        let members = self.members(nodes);
        let nodes = &members[..];
        let root = nodes.len() - 1;
        let span = &nodes[root].span;
        let (binding, labels, selected) = match &nodes[root].kind {
            NodeKind::Name(name) => {
                let binding = self.array_binding(name, span)?;
                let (labels, marks) = crate::selection::Labels::new(&binding.value, span)?;
                let values = binding.value.clone();
                (binding, labels, Marked { labels: marks, values, kind: SelectionKind::Item })
            }
            NodeKind::Group(inner) | NodeKind::Run(inner) => self.selection_expression(inner)?,
            _ if root > 0 && self.holds_array(&nodes[root - 1]) && self.holds_array(&nodes[root]) => {
                let span = Span { source: span.source.clone(), range: nodes[root - 1].span.range.start..span.range.end };
                return Err(span.error(ErrorKind::Syntax, STRAND_TARGET));
            }
            _ => return Err(span.error(ErrorKind::Syntax, "selection must end in an array name")),
        };
        let (_, selected) = Binder::evaluate_marked(nodes, self, false, Some((root, selected)), None)?;
        Ok((binding, labels, selected.expect("a selection target's value stays marked")))
    }

    /// Writes the new value of each target of `selection` into `binding`. `f` takes the target's item on its left and its value from `right` on its right. A target that repeats a position reads the result of the earlier one, so repeats accumulate. A target at a fill reads the fill, and its write is dropped. An error leaves the array unchanged. When `f` is a pervasive primitive, `pervasive::scatter` combines the targets into the array in place. Where it can't, one call covers all the targets at distinct positions, and repeated positions take rounds.
    fn write_modified(&mut self, binding: ArrayBinding, selection: &Selection, f: &Function, right: &Value, span: &Span) -> Result<(), Error> {
        let right = selection.checked(right, &self.at(span))?;
        // The modifier never runs for a target at a fill.
        if let Some((kept, right)) = selection.without_fills(&right, &self.at(span))? { return self.write_modified(binding, &kept, f, &right, span); }
        if let (ResultFrame::Array(_), Targets::Offsets(offsets), Some(p)) = (&selection.frame, &selection.targets, f.as_primitive()) {
            if p.pervasive(true) {
                // A singleton goes to every target, as a unit.
                let unit = if right.is_singleton() {
                    let item = right.at(0);
                    Some(if item.is_atom() { item } else { item.enclose().error_at(span, "invalid modified selection")? })
                } else { None };
                return self.write_kept(binding, span, |z, cx| {
                    if crate::pervasive::scatter(p, z, offsets, unit.as_ref().unwrap_or(&right)) {
                        crate::explain::accumulated(offsets.len(), z.len());
                        return Ok(());
                    }
                    let Some((counts, rounds)) = occurrences(offsets, z.len()) else {
                        let items = selection.read(z, cx)?;
                        let right = match unit { Some(unit) if right.shape() != items.shape() => unit, _ => right };
                        let values = f.call_array(Some(&items), &right, cx)?;
                        return selection.write_into(z, &values, cx);
                    };
                    // Round r holds the r-th target at each position. Its positions are distinct. One call computes the round from the values that earlier rounds wrote into a copy of the array. The copy then replaces the array.
                    let (order, sizes) = rustymath::search::grouped(&counts.iter().map(|&c| c as usize).collect::<Vec<_>>(), rounds);
                    let mut current = z.clone();
                    let mut rest = &order[..];
                    for size in sizes {
                        let (targets, more) = rest.split_at(size);
                        rest = more;
                        let round = Selection::offsets(targets.iter().map(|&t| offsets[t as usize]).collect());
                        let items = round.read(&current, cx)?;
                        let values = match &unit {
                            Some(unit) => unit.clone(), None => Selection::offsets(targets.iter().map(|&t| t as usize).collect()).read(&right, cx)?
                        };
                        let results = f.call_array(Some(&items), &values, cx)?;
                        round.write_into(&mut current, &results, cx)?;
                    }
                    *z = current;
                    Ok(())
                });
            }
        }
        let original = &binding.value;
        let mut results: Vec<Value> = Vec::with_capacity(selection.targets.len());
        let mut latest: HashMap<&[usize], usize> = HashMap::new();
        for i in 0..selection.targets.len() {
            let path = selection.targets.path(i);
            let item = match latest.get(path) { Some(&j) => results[j].clone(), None => crate::primitive::item_at(original, path) };
            results.push(f.call_array(Some(&item), &selection.item(&right, i), &mut self.at(span))?);
            latest.insert(path, i);
        }
        let values = match &selection.frame {
            ResultFrame::Direct => results.pop().unwrap_or(right),
            ResultFrame::Array(layout) if !results.is_empty() => Value::new(layout.shape().to_vec(), results).error_at(span, "invalid modified selection")?,
            ResultFrame::Array(_) => right,
        };
        self.write_selection(binding, selection, &values, span)
    }
}
