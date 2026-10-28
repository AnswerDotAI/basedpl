//! The session's evaluation API, names, display, file loading and name lookup, and calls of dfns with their error guards.

use super::*;
use crate::syntax::Statement;
use rustymath::{buffer::FloatWidth, error::Kind};

impl Session {
    /// A turn at the process's one session, cleared to how a new session starts. It waits while another thread holds a turn.
    pub fn new() -> Turn {
        let turn = Turn::take();
        turn.clear();
        turn
    }
    /// A turn at the process's one session, as earlier turns left it. It waits while another thread holds a turn.
    pub fn current() -> Turn { Turn::take() }
    /// A session's fields as a new session has them, for the process's slot and for `clear`. `Session` has no public constructor, so no code builds a second session outside the slot.
    pub(super) fn blank() -> Self {
        Self { execution: Default::default(), prefacing: Default::default(), bindings: Default::default(), loading: Default::default(),
            stacks: Default::default(), depth: Default::default(), prototype: Default::default(), capture: Default::default(),
            fusion_reports: Default::default(), levels: Default::default(), transforming: Default::default(), generator: Default::default(),
            drawn: Default::default(), args: Default::default(), tacit: Default::default() }
    }
    /// Puts every field except the running evaluation's `execution` back as a new session has it, for `]clear` and a new session's turn. The destructuring names each field, so a field added later is a compile error here until it's reset too.
    pub(crate) fn clear(&self) {
        use std::sync::PoisonError;
        let Session {
            execution: _,
            prefacing,
            bindings,
            loading,
            stacks,
            depth,
            prototype,
            capture,
            fusion_reports,
            levels,
            transforming,
            generator,
            drawn,
            args,
            tacit: _,
        } = Session::blank();
        self.prefacing.set(prefacing.get());
        self.bindings.replace(bindings);
        self.loading.replace(loading.into_inner());
        self.stacks.replace(stacks.into_inner());
        self.depth.set(depth.get());
        self.prototype.set(prototype.get());
        self.capture.set(capture.get());
        self.fusion_reports.set(fusion_reports.get());
        self.levels.replace(levels.into_inner());
        self.transforming.set(transforming.get());
        std::mem::swap(&mut *self.generator.lock().unwrap_or_else(PoisonError::into_inner), &mut *generator.lock().unwrap_or_else(PoisonError::into_inner));
        *self.drawn.lock().unwrap_or_else(PoisonError::into_inner) = drawn.into_inner().unwrap_or_else(PoisonError::into_inner);
        self.args.replace(args.into_inner());
        self.tacit.clear();
    }
    /// A new session's turn for a person at a terminal, with boxed display elided to the terminal's width.
    pub fn interactive() -> Turn { Self::make_interactive(); Self::new() }
    /// Makes the interactive display settings the current ones and the ones that `]clear` restores. Settings are process-wide, so this needs no session. Python's BPL magics use it on the workspace they share with the Python API.
    pub fn make_interactive() { crate::settings::set_display_defaults(crate::display::Settings::interactive()); }
    /// `value` as this session's implicit display shows it, as text.
    pub fn show(&self, value: &Value) -> String { self.display_settings().array(value, false) }
    /// The implicit MIME display of `value`, without publishing it. Rendering is interruptible; a renderer's side output is discarded.
    pub fn render(&self, value: &Value, interrupt: crate::InterruptHandle) -> Result<crate::MimeBundle, Error> {
        let _running = self.execution.begin(crate::EvalOptions { interrupt, echo: false, output: Some(Arc::new(|_| {})), ..crate::EvalOptions::default() });
        self.render_value(value, &Span::whole(Source::new("<display>", "")))
    }
    /// The display settings, with floats of the default width shown without a marker.
    pub(crate) fn display_settings(&self) -> crate::display::Settings {
        let mut settings = crate::settings::display();
        settings.elide.plain = Some(crate::settings::float_width());
        settings
    }
    pub fn names(&self) -> Vec<String> { self.bindings.names() }
    pub fn name_class(&self, name: &str) -> i64 {
        match crate::inspection::item(name, crate::settings::hanzi_aliases()) {
            Some(NodeKind::Name(name, _)) => self.bindings.lookup(name).map_or(0, |(_, binding)| binding.class()),
            Some(NodeKind::Function(_)) => FUNCTION_CLASS, Some(NodeKind::System(_)) => 0, _ => -1
        }
    }
    pub fn name_list(&self, classes: &[i64], prefix: &str) -> Vec<String> {
        self.names().into_iter().filter(|name| name.starts_with(prefix) && classes.contains(&self.name_class(name))).collect()
    }
    pub fn inspect(&self, name: &str) -> Option<crate::Inspection> {
        let info = match crate::inspection::item(name, crate::settings::hanzi_aliases()) {
            Some(NodeKind::Name(name, _)) => self.bindings.lookup(name).and_then(|(_, binding)| binding.inspection(self)),
            Some(NodeKind::Function(builtin)) => Binding::Function(builtin.function()).inspection(self),
            Some(NodeKind::Operator(op)) => Some(crate::Inspection::new("operator", op.glyph().to_string())), _ => None
        };
        info.or_else(|| crate::inspection::documentation(name).map(|_| crate::Inspection::new("syntax", name.into())))
    }
    /// A bound name's native kind and short display, without evaluating it or fully formatting a large array.
    pub fn preview(&self, name: &str) -> Option<(&'static str, String)> {
        let NodeKind::Name(name, _) = crate::inspection::item(name, crate::settings::hanzi_aliases())? else { return None };
        let (_, binding) = self.bindings.lookup(name)?;
        let mut display = self.display_settings();
        display.boxed = false;
        display.elide = Elide { limit: 100, edges: 2, width: 80, ..display.elide };
        let (kind, text) = match binding {
            Binding::Value(a) => ("array", display.array(&a, false)), Binding::Function(f) => ("function", f.text(display.elide, &mut 100)),
            Binding::Operator(op) => ("operator", op.text(display.elide)), _ => return None
        };
        let mut chars = text.chars();
        let mut text: String = chars.by_ref().take(200).collect();
        if chars.next().is_some() { text.push('…'); }
        Some((kind, text))
    }
    pub fn name_source(&self, name: &str) -> Result<String, ErrorKind> {
        match self.name_class(name) {
            FUNCTION_CLASS | OPERATOR_CLASS => Ok(self.inspect(name).unwrap().source), SUBJECT_CLASS => Err(Kind::Domain.into()), _ => Err(ErrorKind::Value)
        }
    }
    /// Whether a level whose kind wraps values is running (`transform::Levels::transforming`).
    #[inline]
    pub(crate) fn transforming(&self) -> bool { self.transforming.get() }
    /// Starts a level of `kind` and returns its id, which is larger than every running level's. A `∂` call's id is its tag, and a record level's id is the level its records carry.
    pub(crate) fn begin(&self, kind: crate::transform::Kind) -> u64 {
        let frames = self.bindings.depth();
        let id = self.levels.borrow_mut().begin(kind, frames);
        self.transforming.set(true);
        id
    }
    /// Ends the innermost level, applying its kind's exit rule to each binding outside its frames that it wrote (`transform::Kind::exit`).
    pub(crate) fn end(&self) {
        let level = self.levels.borrow_mut().end();
        self.transforming.set(self.levels.borrow().transforming());
        let Some(exit) = level.kind.exit() else { return };
        for place in level.writes {
            let Some(Binding::Value(v)) = self.bindings.get(place) else { continue };
            self.bindings.insert(place, Binding::Value(exit(&v, level.id)));
        }
    }
    /// Records a write to the binding at `place`, for each running level whose exit acts on it, or stops a batched map's run before it (`transform::Levels::record_write`).
    pub(super) fn record_write(&self, place: Place) -> Result<(), crate::transform::Stop> { self.levels.borrow_mut().record_write(place) }
    /// Checks `effect` against the running levels (`transform::Levels::effect`).
    pub(crate) fn effect(&self, effect: crate::transform::Effect) -> Result<(), crate::transform::Stop> { self.levels.borrow().effect(effect) }
    /// The running trace's level and tracer, if a trace is running (`transform::Levels::tracing`).
    pub(crate) fn tracing(&self) -> Option<(u64, std::sync::Arc<crate::program::trace::Tracer>)> {
        self.levels.borrow().tracing().map(|(level, t, _)| (level, t.clone()))
    }
    /// Keeps a fresh name in the running trace or batched run (`transform::Levels::keep_fresh`).
    pub(crate) fn keep_fresh(&self, name: std::sync::Arc<str>) { self.levels.borrow_mut().keep_fresh(name) }
    /// Whether `value` holds a fresh name that the running batched run drew (`transform::Levels::holds_fresh`).
    pub(crate) fn holds_fresh(&self, value: &Value) -> bool { self.levels.borrow().holds_fresh(value) }
    /// The id and frame of the batched map that is running, if one is (`transform::Levels::batched`).
    pub(crate) fn batched(&self) -> Option<(u64, crate::array::Layout)> { self.levels.borrow().batched().map(|(id, frame)| (id, frame.clone())) }
    /// The id of the batched run, if one is running, without the frame that `batched` clones.
    pub(crate) fn batch_level(&self) -> Option<u64> { self.levels.borrow().batched().map(|(id, _)| id) }
    /// Whether the running levels let a map run batched (`transform::Levels::admits_batch`).
    pub(crate) fn admits_batch(&self) -> bool { self.levels.borrow().admits_batch() }
    /// Whether a map's batched try widens the running batched run (`transform::Levels::widens`).
    pub(crate) fn widens(&self) -> bool { self.levels.borrow().widens() }
    pub fn erase(&self, name: &str) -> bool {
        let Some(NodeKind::Name(name, _)) = crate::inspection::item(name, crate::settings::hanzi_aliases()) else { return false; };
        if name.implicit().is_some() { return false; }
        if let Some((place, _)) = self.bindings.lookup(name) { self.bindings.remove(place); }
        true
    }
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        let mut names = self.name_list(&[2, 3, 4], prefix);
        names.extend(crate::system::Entry::all().map(crate::system::Entry::name).filter(|name| name.starts_with(&prefix.to_lowercase())).map(str::to_owned));
        names.sort_unstable();
        names
    }
    /// `f` applied to the name that `right` holds. The system table maps `•nc`, `•ex` and `•src` over an array of names.
    fn named(&self, right: &Value, span: &Span, f: impl FnOnce(&Self, &str) -> Result<Value, ErrorKind>) -> Result<Value, Error> {
        let name = crate::keyed::name(right).ok_or_else(|| Error::domain("expected a name string").at(span))?;
        f(self, &name).map_err(|kind| Error::new(kind, format!("cannot inspect name: {name}")).at(span))
    }
    pub(crate) fn system_nc(&self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.named(right, span, |s, name| Ok(Value::integer(s.name_class(name))))
    }
    pub(crate) fn system_ex(&self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        crate::transform::acts().at(span)?;
        self.named(right, span, |s, name| Ok(Value::integer(s.erase(name) as i64)))
    }
    pub(crate) fn system_src(&self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.named(right, span, |s, name| s.name_source(name).map(|text| crate::keyed::text(&text)))
    }
    /// `•explain text` runs `text` in the caller's scope, as `⍎` does, computes its value, and gives a record of that value and of what happened while it ran and computed (`explain`). Computing a pending value records no `forced` row, as the speed timer's computing records none.
    pub(crate) fn system_explain(&self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        crate::transform::acts().at(span)?;
        let (result, events) = crate::explain::explained(|| {
            let result = self.execute(None, right, span).and_then(|r| r.array(span));
            if let Ok(value) = &result { value.force(); }
            result
        });
        events.report(result?, span)
    }
    /// `•literal text` reads a value from BPL source, as `•literal⁻¹` writes it, without running code. The text may hold literals and the functions that `•literal⁻¹` writes: `⍴`, `⊂`, `,`, `:` and `•ucs`.
    pub(crate) fn system_literal(&self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let parsed = self.parse(Source::new("<•literal>", Self::source_text(right, span)?)).complete()?;
        let [statement] = parsed.statements.as_slice() else { return Err(Error::domain("•literal reads one value").at(span)) };
        let written = |kind: &NodeKind| match kind {
            NodeKind::Function(Builtin::Primitive(p)) => matches!(p, Primitive::Shape | Primitive::Enclose | Primitive::Ravel),
            NodeKind::Function(Builtin::System(crate::system::Entry::UCS)) => true, _ => false
        };
        if let Some(node) = crate::syntax::non_literal(&statement.nodes, &written) {
            return Err(Error::domain(format!("•literal reads data, not code: {}", node.span.text())).at(span));
        }
        self.bind(&statement.nodes)?.array(span)
    }
    /// `•time t` gives the seconds since `t`, counting from the Unix epoch. `F •time x` gives each function's fastest time per call on `x`, in the layout of `F`.
    pub(super) fn system_time(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        crate::transform::acts().at(span)?;
        // Times stay 64-bit, as `•date` gives them, because a 32-bit float holds seconds since 1970 only to within about two minutes.
        let second = |s| Value::Number(crate::Number::float(s, FloatWidth::F64));
        let Some(functions) = left else {
            let now = second(crate::date::now());
            return Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(Some(&now), right, span);
        };
        let times = functions.elements().map(|f| match f {
            Value::Function(f) => self.fastest(&f, right, span), _ => Err(Error::domain("•time needs functions on its left").at(span))
        }).collect::<Result<Vec<_>, _>>()?;
        if functions.is_atom() { return Ok(second(times[0])); }
        Value::floats(functions.shape().to_vec(), FloatWidth::F64, times).and_then(|t| t.with_layout(functions.layout().clone()))
            .described("invalid •time result").at(span)
    }
    /// The fastest time per call of `f` on `x`, in seconds. Calls repeat for about 0.1 s, in batches that double until one takes 1 ms.
    fn fastest(&self, f: &Function, x: &Value, span: &Span) -> Result<f64, Error> {
        use {crate::host::Instant, std::time::Duration};
        let start = Instant::now();
        let (mut batch, mut best) = (1u32, f64::INFINITY);
        loop {
            let begun = Instant::now();
            for _ in 0..batch { f.call(None, x, span)?; }
            let taken = begun.elapsed();
            best = best.min(taken.as_secs_f64() / f64::from(batch));
            if start.elapsed() >= Duration::from_millis(100) { return Ok(best); }
            if taken < Duration::from_millis(1) { batch *= 2; }
        }
    }
    pub(crate) fn system_nl(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let prefix = match left {
            Some(value) => crate::keyed::name(value).ok_or_else(|| Error::domain("•nl prefix must be a string").at(span))?, None => "".into()
        };
        if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "•nl needs a class or class vector").at(span)); }
        let classes = right.as_items().integers().described("invalid name class").at(span)?;
        if classes.iter().any(|n| !(SUBJECT_CLASS..=OPERATOR_CLASS).contains(n)) { return Err(Error::domain("name classes are 2, 3 and 4").at(span)); }
        let values = self.name_list(&classes, &prefix).iter().map(|n| crate::keyed::text(n)).collect::<Vec<_>>();
        crate::keyed::texts(&[values.len()], values).described("invalid name list").at(span)
    }
    pub fn set(&self, name: &str, value: Value) -> Result<(), ErrorKind> { self.set_value(name, Binding::from_element(value)) }
    pub fn set_function(&self, name: &str, value: Function) -> Result<(), ErrorKind> { self.set_value(name, Binding::Function(value)) }
    fn set_value(&self, name: &str, value: Binding) -> Result<(), ErrorKind> {
        let Some(NodeKind::Name(name, _)) = crate::inspection::item(name, crate::settings::hanzi_aliases()) else { return Err(ErrorKind::Syntax); };
        if name.implicit().is_some() { return Err(ErrorKind::Syntax); }
        let place = self.bindings.at_top_level(Some(0), || self.bindings.here(name));
        self.bindings.insert(place, value);
        Ok(())
    }
    pub fn eval(&self, code: &str) -> Evaluation { self.eval_with(code, crate::EvalOptions::default()) }
    pub fn eval_with(&self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.evaluate_source(Source::new("<input>", code)))
    }
    /// `code` evaluated in a module of its own, as `•load` runs a file: it sees none of the session's names and adds none. An expectation in the reference harness or a test evaluates this way, beside the code it checks.
    pub fn eval_apart(&self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.bindings.at_top_level(Some(s.bindings.new_module()), || s.evaluate_source(Source::new("<input>", code))))
    }
    /// One evaluation under `options`: `evaluate`'s result, with the output it produced.
    fn evaluation(&self, options: crate::EvalOptions, evaluate: impl FnOnce(&Self) -> Evaluation) -> Evaluation {
        let _running = self.execution.begin(options);
        let result = evaluate(self);
        self.execution.flush();
        self.stacks.take();
        Evaluation { output: self.execution.take_output(), ..result }
    }
    pub fn eval_timeout(&self, code: &str, timeout: std::time::Duration) -> Evaluation {
        self.eval_with(code, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() })
    }
    /// Resolve a function expression in this session and call it with one (right) or two (left, right) arrays. No function or lexical-frame handle escapes the call, and no temporary names are bound.
    pub fn call(&self, function: &str, args: &[Value]) -> Evaluation { self.call_with(function, args, crate::EvalOptions::default()) }
    pub fn call_with(&self, function: &str, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        match self.late_bound(function) { Ok(f) => self.call_function_with(&f, args, options), Err(error) => Evaluation::failed(error) }
    }
    /// The function that `expression` writes, late-bound, with its numbers read at the session's default width.
    pub fn late_bound(&self, expression: &str) -> Result<Function, Error> {
        let span = Span::whole(Source::new("<call>", expression));
        let parsed = self.parse(span.source.clone()).complete()?;
        if parsed.statements.len() != 1 { return Err(Error::new(ErrorKind::Syntax, "call requires one function expression").at(&span)); }
        Function::new(FunctionNode::LateBound(Arc::new(parsed), span.clone()), &span)
    }
    pub fn call_function_with(&self, function: &Function, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.call_function(function, args))
    }
    fn call_function(&self, function: &Function, args: &[Value]) -> Evaluation {
        let span = Span::whole(Source::new("<call>", function.bpl()));
        let mut result = Evaluation::default();
        let called = (|| {
            let (left, right) = match args {
                [right] => (None, right), [left, right] => (Some(left), right),
                _ => return Err(Error::new(Kind::Length, "call requires one or two arguments").at(&span))
            };
            function.call(left, right, &span)?.result(&span)
        })();
        match called {
            Ok(Bound { value: Binding::Value(a), shy, .. }) => {
                if self.execution.echo.get() && !shy { if let Err(e) = self.display_value(&a, &span) { result.error = Some(e); } }
                result.value = Some(a);
            }
            Ok(Bound { value: Binding::Function(f), shy, .. }) => {
                if self.execution.echo.get() && !shy { self.display_function(&f, None); }
                result.function = Some(f);
            }
            Ok(_) => (),
            Err(e) => result.error = Some(e),
        }
        result
    }
    pub fn eval_source(&self, source: Arc<Source>, options: crate::EvalOptions) -> Evaluation { self.evaluation(options, |s| s.evaluate_source(source)) }
    /// A line that starts with `]` is a command, unless it closes brackets that the code before it left open. Each part keeps its line numbers, so errors point at the right line.
    fn evaluate_source(&self, source: Arc<Source>) -> Evaluation {
        let (mut parts, mut start, mut offset) = (Vec::new(), 0, 0);
        for line in source.text.split_inclusive('\n') {
            let pending = &source.text[start..offset];
            if line.trim_start().starts_with(']') && (pending.trim().is_empty() || !matches!(self.parse(Source::new("", pending)), ParseStatus::Incomplete(_)))
            {
                if !pending.trim().is_empty() { parts.push(start..offset); }
                parts.push(offset..offset + line.len());
                start = offset + line.len();
            }
            offset += line.len();
        }
        if parts.is_empty() { return self.evaluate_code(source); }
        if !source.text[start..].trim().is_empty() { parts.push(start..source.text.len()); }
        let mut result = Evaluation::default();
        for range in parts {
            let text = &source.text[range.clone()];
            let code = format!("{}{text}", "\n".repeat(source.text[..range.start].matches('\n').count()));
            let part = Arc::new(Source { name: source.name.clone(), text: code, file: source.file });
            let next = if text.trim_start().starts_with(']') { self.command(part) } else { self.evaluate_code(part) };
            (result.value, result.function, result.operator) = (next.value, next.function, next.operator);
            if next.error.is_some() { result.error = next.error; break; }
        }
        result
    }
    /// Code at the top level of an evaluation, such as a script, a notebook cell or a REPL turn. A syntax error anywhere in the code stops it before anything runs.
    fn evaluate_code(&self, source: Arc<Source>) -> Evaluation {
        match self.parse(source).complete() { Ok(parsed) => self.eval_display(&parsed, false, false), Err(e) => Evaluation::failed(e) }
    }
    /// The program in `source`, with numbers that have no marker read as floats of the default width, or of the width that a leading `•prefs` statement on a literal record sets.
    pub fn parse(&self, source: Arc<Source>) -> crate::ParseStatus { crate::syntax::parse_with_aliases(source, crate::settings::hanzi_aliases()) }
    pub fn eval_parsed(&self, parsed: &Parsed, options: crate::EvalOptions) -> Evaluation { self.evaluation(options, |s| s.eval_display(parsed, false, false)) }
    fn eval_display(&self, parsed: &Parsed, diagram: bool, dissect: bool) -> Evaluation {
        let mut result = Evaluation::default();
        let mut leading = true;
        for (i, statement) in parsed.statements.iter().enumerate() {
            (result.function, result.operator) = (None, None);
            let nodes = &statement.nodes;
            let previous = self.capture.get();
            let dissecting = self.depth.get() == 0 && (dissect || crate::settings::display().dissect && self.execution.echo.get());
            self.capture.set(dissecting);
            leading &= crate::syntax::prefs_record(nodes).is_some();
            let bound = self.bind_statement(statement, leading);
            self.capture.set(previous);
            match bound {
                Ok(bound) => {
                    let dissected = if dissecting && (dissect || !bound.shy && !matches!(bound.value, Binding::NoResult | Binding::Absent)) {
                        bound.expression.map(|e| *e).or_else(|| Expression::binding(&bound.value)).map(|e| {
                            let display = crate::settings::display();
                            e.display(display.trees, display.elide)
                        })
                    } else { None };
                    if let Some(text) = &dissected { self.execution.output(crate::OutputKind::Display, text.clone()); }
                    result.value = match bound.value {
                        Binding::NoResult | Binding::Absent | Binding::Released => None,
                        Binding::Value(a) => {
                            if diagram && i + 1 == parsed.statements.len() {
                                self.execution.output(crate::OutputKind::Display, self.display_settings().diagram(&a));
                            } else if self.execution.echo.get() && !bound.shy {
                                if let Err(e) = self.display_value(&a, &nodes[0].span) { result.error = Some(e); return result; }
                            }
                            Some(a)
                        }
                        _ if bound.shy => None,
                        Binding::Function(f) => {
                            if self.execution.echo.get() { self.display_function(&f, dissected.as_deref()); }
                            if i + 1 == parsed.statements.len() { result.function = Some(f); }
                            None
                        }
                        Binding::Operator(op) => {
                            let op = Operator(Arc::new(op));
                            if self.execution.echo.get() { self.execution.output(crate::OutputKind::Display, op.text(crate::settings::display().elide)); }
                            if i + 1 == parsed.statements.len() { result.operator = Some(op); }
                            None
                        }
                    };
                }
                Err(e) => {
                    result.value = None;
                    result.error = Some(e.at(&crate::syntax::cover(nodes)));
                    break;
                }
            }
        }
        result
    }
    fn command(&self, source: Arc<Source>) -> Evaluation {
        let span = Span::whole(source);
        let failed = |kind, message: String| Evaluation::failed(Error::new(kind, message).at(&span));
        let code = span.source.text.trim();
        let (command, args) = code.split_once(char::is_whitespace).unwrap_or((code, ""));
        let searching = command.eq_ignore_ascii_case("]help?");
        if searching || command.eq_ignore_ascii_case("]help") {
            use crate::inspection::{self, Help};
            let usage = "usage: ]help, ]help topic [-source], or ]help? regex";
            let Some(request) = inspection::help_command(args, searching) else { return failed(ErrorKind::Syntax, usage.into()) };
            let result = (|| -> Result<_, Error> {
                let markdown = match request {
                    Help::Topics => inspection::topics(),
                    Help::Search(pattern) => inspection::search(pattern, true)?,
                    Help::Topic("llms", _) => inspection::LLMS.to_owned(),
                    Help::Topic(name, detail) => {
                        if !detail && let Some(page) = inspection::page(name) { page.to_owned() }
                        else if let Some(info) = self.inspect(name) { return Ok((info.text(detail), info.markdown(detail))); }
                        else if !detail { inspection::search(name, false)? }
                        else { return Err(Error::new(ErrorKind::Value, format!("name not found: {name}"))); }
                    }
                };
                Ok((markdown.clone(), markdown))
            })();
            let (text, markdown) = match result { Ok(output) => output, Err(error) => return Evaluation::failed(error.at(&span)) };
            let data = [("text/plain", text), ("text/markdown", markdown)].map(|(kind, text)| (kind.to_string(), crate::MimeData::Text(text))).into();
            self.execution.emit(crate::Output { kind: crate::OutputKind::Display, data });
            return Evaluation::default();
        }
        if command.eq_ignore_ascii_case("]clear") {
            if !args.is_empty() { return failed(ErrorKind::Syntax, "usage: ]clear".into()); }
            crate::settings::reset();
            self.clear();
            return Evaluation::default();
        }
        if command.eq_ignore_ascii_case("]display") {
            return match self.parse(Source::new("<display>", args)).complete() {
                Ok(parsed) => self.eval_display(&parsed, true, false), Err(e) => Evaluation::failed(e)
            };
        }
        if command.eq_ignore_ascii_case("]dissect") {
            if args.trim().is_empty() { return failed(ErrorKind::Syntax, "usage: ]dissect expression".into()); }
            return match self.parse(Source::new("<dissect>", args)).complete() {
                Ok(parsed) => self.eval_display(&parsed, false, true), Err(e) => Evaluation::failed(e)
            };
        }
        failed(ErrorKind::Syntax, "unknown user command".into())
    }
    pub(super) fn bind(&self, nodes: &[Node]) -> Result<Bound, Error> { self.bound(nodes, None) }
    /// `nodes` evaluated as one statement through `slot`, as `statement_step` uses it, one level deeper. A compiled statement runs at once: one loop runs its steps, so it needs only the depth count. The binder evaluates any other statement with an interrupt check and a stack that grows as needed (`nested`).
    pub(super) fn bound(&self, nodes: &[Node], slot: Option<&std::sync::OnceLock<Option<super::Recording>>>) -> Result<Bound, Error> {
        let done = |step: Step| {
            let Step::Done(result) = step else { unreachable!("a statement that isn't a dfn's last gives its value") };
            result
        };
        match super::tree::compiled(slot) {
            Some(recording) => {
                self.descend(&nodes[0].span)?;
                let result = recording.evaluate(nodes, self, false).map(done);
                self.ascend();
                result
            }
            None => self.nested(nodes, |session| session.statement_step(slot, nodes, false).map(done)),
        }
    }
    /// `f` evaluating `nodes` one level deeper, after an interrupt check, within the depth limit and on a stack that grows as needed.
    pub(super) fn nested<T>(&self, nodes: &[Node], f: impl FnOnce(&Self) -> Result<T, Error>) -> Result<T, Error> {
        crate::execution::check().at(&nodes[0].span)?;
        self.descend(&nodes[0].span)?;
        let result = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || f(self));
        self.ascend();
        result
    }
    /// Counts one more level of evaluation, or fails at the depth limit (`MAX_CALL_DEPTH`): a call, or a statement or group that a call evaluates.
    #[inline(always)]
    pub(super) fn descend(&self, span: &Span) -> Result<(), Error> { if self.deeper() { Ok(()) } else { Err(depth_error(span)) } }
    /// Counts one more level of evaluation, as `descend` does, and gives false without counting at the depth limit, for a caller that finds the error's text only then.
    #[inline(always)]
    pub(super) fn deeper(&self) -> bool {
        let depth = self.depth.get();
        if depth == MAX_CALL_DEPTH { return false; }
        self.depth.set(depth + 1);
        true
    }
    /// Counts one level of evaluation less, after `descend`.
    #[inline(always)]
    pub(super) fn ascend(&self) { self.depth.set(self.depth.get() - 1); }

    /// `•mime Y` is the MIME bundle that displays `Y`. `X •mime Y` is `Y` displayed through `X`, which holds a function or names a MIME type. A MIME type shows `Y` itself as that type.
    pub(super) fn mime(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let Some(left) = left else { return self.mime_value(right, span) };
        let renderer = match left.is_singleton().then(|| left.at(0)) {
            Some(Value::Function(f)) => f,
            _ => {
                let Some(kind) = crate::keyed::name(left) else { return Err(Error::domain("•mime needs a function or a MIME type on its left").at(span)) };
                before(crate::keyed::text(&kind), Function::primitive(Primitive::Keys), span)?
            }
        };
        right.clone().with_renderer(renderer).described("invalid •mime result").at(span)
    }
    fn mime_value(&self, right: &Value, span: &Span) -> Result<Value, Error> {
        let fallback = crate::keyed::vector(vec!["text/plain".into()], vec![crate::keyed::text(&self.show(right))]).unwrap();
        let Some(renderer) = right.renderer().cloned() else { return Ok(fallback); };
        let echo = self.execution.echo.replace(false);
        let rendered = renderer.call_array(None, right, span);
        self.execution.echo.set(echo);
        let bundle = crate::keyed::merge(&fallback, &rendered?).described("MIME renderer must return a keyed vector").at(span)?;
        crate::display::bundle(&bundle).described("MIME bundle must map MIME types to text or bytes").at(span)?;
        Ok(bundle)
    }

    fn display_value(&self, value: &Value, span: &Span) -> Result<(), Error> {
        self.execution.emit(Output { kind: crate::OutputKind::Display, data: self.render_value(value, span)? });
        Ok(())
    }

    /// A value with a renderer formats through it. Other values, and a renderer that fails, format as text.
    fn render_value(&self, value: &Value, span: &Span) -> Result<crate::MimeBundle, Error> {
        if value.renderer().is_some() {
            match self.mime_value(value, span) {
                Ok(bundle) => { return Ok(crate::display::bundle(&bundle).expect("•mime checks its bundle")); }
                Err(e) if e.kind.is_control() => return Err(e), Err(_) => ()
            }
        }
        Ok([("text/plain".into(), crate::MimeData::Text(self.show(value)))].into_iter().collect())
    }

    /// Displays a function using the trees setting, unless that view has already been shown.
    fn display_function(&self, f: &Function, shown: Option<&str>) {
        let display = crate::settings::display();
        let text = if display.trees { f.tree(display.elide, &mut 1000).render() } else { f.text(display.elide, &mut 1000) };
        if shown != Some(&text) { self.execution.output(crate::OutputKind::Display, text); }
    }

    pub(super) fn execute(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        let source = Source::new("<execute>", Self::source_text(right, span)?);
        match left { Some(record) => self.scoped(record, span, |session| session.execute_source(source, span)), None => self.execute_source(source, span) }
    }

    /// The text that `right` holds, as `⍎` and `•load` read it: an observing read, which stops a batched run where `right` is a batch of each cell's text (`Value::observed`).
    fn source_text(right: &Value, span: &Span) -> Result<String, Error> {
        right.observed().at(span)?;
        if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "expected a unit or vector of characters").at(span)); }
        if !matches!(right.prototype(), Value::Character(_)) { return Err(Error::domain("expected characters").at(span)); }
        right.elements().map(|e| match e { Value::Character(c) => Ok(c), _ => Err(Error::domain("expected characters").at(span)) }).collect()
    }

    /// `•load path` runs a file in a module of its own and returns a record of the module's public names: the names its top level assigns that don't start with `_`, in alphabetical order. The file adds no names to the caller, and shows nothing except explicit output.
    pub(super) fn load(&self, right: &Value, span: &Span) -> Result<Bound, Error> {
        crate::transform::acts().at(span)?;
        let text = Self::source_text(right, span)?;
        let path = span.path(&text);
        let code =
            crate::host::read(&path).and_then(|bytes| String::from_utf8(bytes).map_err(std::io::Error::other)).map_err(|e| Error::io(&text, e).at(span))?;
        let file = crate::host::canonical(&path);
        if self.loading.borrow().contains(&file) { return Err(Error::domain(format!("load cycle: {text} is already loading")).at(span)); }
        let module = self.bindings.new_module();
        let echo = self.execution.echo.replace(false);
        self.loading.borrow_mut().push(file);
        let result = self.bindings.at_top_level(Some(module), || self.execute_source(Source::file(path.to_string_lossy(), code), span));
        self.loading.borrow_mut().pop();
        self.execution.echo.set(echo);
        result?;
        let exported = self.bindings.module_names(module).into_iter().filter(|(name, _)| !name.text().starts_with('_'));
        let (keys, values): (Vec<_>, Vec<_>) = exported.map(|(name, value)| (Arc::<str>::from(name.text()), value)).unzip();
        let values = values.into_iter().map(|v| v.into_value(span)).collect::<Result<Vec<_>, _>>()?;
        Ok(Bound::new(Binding::Value(crate::keyed::vector(keys, values).described("invalid module record").at(span)?)))
    }

    fn execute_source(&self, source: Arc<Source>, span: &Span) -> Result<Bound, Error> {
        let parsed = self.parse(source).complete()?;
        let mut result = Bound::new(Binding::NoResult);
        let mut leading = true;
        for statement in &parsed.statements {
            if let Binding::Value(a) = &result.value { if self.execution.echo.get() && !result.shy { self.display_value(a, span)?; } }
            leading &= crate::syntax::prefs_record(&statement.nodes).is_some();
            result = self.bind_statement(statement, leading).map_err(|mut e| { e.calls.push(span.clone()); e })?;
        }
        Ok(result)
    }

    fn bind_statement(&self, statement: &Statement, leading: bool) -> Result<Bound, Error> {
        let prefacing = self.prefacing.replace(leading);
        let result = self.bind(&statement.nodes);
        self.prefacing.set(prefacing);
        result
    }

    // Resolving one structural item may execute a group, but never derives an operator or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    pub(super) fn resolve(&self, node: &Node) -> Result<Bound, Error> {
        Ok(Bound::new(match &node.kind {
            NodeKind::Literal(a) => Binding::Value(a.clone()),
            NodeKind::Function(builtin) => Binding::Function(builtin.function()),
            NodeKind::Operator(op) => Binding::Operator(OperatorNode::Primitive(*op)),
            NodeKind::Name(name, last) => match last.then(|| self.bindings.take(*name)).flatten().or_else(|| self.bindings.lookup(*name)) {
                Some((_, Binding::Released)) => return Err(released(name, &node.span)),
                Some((place, value)) => self.read_binding(value, || Some(place))?,
                None if *name == Name::ERROR => {
                    return Err(Error::new(ErrorKind::Value, "$e is the error a handler caught, and exists only in a handler").at(&node.span));
                }
                None => return Err(undefined(name, &node.span)),
            },
            NodeKind::System(name) => return Err(undefined(name, &node.span)),
            NodeKind::Group(_) | NodeKind::Run(_) => return Binder::nested(node, self),
            NodeKind::Scope(root, body) => {
                let record = self.resolve(root)?.array(&root.span)?;
                return self.scoped(&record, &node.span, |session| session.resolve(body));
            }
            NodeKind::ArrayLiteral { .. } => self.bracket(node, |nodes, _, _, within| match within {
                Within::Key => self.array_result(nodes).map(Some), _ => self.item_result(nodes)
            })?,
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.bindings.current(), module: self.bindings.module() };
                match definition.kind {
                    DefinitionKind::Function => Binding::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Binding::Operator(OperatorNode::Defined(closure)),
                }
            }
            NodeKind::Output => {
                self.effect(crate::transform::Effect::Acts).at(&node.span)?;
                match self.execution.line(&node.span)? {
                    Some(line) => Binding::Value(crate::keyed::text(&line)),
                    None => Binding::Value(crate::syntax::zilde(crate::Number::float(0.0, crate::settings::float_width()))),
                }
            }
            NodeKind::Joiner(_) => return Err(Error::new(ErrorKind::Syntax, "ⱼ, ₑ and ᵣ join the two arrays they touch, as in xⱼy or 2ₑn").at(&node.span)),
            _ => return Err(Error::new(ErrorKind::Syntax, "unexpected assignment symbol").at(&node.span)),
        }))
    }

    /// The bracket literal `node`, built from what `part` gives for each of its items and keys. `part` gets them in the order that they evaluate: left to right, with an item's value before its key. It gets a part's nodes, the cell that holds them, their range in that cell and what they are (`Within`), and gives `None` for an absent `⍺`, which drops out. A keyed item's key applies as soon as the key is ready (`Session::keyed`), before the next item evaluates. The key of an item whose value is absent isn't evaluated. A key with no value is a name, which gives both: `[a:]` is `["a":a]`.
    pub(super) fn bracket(
        &self, node: &Node, mut part: impl FnMut(&[Node], usize, std::ops::Range<usize>, Within) -> Result<Option<Value>, Error>) -> Result<Binding, Error> {
        let NodeKind::ArrayLiteral { cells, record, .. } = &node.kind else { unreachable!("a bracket literal") };
        if !*record {
            let mut arrays = Vec::with_capacity(cells.len());
            for (k, nodes) in cells.iter().enumerate() { arrays.extend(part(nodes, k, 0..nodes.len(), Within::Item)?); }
            return Self::list_literal(node, arrays);
        }
        let mut entries = Vec::new();
        for (k, nodes) in cells.iter().enumerate() {
            let Some(colon) = crate::syntax::key_colon(nodes) else {
                entries.extend(part(nodes, k, 0..nodes.len(), Within::Item)?.map(|item| (None, item)));
                continue;
            };
            // The value and the key evaluate separately, value first. As one expression, `key:f` would be a train.
            let named = match &nodes[..colon] {
                _ if colon + 1 < nodes.len() => None, [Node { kind: NodeKind::Name(name, _), .. }] => Some(crate::keyed::text(name.text())),
                _ => return Err(Error::new(ErrorKind::Syntax, "a key with no value is a name, as in [a:]").at(&nodes[colon].span))
            };
            let at = if named.is_some() { 0..colon } else { colon + 1..nodes.len() };
            let Some(value) = part(&nodes[at.clone()], k, at, Within::Value { colon, named: named.clone() })? else { continue };
            let key = match named { Some(key) => key, None => part(&nodes[..colon], k, 0..colon, Within::Key)?.expect("a key gives an array") };
            entries.extend(self.keyed(node, k, colon, &key, &value)?);
        }
        Self::record_literal(node, entries)
    }

    /// The entries of the keyed item in cell `cell` of the bracket literal `node`, whose `:` is at `colon`: `Keys` applies `key` to `value`, which must give a keyed vector. A name with a batch of atoms gives one entry that holds the batch, as `Keys` gives one entry in each cell, so a record built in a batched run holds batches as a list literal does. `bracket` and a compiled item's end both call it as soon as the item's value is ready, so an item that fails stops the items after it.
    pub(super) fn keyed(&self, node: &Node, cell: usize, colon: usize, key: &Value, value: &Value) -> Result<Vec<crate::keyed::Entry>, Error> {
        let NodeKind::ArrayLiteral { cells, .. } = &node.kind else { unreachable!("a bracket literal") };
        if let Some(name) = crate::keyed::name(key) && value.carrier().and_then(crate::transform::Carrier::batch).is_some_and(|b| b.rank() == 0) {
            return Ok(vec![(Some(name), value.clone())]);
        }
        let item = Function::primitive(Primitive::Keys).call(Some(key), value, &cells[cell][colon].span)?.array(&node.span)?;
        crate::keyed::entries(&item).described("key:value items must give keyed vectors").at(&node.span)
    }

    /// The record literal `node`, from its items' entries in order. Its keys must be unique.
    pub(super) fn record_literal(node: &Node, entries: Vec<crate::keyed::Entry>) -> Result<Binding, Error> {
        let (names, values) = entries.into_iter().unzip();
        Ok(Binding::Value(crate::keyed::partial_vector(names, values).described("keys must be unique").at(&node.span)?))
    }

    /// The list literal `node`, from its items' arrays in order, which assemble by its form.
    pub(super) fn list_literal(node: &Node, arrays: Vec<Value>) -> Result<Binding, Error> {
        let NodeKind::ArrayLiteral { form, .. } = &node.kind else { unreachable!("a bracket literal") };
        if arrays.is_empty() { return Ok(Binding::from_element(crate::syntax::zilde(crate::Number::float(0.0, crate::settings::float_width())))); }
        let result = if *form == ListForm::Cells {
            // Each row is a major cell, so unit rows give a vector: `[1⋄2]` is `1 2`.
            Value::assemble(&[arrays.len()], &arrays, &arrays[0])
        } else { Value::new(vec![arrays.len()], arrays) };
        Ok(Binding::Value(result.described("invalid array literal").at(&node.span)?))
    }

    // Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
    #[inline(never)]
    pub(super) fn call_defined<L: Argument, R: Argument>(&self, first: &Function, left: Option<L>, right: R) -> Result<Bound, Error> {
        let caller = self.bindings.current();
        let base = self.bindings.depth();
        let (mut function, mut left, mut right) = (first.clone(), left.map(L::owned), right.owned());
        let mut tail_span = None;
        let mut unshy = false;
        let mut result = loop {
            let (closure, operand, right_operand) = match function.node() {
                FunctionNode::Defined(c) => (c, None, None), FunctionNode::Derived(c, operand, right) => (c, Some(operand), right.as_ref()), _ => unreachable!()
            };
            let frame = Frame::new(closure.environment, closure.module, closure.definition.locals());
            // The six names that a call binds itself, in slot order (`Name::implicit`): ⍺, absent without a left argument, then ⍵ ⍶ ⍹ ∇ ⍢.
            let implicit = [
                Some(left.map_or(Binding::Absent, Binding::Value)), Some(Binding::Value(right)), operand.map(|f| f.value()), right_operand.map(|f| f.value()),
                Some(Binding::Function(function.clone())), operand.map(|_| Binding::Operator(OperatorNode::Defined(closure.clone()))),
            ];
            if let Err(error) = self.bindings.enter(frame, implicit, &closure.definition.span) { break Err(error); }
            match self.run_definition(&closure.definition, (0, 0), true) {
                Ok(Step::Done(bound)) => break Ok(bound),
                Err(error) => break Err(error),
                Ok(Step::Tail(call)) => {
                    // Keep lexical dependencies, not the tail caller's execution frame.
                    let environment = call.function.environment().max(call.right.environment()).max(call.left.as_ref().and_then(Value::environment));
                    let keep = base.max(environment.map_or(0, |i| i + 1));
                    self.bindings.truncate(keep);
                    function = call.function;
                    left = call.left;
                    right = call.right;
                    tail_span = Some(call.span);
                    unshy |= call.unshy;
                }
            }
        };
        if let (Err(error), Some(span)) = (&mut result, tail_span) { error.calls.push(span); }
        if let Ok(bound) = &mut result { if unshy { bound.shy = false; } }
        self.leave(base, caller, result, &first.definition().expect("call_defined calls a dfn").span)
    }

    /// Drops the frames from `base` on, makes `caller` current and gives back `result`. A result that refers to a dropped frame is an error.
    fn leave(&self, base: usize, caller: Option<usize>, result: Result<Bound, Error>, span: &Span) -> Result<Bound, Error> {
        let result = match result {
            Ok(bound) if bound.value.environment().is_some_and(|i| i >= base) => Err(Error::domain("result would return a local closure").at(span)),
            result => result,
        };
        self.bindings.leave(base, caller);
        result
    }

    /// Runs `f` in a new frame in which each key on the first axis of `record` names what `T.key` selects: an item of a vector, or a major cell of a higher-rank array. Other names resolve in the caller's scope, and assignments stay in the new frame. `T.(expr)` and `T⍎text` evaluate this way.
    fn scoped(&self, record: &Value, span: &Span, f: impl FnOnce(&Self) -> Result<Bound, Error>) -> Result<Bound, Error> {
        record.keys(0).ok_or_else(|| ErrorKind::from(Kind::Domain)).described("a scope needs keys on its first axis").at(span)?;
        let (caller, base) = (self.bindings.current(), self.bindings.depth());
        self.bindings.enter(Frame::scope(caller, self.bindings.module(), record.clone()), Default::default(), span)?;
        let result = f(self);
        self.leave(base, caller, result, span)
    }

    pub(super) fn array_result(&self, nodes: &[Node]) -> Result<Value, Error> { self.bind(nodes)?.array(&nodes[0].span) }
    /// An item of a bracket list, or `None` for an absent `⍺`, which drops out of the list.
    fn item_result(&self, nodes: &[Node]) -> Result<Option<Value>, Error> { listed(self.bind(nodes)?.value, &nodes[0].span) }

    /// A body's last statement, or an error handler's expression, as the dfn's result. `slot` holds a statement's recording of its binder steps (`tree.rs`).
    fn return_expression(&self, mut nodes: &[Node], tail: bool, slot: Option<&std::sync::OnceLock<Option<super::Recording>>>) -> Result<Step, Error> {
        if nodes.is_empty() { return Ok(Step::Done(Bound::new(Binding::NoResult))); }
        let mut unshy = false;
        while let [Node { kind: NodeKind::Group(inner), .. }] = nodes {
            nodes = inner;
            unshy = true;
        }
        let step = self.statement_step(slot, nodes, tail)?;
        match step {
            Step::Done(mut bound) => { if unshy { bound.shy = false; } bound.result(&nodes[0].span).map(Step::Done) }
            Step::Tail(mut call) => { call.unshy = unshy; Ok(Step::Tail(call)) }
        }
    }

    /// Runs `definition`'s bodies in the current frame, from statement `start.1` of body `start.0`. With `tail`, a last statement that calls a function gives the call to the caller's loop (`Step::Tail`), unless an error guard is active.
    fn run_definition(&self, definition: &Definition, start: (usize, usize), tail: bool) -> Result<Step, Error> {
        let frame = self.bindings.current().unwrap();
        let mut handlers = Vec::new();
        let result = (|| {
            'bodies: for (b, body) in definition.bodies.iter().enumerate().skip(start.0) {
                for (position, statement) in body.iter().enumerate().skip(if b == start.0 { start.1 } else { 0 }) {
                    let nodes = &statement.nodes;
                    match statement.kind {
                        StatementKind::DefaultArgument => {
                            if nodes.len() == 2 { return Err(Error::new(ErrorKind::Syntax, "default argument needs a value").at(&nodes[1].span)); }
                            if self.bindings.category(Name::ALPHA) == Some(Category::Absent) {
                                let value = self.bound(&nodes[2..], Some(&statement.recording))?.value;
                                if matches!(value, Binding::NoResult | Binding::Absent) {
                                    return Err(Error::new(ErrorKind::Value, "default argument requires a value").at(&nodes[0].span));
                                }
                                self.bindings.insert(self.bindings.here(Name::ALPHA), value);
                            }
                        }
                        StatementKind::ErrorGuard { index: i } => {
                            let catch = Catch::new(&self.bound(&nodes[..i], Some(&statement.recording))?.array(&nodes[0].span)?, &nodes[i].span)?;
                            handlers.push((&nodes[i + 1..], catch));
                        }
                        StatementKind::Predicate => {
                            let span = crate::syntax::cover(nodes);
                            let value = self.bound(nodes, Some(&statement.recording))?.array(&nodes[0].span)?;
                            let level = if self.transforming() { self.batch_level() } else { None };
                            let holds = match level.and_then(|level| crate::batch::truths(&value, level).map(|truths| (level, truths))) {
                                None => value.boolean().described(PREDICATE).at(&span)?,
                                Some((level, truths)) => {
                                    match self.guard(definition, frame, (b, position), &truths.described(PREDICATE).at(&span)?, level, &span)? {
                                        Guarded::Unanimous(holds) => holds, Guarded::Joined(value) => return Ok(Step::Done(Bound::from(value)))
                                    }
                                }
                            };
                            if !holds { continue 'bodies; }
                        }
                        StatementKind::Expression => {
                            if position + 1 == body.len() { return self.return_expression(nodes, tail && handlers.is_empty(), Some(&statement.recording)); }
                            self.bound(nodes, Some(&statement.recording))?;
                        }
                    }
                }
                return Ok(Step::Done(Bound::new(Binding::NoResult)));
            }
            Err(Error::domain("every predicate failed, so no body applies").at(&definition.span))
        })();
        let mut result = result;
        while let Err(error) = &result {
            let Some((handler, catch)) = handlers.pop() else { break };
            if !catch.catches(&error.kind) { continue; }
            if let Err(stop) = self.effect(crate::transform::Effect::Catches) { result = Err(stop.into()); break; }
            if matches!(error.kind, ErrorKind::Interrupt) { crate::execution::clear_interrupt(); }
            self.bindings.insert(self.bindings.here(Name::ERROR), Binding::Value(caught(error)));
            result = self.return_expression(handler, false, None);
        }
        result
    }

    /// A guard on predicate `truths`, which gives each cell of batched run `level` a Boolean of its own, at statement `at.1` of body `at.0` of `definition`, running in frame `frame` (`operators::guard`). Each side runs as a guard case on its own cells (`transform::Levels::case`): the rest of the body where the predicate holds, and the bodies after it elsewhere. A case reads each batched value at its cells (`restricted`), and runs its last call before it returns, to give a value.
    #[cold]
    #[inline(never)]
    fn guard(&self, definition: &Definition, frame: usize, at: (usize, usize), truths: &Value, level: u64, span: &Span) -> Result<Guarded, Error> {
        let key = definition as *const Definition as usize;
        super::operators::guard(truths, level, |holds, rows, cells| {
            let (_, first) = session().levels.borrow_mut().case(Some(key), frame, cells.clone()).at(span)?;
            let restricted = session().restricted(frame, first, level, truths.len(), rows, span);
            let result = restricted.and_then(|bindings| {
                let saved = session().bindings.swap(bindings);
                let step = session().run_definition(definition, if holds { (at.0, at.1 + 1) } else { (at.0 + 1, 0) }, false);
                session().bindings.swap(saved);
                step
            });
            session().levels.borrow_mut().end_case();
            match result? { Step::Done(bound) => bound.array(span), Step::Tail(_) => unreachable!("a case runs its last call") }
        })
    }

    /// The bindings of each frame from `first` to `frame`, with each batched value of run `level` read at a guard case's cells, the cells at positions `rows` of the run's `cells` cells (`batch::read_cells`), for `Bindings::swap`. These frames are the calls that started during the run. A closure that a case calls reads its defining frame, which can be any of them, whether or not it lies on `frame`'s lexical chain. Frames before `first` started before the run and hold none of its batches. A batch made before the run widened has fewer cells than the run, one for each run of cells that the widening's inner frame repeats it over (`read_binding`), so the case reads it at the cells that hold its rows.
    fn restricted(&self, frame: usize, first: usize, level: u64, cells: usize, rows: &[u32], span: &Span) -> Result<Vec<(usize, FrameBindings)>, Error> {
        let read = |whole: &Value, axes: usize| {
            let repeats = (cells / crate::array::count(&whole.shape()[..axes])) as u32;
            let rows: std::borrow::Cow<[u32]> = if repeats == 1 { rows.into() } else { rows.iter().map(|r| r / repeats).collect() };
            Ok((super::operators::cells_at(whole, axes, &rows, span)?, 1))
        };
        (first..=frame).map(|i| {
            let mut copy = self.bindings.frame_bindings(i);
            for value in copy.values_mut() { *value = crate::batch::read_cells(value, level, &read)?; }
            Ok((i, copy))
        }).collect()
    }

    /// Runs `run` in the batched run widened to `frame`, the run's frame followed by the frame of a map inside the run (`batch::tried`). Until it returns, a write to a binding in a frame that started before the widening stops the run (`transform::Levels::case`), and code reads each batch of the run that such a frame holds laid over `frame` (`read_binding`).
    pub(crate) fn widened<T>(&self, frame: &crate::array::Layout, span: &Span, run: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        self.levels.borrow_mut().case(None, self.bindings.depth(), frame.clone()).at(span)?;
        let result = run();
        self.levels.borrow_mut().end_case();
        result
    }

    /// `binding` as code reads it. Inside a widened batched run, each batch of the run that a binding made before the widening holds reads as laid over the widened frame (`batch::widened`). While a trace runs, a binding at `place` from outside the traced call's frames is a free name of the call (`program::trace::read`). Elsewhere a binding reads as it is.
    #[inline]
    pub(super) fn read_binding(&self, binding: Binding, place: impl FnOnce() -> Option<Place>) -> Result<Binding, Error> {
        if !self.transforming() { return Ok(binding); }
        let tracing = self.levels.borrow().tracing().map(|(level, tracer, frames)| (level, tracer.clone(), frames));
        if let Some((level, tracer, frames)) = tracing && let Some(place) = place() && place.frame().is_none_or(|f| f < frames) {
            return Ok(crate::program::trace::read(binding, place, level, &tracer));
        }
        let Binding::Value(value) = &binding else { return Ok(binding) };
        let levels = self.levels.borrow();
        let Some((level, frame)) = levels.widened() else { return Ok(binding) };
        crate::batch::widened(value, level, frame).map(Binding::Value)
    }
    /// What the binding at `place` holds now, as a program's replay reads a free name again (`program::Found`). `None` where nothing is bound there.
    pub(crate) fn captured(&self, place: Place) -> Option<crate::program::Found> {
        Some(match self.bindings.get(place)? {
            Binding::Value(Value::Function(f)) | Binding::Function(f) => crate::program::Found::Function(f),
            Binding::Value(v) => crate::program::Found::Value(v), _ => crate::program::Found::Other
        })
    }
}

