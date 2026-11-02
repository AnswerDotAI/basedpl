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
    pub(super) fn blank() -> Self {
        Self { execution: Default::default(), prefacing: Default::default(), bindings: Default::default(), loading: Default::default(),
            depth: Default::default(), prototype: Default::default(), capture: Default::default(), levels: Default::default(), traces: Default::default(),
            generator: crate::distribution::fresh(), args: Default::default(), error: Default::default(), token: Default::default(), guards: Default::default() }
    }
    pub(crate) fn clear(&self) {
        use std::sync::PoisonError;
        self.bindings.replace(Bindings::default());
        self.prefacing.set(false); self.depth.set(0); self.prototype.set(false); self.capture.set(false);
        self.loading.borrow_mut().clear(); self.args.borrow_mut().clear(); self.guards.borrow_mut().clear();
        self.levels.clear(); self.traces.borrow_mut().clear(); self.error.take(); self.token.take();
        let generator = crate::distribution::fresh();
        std::mem::swap(&mut *self.generator.lock().unwrap_or_else(PoisonError::into_inner), &mut *generator.lock().unwrap_or_else(PoisonError::into_inner));
    }
    /// A new session's turn for a person at a terminal, with boxed display elided to the terminal's width.
    pub fn interactive() -> Turn { Self::make_interactive(); Self::new() }
    /// Makes the interactive display settings the current ones and the ones that `]clear` restores. Settings are process-wide, so this needs no session. Python's BPL magics use it on the workspace they share with the Python API.
    pub fn make_interactive() { crate::settings::set_display_defaults(crate::display::Settings::interactive()); }
    /// `value` as this session's implicit display shows it, as text. Display is a forcing read (`graph::forced`): a data error in the value's work raises here.
    pub fn show(&self, value: &Value) -> Result<String, Error> {
        self.within(|| {
            let value = value.read(crate::ReadKind::Value)?;
            self.raise()?;
            Ok(self.display_settings().array(&value, false))
        })
    }

    pub fn render(&self, value: &Value, interrupt: crate::InterruptHandle) -> Result<crate::MimeBundle, Error> {
        let _running = self.execution.begin(crate::EvalOptions { interrupt, echo: false, output: Some(Arc::new(|_| {})), ..crate::EvalOptions::default() });
        self.within(|| self.render_value(value, &Span::whole(Source::new("<display>", ""))))
    }
    /// The display settings, with floats of the default width shown without a marker.
    pub(crate) fn display_settings(&self) -> crate::display::Settings {
        let mut settings = crate::settings::display();
        settings.elide.plain = Some(crate::settings::float_width());
        settings
    }
    pub fn names(&self) -> Vec<String> { self.bindings.names() }
    pub(crate) fn capture_binding(&self, parent: Option<usize>, module: usize, name: Name) -> Result<Option<(Place, Value)>, Error> {
        let Some((place, binding)) = self.bindings.capture(parent, module, name) else { return Ok(None) };
        if matches!(binding, Binding::NoResult | Binding::Absent | Binding::Released) { return Ok(None); }
        let span = Span::whole(Source::new("<capture>", name.text()));
        Ok(Some((place, binding.into_value(&span)?)))
    }
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
            Some(NodeKind::Function(builtin)) => Binding::from(builtin.function()).inspection(self),
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
            Binding::Value(a) => ("array", display.array(&a, false)), binding @ Binding::Function(_) =>
                ("function", Function::from_value(binding, &Span::whole(Source::new("<inspect>", name.text()))).ok()?.text(display.elide, &mut 100)),
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
    pub(crate) fn guard_node(&self, node: &crate::graph::Node) {
        let mut guards = self.guards.borrow_mut();
        let Some(scope) = guards.last_mut() else { return };
        if scope.nodes.len() >= scope.live.saturating_mul(2).max(8) { scope.nodes.retain(|node| node.strong_count() > 0); scope.live = scope.nodes.len(); }
        scope.nodes.push(node.downgrade());
    }
    fn guarded(&self, value: Option<&Value>) -> Result<(), Error> {
        let scope = std::mem::take(self.guards.borrow_mut().last_mut().expect("a running error guard has a scope"));
        for node in scope.nodes.into_iter().filter_map(|node| node.upgrade()).map(crate::graph::Node) {
            for output in 0..node.descs().len() {
                Value::Pending(Pending { node: node.clone(), output: output as u32 }).read(crate::ReadKind::Value)?;
            }
        }
        if let Some(value) = value { value.read(crate::ReadKind::Value)?; }
        self.raise()
    }
    pub(super) fn writing(&self, place: Place, span: &Span) -> Result<(), Error> {
        if place.frame() != self.bindings.current() { crate::program::trace::Tracing::stop(crate::program::Reason::OuterWrite, Some(span.clone())); }
        self.levels.writing(place).map_err(|error| error.at(span))
    }
    pub fn erase(&self, name: &str) -> Result<bool, Error> {
        let Some(NodeKind::Name(name, _)) = crate::inspection::item(name, crate::settings::hanzi_aliases()) else { return Ok(false); };
        if name.implicit().is_some() { return Ok(false); }
        if let Some((place, _)) = self.bindings.lookup(name) {
            self.writing(place, &Span::whole(Source::new("<erase>", name.text())))?;
            self.bindings.remove(place);
        }
        Ok(true)
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
        self.named(right, span, |s, name| Ok(Value::integer(s.erase(name)? as i64)))
    }
    pub(crate) fn system_src(&self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.named(right, span, |s, name| s.name_source(name).map(|text| crate::keyed::text(&text)))
    }
    /// `•explain text` runs `text` in the caller's scope, as `⍎` does, computes its value, and gives a record of that value and of what happened while it ran and computed (`explain`). Computing a pending value records no `forced` row, as the speed timer's computing records none. Its options record: `"sites":$t` fills the `site` columns of `forced` and `stopped` rows with the code that computed the items or stopped the trace (`explain::site`).
    pub(crate) fn system_explain(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let sites = crate::data::Options::new("•explain", left, None, &["sites"])?.boolean("sites", false)?;
        let (result, events) = crate::explain::explained(sites, || {
            let result = self.execute(None, right, span).and_then(|r| r.array(span));
            let result = result.and_then(|value| crate::graph::forced(&[&value]).map(|()| value));
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
    pub(crate) fn system_time(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
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
    /// The fastest time per call of `f` on `x`, in seconds, including the computation of any items that the result defers (`Value::force`). Calls repeat for about 0.1 s, in batches that double until one takes 1 ms.
    fn fastest(&self, f: &Function, x: &Value, span: &Span) -> Result<f64, Error> {
        use {crate::host::Instant, std::time::Duration};
        let start = Instant::now();
        let (mut batch, mut best) = (1u32, f64::INFINITY);
        loop {
            let begun = Instant::now();
            for _ in 0..batch { if let Binding::Value(v) = &f.call(None, x, span)?.value { v.force(); } }
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
    pub fn set_function(&self, name: &str, value: Function) -> Result<(), ErrorKind> { self.set_value(name, value.into()) }
    fn set_value(&self, name: &str, value: Binding) -> Result<(), ErrorKind> {
        let Some(NodeKind::Name(name, _)) = crate::inspection::item(name, crate::settings::hanzi_aliases()) else { return Err(ErrorKind::Syntax); };
        if name.implicit().is_some() { return Err(ErrorKind::Syntax); }
        let place = self.bindings.at_top_level(Some(0), || self.bindings.here(name));
        self.store_at(place, value, &Span::whole(Source::new("<set>", name.text()))).map_err(|error| error.kind.clone())
    }
    pub fn eval(&self, code: &str) -> Evaluation { self.eval_with(code, crate::EvalOptions::default()) }
    pub fn eval_with(&self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.evaluate_source(Source::new("<input>", code)))
    }
    /// `code` evaluated in a module of its own, as `•load` runs a file: it sees none of the session's names and adds none. An expectation in the reference harness or a test evaluates this way, beside the code it checks.
    pub fn eval_apart(&self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.bindings.at_top_level(Some(s.bindings.new_module()), || s.evaluate_source(Source::new("<input>", code))))
    }
    /// One evaluation under `options`: `evaluate`'s result, with the output it produced. Its value leaves the session computed: a forcing read (`graph::forced`), whose error becomes the evaluation's.
    fn evaluation(&self, options: crate::EvalOptions, evaluate: impl FnOnce(&Self) -> Evaluation) -> Evaluation {
        let _running = self.execution.begin(options);
        let mut result = evaluate(self);
        if let Some(Err(error)) = result.value.as_ref().map(|value| crate::graph::forced(&[value])) { result = Evaluation { value: None, error: Some(error), ..result }; }
        self.execution.flush();
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
            Ok(Bound { value: Binding::Function(f), shy, .. }) => match Function::from_value(Binding::Function(f), &span) {
                Ok(f) => { if self.execution.echo.get() && !shy { self.display_function(&f, None); } result.function = Some(f); }
                Err(error) => result.error = Some(error),
            },
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
                            let f = match Function::from_value(Binding::Function(f), &nodes[0].span) { Ok(f) => f, Err(error) => { result.error = Some(error); return result; } };
                            if self.execution.echo.get() { self.display_function(&f, dissected.as_deref()); }
                            if i + 1 == parsed.statements.len() { result.function = Some(f); }
                            None
                        }
                        Binding::Operator(op) => {
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
            self.clear();
            crate::settings::reset();
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
    pub(super) fn bind(&self, nodes: &[Node]) -> Result<Bound, Error> { self.bound(nodes) }
    pub(super) fn bound(&self, nodes: &[Node]) -> Result<Bound, Error> {
        self.nested(nodes, |session| match session.statement_step(nodes, false)? {
            Step::Done(result) => Ok(result), Step::Tail(_) => unreachable!("a non-tail statement returns a value"),
        })
    }
    /// `f` evaluating `nodes` one level deeper, after an interrupt check, within the depth limit and on a stack that grows as needed.
    pub(super) fn nested<T>(&self, nodes: &[Node], f: impl FnOnce(&Self) -> Result<T, Error>) -> Result<T, Error> {
        crate::execution::check().at(&nodes[0].span)?;
        self.descend(&nodes[0].span)?;
        let result = if room() { f(self) } else { stacker::grow(STACK_SEGMENT, || f(self)) };
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
    pub(crate) fn mime(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
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
        let fallback = crate::keyed::vector(vec!["text/plain".into()], vec![crate::keyed::text(&self.show(right)?)]).unwrap();
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

    /// A value with a renderer formats through it. Other values, and a renderer that fails, format as text. Display is a forcing read and a checkpoint (`graph::forced`): a data error raises before any output, before a failing renderer's fallback could hide it.
    fn render_value(&self, value: &Value, span: &Span) -> Result<crate::MimeBundle, Error> {
        let value = value.read(crate::ReadKind::Value)?;
        self.raise()?;
        if value.renderer().is_some() {
            match self.mime_value(&value, span) {
                Ok(bundle) => {
                    let bundle = bundle.read(crate::ReadKind::Value)?;
                    self.raise()?;
                    return Ok(crate::display::bundle(&bundle).expect("•mime checks its bundle"));
                }
                Err(error) if error.kind.is_control() => return Err(error),
                Err(_) => (),
            }
        }
        Ok([("text/plain".into(), crate::MimeData::Text(self.display_settings().array(&value, false)))].into_iter().collect())
    }

    /// Displays a function using the trees setting, unless that view has already been shown.
    fn display_function(&self, f: &Function, shown: Option<&str>) {
        let display = crate::settings::display();
        let text = if display.trees { f.tree(display.elide, &mut 1000).render() } else { f.text(display.elide, &mut 1000) };
        if shown != Some(&text) { self.execution.output(crate::OutputKind::Display, text); }
    }

    /// Runs the text that `right` holds, as `⍎` does, with `left`'s keys as names where it's given. Under a trace, it stops the trace first (`program::trace::each_call`).
    pub(super) fn execute(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        crate::program::trace::Tracing::stop(crate::program::Reason::Execute, Some(span.clone()));
        let source = Source::new("<execute>", Self::source_text(right, span)?);
        match left { Some(record) => self.scoped(record, span, |session| session.execute_source(source, span)), None => self.execute_source(source, span) }
    }

    /// The character atom or vector that `⍎` and `•load` observe.
    fn source_text(right: &Value, span: &Span) -> Result<String, Error> {
        let right = right.read(crate::ReadKind::Value).at(span)?;
        if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "expected a unit or vector of characters").at(span)); }
        if !matches!(right.prototype(), Value::Character(_)) { return Err(Error::domain("expected characters").at(span)); }
        right.elements().map(|e| match e { Value::Character(c) => Ok(c), _ => Err(Error::domain("expected characters").at(span)) }).collect()
    }

    /// The `⎕` read's act (`program::Act::Input`, target part 17): the next line of input as text, or an empty vector at the end of input.
    pub(crate) fn line(&self, span: &Span) -> Result<Value, Error> {
        Ok(match self.execution.line(span)? {
            Some(line) => crate::keyed::text(&line),
            None => crate::syntax::zilde(crate::Number::float(0.0, crate::settings::float_width())),
        })
    }

    /// `•load path` runs a file in a module of its own and returns a record of the module's public names: the names its top level assigns that don't start with `_`, in alphabetical order. The file adds no names to the caller, and shows nothing except explicit output.
    pub(crate) fn load(&self, right: &Value, span: &Span) -> Result<Bound, Error> {
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

    /// The role (`binder::role`) of `name`'s binding, or of the field that `keys` reach from the array it holds (`binder::field_role`), as the compiler binds it, read without evaluating anything; an unbound name binds as an array. While a statement compiles, `log` keeps the fact that it read (`tree::Fact`), except for a name whose role no assignment can change (`Name::fixed`). The 1 home of a compile-time role read: the compiler's names and dot reads, and `path_category`.
    pub(super) fn path_role(&self, name: Name, keys: &[Arc<str>], log: &mut super::tree::Facts) -> (Category, bool) {
        let role = if keys.is_empty() { self.bindings.role(name).unwrap_or(super::binder::UNBOUND) }
            else { super::binder::field_role(self.bindings.lookup(name).map(|(_, b)| b), keys) };
        if let Some(log) = log && !(keys.is_empty() && name.fixed()) {
            let fact = super::tree::Fact::new(self.bindings.address(name), keys.into(), role);
            if !log.contains(&fact) { log.push(fact); }
        }
        role
    }

    /// The entries of the keyed item in cell `cell` of the bracket literal `node`, whose `:` is at `colon`: `Keys` applies `key` to `value`, which must give a keyed vector. A name with a batch of atoms gives one entry that holds the batch, as `Keys` gives one entry in each cell, so a record built in a batched run holds batches as a list literal does. The level that finishes a compiled keyed item calls it as soon as the item's value and key are both ready (`tree::Keyed`). An item that fails there stops the items after it.
    pub(super) fn keyed(&self, node: &Node, cell: usize, colon: usize, key: &Value, value: &Value) -> Result<Vec<crate::keyed::Entry>, Error> {
        let NodeKind::ArrayLiteral { cells, .. } = &node.kind else { unreachable!("a bracket literal") };
        if let Some(name) = crate::keyed::name(key) && value.rank() == 0 {
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
        let result = if *form == ListForm::Cells { Operation::Join(crate::primitive::Notation::Rows).call(&arrays, 0) }
            else { Value::new(vec![arrays.len()], arrays).map_err(Error::from) };
        Ok(Binding::Value(result.map_err(|error| error.at(&node.span))?))
    }

    // Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
    #[inline(never)]
    pub(super) fn call_named(&self, function: &Function, origin: crate::program::Origin<&[Value]>) -> Result<Value, Error> {
        let FunctionNode::LateBound(name) = function.node() else { unreachable!("a late-bound name") };
        let span = Span::whole(Source::new("<call>", name.text()));
        let position = origin.bindings.scope(function).and_then(|(_, scope)| scope.names.iter().find(|entry| entry.name == *name)).and_then(|entry| entry.position.as_ref())
            .ok_or_else(|| undefined(*name, &span))?;
        let function = Function::from_value(Binding::from_element(Function::capture_at(&origin, position).into_owned()), &span)?;
        let arguments = Function::body_arguments(&origin)?;
        let from: Vec<_> = Function::argument_ranges(&origin)?.into_iter().flatten().map(|range| Some(range.start)).collect();
        function.call_bound(&origin, &arguments, &from)
    }
    pub(super) fn with_call_context<T>(&self, function: &Function, origin: crate::program::Origin<&[Value]>, run: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        if function.definition().is_some() || !origin.bindings.scopes.iter().any(|scope| !scope.names.is_empty()) { return run(); }
        let base = self.bindings.depth();
        let caller = self.bindings.current();
        let span = origin.site.clone().unwrap_or_else(|| Span::whole(Source::new("<call>", "")));
        self.bindings.enter_context(origin, &span)?;
        let result = run();
        self.bindings.leave(base, caller);
        result
    }
    pub(super) fn call_defined(&self, function: &Function, origin: crate::program::Origin<&[Value]>) -> Result<Value, Error> {
        let defined = match function.node() { FunctionNode::Defined(_) => function, FunctionNode::Applied { op: Operator::Closure(defined), .. } => defined, _ => unreachable!("a defined body") };
        let definition = defined.definition().expect("a defined body");
        let (scope_index, scope) = origin.bindings.scope(defined).ok_or_else(Error::planning)?;
        let (parent, module) = (scope.parent, scope.module);
        let args = Function::body_arguments(&origin)?;
        let (left, right) = (if args.len() == 2 { Some(args[0].clone()) } else { None }, args.last().expect("a function has an argument").clone());
        let operands = function.node().operands();
        let operand = |i| (i < operands.len()).then(|| Binding::from_element(function.operand(&origin, i).into_owned()));
        let implicit = [Some(left.map_or(Binding::Absent, Binding::Value)), Some(Binding::Value(right)), operand(0), operand(1),
            Some(function.clone().into()), matches!(function.node(), FunctionNode::Applied { op: Operator::Closure(_), .. })
                .then(|| Binding::Operator(OperatorValue::Operator(Operator::Closure(defined.clone()))))];
        let (base, caller) = (self.bindings.depth(), self.bindings.current());
        self.bindings.enter(Frame::new(parent, module, definition.locals()), implicit, Some((Some(scope_index), origin.reborrow())), &definition.span)?;
        let result = match self.run_definition(definition, (0, 0), true) {
            Ok(Step::Done(bound)) => Ok(bound),
            Ok(Step::Tail(call)) => {
                let value = Operation::dispatch(call.origin.reborrow()).and_then(|outputs| Bound::from_outputs(outputs, &call.span)).map_err(|error| error.at(&call.span));
                value.map(|mut bound| { if call.unshy { bound.shy = false; } bound })
            }
            Err(error) => Err(error),
        };
        self.leave(base, caller, result, &definition.span)?.array(&definition.span)
    }

    /// Drops the frames from `base` on, makes `caller` current and gives back `result`. A result that refers to a dropped frame is an error.
    fn leave(&self, base: usize, caller: Option<usize>, result: Result<Bound, Error>, span: &Span) -> Result<Bound, Error> {
        let result = result.and_then(|mut bound| {
            if matches!(bound.value, Binding::Function(_)) { bound.value = Function::from_value(bound.value, span)?.into(); }
            Ok(bound)
        });
        let result = match result {
            Ok(bound) if bound.value.environment().is_some_and(|i| i >= base) => Err(Error::domain("result would return a local closure").at(span)),
            result => result,
        };
        self.bindings.leave(base, caller);
        result
    }

    /// Runs `f` in a new frame in which each key on the first axis of `record` names what `T.key` selects: an item of a vector, or a major cell of a higher-rank array. Other names resolve in the caller's scope, and assignments stay in the new frame. `T.(expr)` and `T⍎text` evaluate this way.
    pub(super) fn scoped(&self, record: &Value, span: &Span, f: impl FnOnce(&Self) -> Result<Bound, Error>) -> Result<Bound, Error> {
        record.keys(0).ok_or_else(|| ErrorKind::from(Kind::Domain)).described("a scope needs keys on its first axis").at(span)?;
        let (caller, base) = (self.bindings.current(), self.bindings.depth());
        self.bindings.enter(Frame::scope(caller, self.bindings.module(), record.clone()), Default::default(), None, span)?;
        let result = f(self);
        self.leave(base, caller, result, span)
    }

    /// The last statement or an error handler supplies the dfn's result.
    fn return_expression(&self, mut nodes: &[Node], tail: bool) -> Result<Step, Error> {
        if nodes.is_empty() { return Ok(Step::Done(Bound::new(Binding::NoResult))); }
        let mut unshy = false;
        while let [Node { kind: NodeKind::Group(inner), .. }] = nodes {
            nodes = inner;
            unshy = true;
        }
        let step = self.statement_step(nodes, tail)?;
        match step {
            Step::Done(mut bound) => { if unshy { bound.shy = false; } bound.result(&nodes[0].span).map(Step::Done) }
            Step::Tail(mut call) => { call.unshy = unshy; Ok(Step::Tail(call)) }
        }
    }

    /// Runs the current call's statements, forcing guarded work before a handler can catch its checked errors.
    fn run_definition(&self, definition: &Definition, start: (usize, usize), tail: bool) -> Result<Step, Error> {
        let mut handlers = Vec::new();
        let result = (|| {
            'bodies: for (b, body) in definition.bodies.iter().enumerate().skip(start.0) {
                for (position, statement) in body.iter().enumerate().skip(if b == start.0 { start.1 } else { 0 }) {
                    let nodes = &statement.nodes;
                    match statement.kind {
                        StatementKind::DefaultArgument => {
                            if nodes.len() == 2 { return Err(Error::new(ErrorKind::Syntax, "default argument needs a value").at(&nodes[1].span)); }
                            if self.bindings.category(Name::ALPHA) == Some(Category::Absent) {
                                let value = self.bound(&nodes[2..])?.value;
                                if matches!(value, Binding::NoResult | Binding::Absent) {
                                    return Err(Error::new(ErrorKind::Value, "default argument requires a value").at(&nodes[0].span));
                                }
                                self.bindings.insert(self.bindings.here(Name::ALPHA), value);
                            }
                        }
                        StatementKind::ErrorGuard { index: i } => {
                            crate::program::trace::Tracing::stop(crate::program::Reason::Guard, Some(nodes[i].span.clone()));
                            let catch = Catch::new(&self.bound(&nodes[..i])?.array(&nodes[0].span)?, &nodes[i].span)?;
                            if handlers.is_empty() { self.guards.borrow_mut().push(super::GuardScope::default()); } else { self.guarded(None)?; }
                            handlers.push((&nodes[i + 1..], catch));
                        }
                        StatementKind::Predicate => {
                            let span = crate::syntax::cover(nodes);
                            let value = self.bound(nodes)?.array(&nodes[0].span)?;
                            let holds = value.read(crate::ReadKind::Value)?.boolean().described(PREDICATE).at(&span)?;
                            if !holds { continue 'bodies; }
                        }
                        StatementKind::Expression => {
                            if position + 1 == body.len() { return self.return_expression(nodes, tail && handlers.is_empty()); }
                            self.bound(nodes)?;
                        }
                    }
                }
                return Ok(Step::Done(Bound::new(Binding::NoResult)));
            }
            Err(Error::domain("every predicate failed, so no body applies").at(&definition.span))
        })();
        let mut result = result;
        if !handlers.is_empty() {
            if let Ok(Step::Done(bound)) = &result && let Err(error) = self.guarded(match &bound.value { Binding::Value(value) => Some(value), _ => None }) { result = Err(error); }
            self.guards.borrow_mut().pop();
        }
        while let Err(error) = &result {
            let Some((handler, catch)) = handlers.pop() else { break };
            if !catch.catches(&error.kind) { continue; }
            if let Some(level) = crate::transform::Level::innermost_batched() { return Err(Error::batch_stop(level.id)); }
            self.bindings.insert(self.bindings.here(Name::ERROR), Binding::Value(caught(error)));
            result = self.return_expression(handler, false);
        }
        result
    }

}

/// The error for a predicate that isn't one Boolean, in a call and in each cell of a batched run.
const PREDICATE: &str = "a predicate requires a Boolean singleton";

/// The errors a guard catches: the kinds it names, or with `∞` every kind but an interrupt, a timeout or an unsupported feature. Unsupported features must not turn into plausible results. No guard catches a batched run's stop, even one that names its kind, because only the level that owns the run catches it. Inside a batched run, a guard that would catch an error stops the run instead (`transform::Levels::effect` with `Effect::Catches`). The run over cells then catches the error in the cells that raise it.
pub(super) enum Catch { All, Kinds(Vec<ErrorKind>) }
impl Catch {
    fn new(value: &Value, span: &Span) -> Result<Self, Error> {
        if value.read(crate::ReadKind::Value)?.as_real() == Some(f64::INFINITY) { return Ok(Self::All); }
        let invalid = || Error::domain("a guard names error kinds, as in \"DOMAIN\"::, or catches every kind with ∞::").at(span);
        let (_, names) = crate::keyed::text_items(value).ok_or_else(invalid)?;
        Ok(Self::Kinds(names.iter().map(|n| ErrorKind::named(n)).collect::<Option<_>>().ok_or_else(invalid)?))
    }
    pub(super) fn catches(&self, kind: &ErrorKind) -> bool {
        if kind.is_control() { return false; }
        match self { Self::All => !matches!(kind, ErrorKind::Unsupported), Self::Kinds(kinds) => kinds.iter().any(|k| k.name() == kind.name()) }
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

/// A bracket item's value as its literal takes it: `None` for an absent `⍺`, which drops out, or the value as an array, where `span` reports one that isn't.
pub(super) fn listed(value: Binding, span: &Span) -> Result<Option<Value>, Error> {
    if matches!(value, Binding::Absent) { Ok(None) } else { value.into_value(span).map(Some) }
}