/// The error for a predicate that isn't one Boolean, in a call and in each cell of a batched run.
const PREDICATE: &str = "a predicate requires a Boolean singleton";

/// The errors a guard catches: the kinds it names, or with `∞` every kind but an interrupt, a timeout or an unsupported feature. Unsupported features must not turn into plausible results. No guard catches a batched run's stop, even one that names its kind, because only the level that owns the run catches it. Inside a batched run, a guard that would catch an error stops the run instead (`transform::Levels::effect` with `Effect::Catches`). The run over cells then catches the error in the cells that raise it.
pub(super) enum Catch { All, Kinds(Vec<ErrorKind>) }
impl Catch {
    fn new(value: &Value, span: &Span) -> Result<Self, Error> {
        if value.observed().at(span)?.infinity() == Some(false) { return Ok(Self::All); }
        let invalid = || Error::domain("a guard names error kinds, as in \"DOMAIN\"::, or catches every kind with ∞::").at(span);
        let (_, names) = crate::keyed::text_items(value).ok_or_else(invalid)?;
        Ok(Self::Kinds(names.iter().map(|n| ErrorKind::named(n)).collect::<Option<_>>().ok_or_else(invalid)?))
    }
    pub(super) fn catches(&self, kind: &ErrorKind) -> bool {
        match self {
            Self::All => !kind.is_control() && !matches!(kind, ErrorKind::Unsupported),
            Self::Kinds(kinds) => !matches!(kind, ErrorKind::Stop { .. }) && kinds.iter().any(|k| k.name() == kind.name()),
        }
    }
}

/// `$e` in a handler: the caught error's kind and message, and the source, line and column where it happened.
fn caught(error: &Error) -> Value {
    let (line, column) = error.span.as_ref().map_or((0, 0), Span::position);
    let number = |n: usize| Value::integer(n as i64);
    let text = crate::keyed::text;
    let values = vec![text(error.kind.name()), text(&error.message), text(error.span.as_ref().map_or("", |s| &s.source.name)), number(line), number(column)];
    crate::keyed::vector(["kind", "message", "source", "line", "column"].map(Into::into).to_vec(), values).expect("an error fits in a record")
}

/// A bracket item's value as its list takes it (`Session::item_result`): `None` for an absent `⍺`, which drops out, or the value as an array, where `span` reports one that isn't.
pub(super) fn listed(value: Binding, span: &Span) -> Result<Option<Value>, Error> {
    if matches!(value, Binding::Absent) { Ok(None) } else { value.into_value(span).map(Some) }
}

/// What `Session::bracket` asks its `part` for: an item without a key, the value of a keyed item, with the position of its `:` and the key where the item's name gives it, or a key.
pub(super) enum Within { Item, Value { colon: usize, named: Option<Value> }, Key }
