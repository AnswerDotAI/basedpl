//! The session's evaluation API, names, display, file loading and name lookup, and calls of dfns with their error guards.

use rustymath::{buffer::FloatWidth, error::Kind};
use crate::syntax::Statement;
use super::*;

impl Session {
    pub fn new() -> Self { Self::default() }
    /// This session with `args` as the program's command-line arguments, which `•host "args"` gives.
    pub fn with_args(self, args: Vec<String>) -> Self { Self { args, ..self } }
    /// This session at `span`, for a call that takes a `Context`.
    pub(crate) fn at<'a>(&'a mut self, span: &'a Span) -> Context<'a> { Context { span, session: self } }
    /// Whether the `batch` pref is on. While it is, maps first try one call of their operand on batches of all their cells (`batch::tried`). A session starts with it on.
    pub(crate) fn batching(&self) -> bool { self.batch.unwrap_or(true) }
    /// A session for a person at a terminal, with boxed display elided to the terminal's width.
    pub fn interactive() -> Self {
        let mut session = Self::default();
        session.make_interactive();
        session
    }
    /// Gives this session an interactive session's display settings, both as its current settings and as the defaults that `]clear` restores. Python's BPL magics use it on the workspace they share with the Python API.
    pub fn make_interactive(&mut self) {
        self.display = crate::display::Settings::interactive();
        self.display_defaults = self.display;
    }
    /// `value` as this session's implicit display shows it, as text.
    pub fn show(&self, value: &Value) -> String { self.display_settings().array(value, false) }
    /// The display settings, with floats of the default width shown without a marker.
    pub(crate) fn display_settings(&self) -> crate::display::Settings {
        let mut settings = self.display;
        settings.elide.plain = Some(self.numeric.width);
        settings
    }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let mut names = HashSet::new();
        let mut scope = self.current;
        while let Some(i) = scope {
            names.extend(self.frames[i].names.keys().map(String::as_str));
            scope = self.frames[i].parent;
        }
        names.extend(self.table(self.module()).keys().map(String::as_str));
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort_unstable();
        names.into_iter()
    }
    pub fn name_class(&self, name: &str) -> i64 {
        match crate::inspection::item(name, self.hanzi_aliases) {
            Some(NodeKind::Name(name)) => self.lookup(&name).map_or(0, Binding::class),
            Some(NodeKind::System(name)) => crate::system::lookup(&name).map_or(0, |f| Binding::Function(f).class()), _ => -1
        }
    }
    pub fn name_list(&self, classes: &[i64], prefix: &str) -> Vec<String> {
        self.names().filter(|name| name.starts_with(prefix) && classes.contains(&self.name_class(name))).map(str::to_owned).collect()
    }
    pub fn inspect(&self, name: &str) -> Option<crate::Inspection> {
        let info = match crate::inspection::item(name, self.hanzi_aliases) {
            Some(NodeKind::Name(name)) => self.lookup(&name).and_then(|binding| binding.inspection(self)),
            Some(NodeKind::System(name)) => {
                let mut info = Binding::Function(crate::system::lookup(&name)?).inspection(self)?;
                if let Some(help) = crate::inspection::documentation(&name) { info.help = help.to_owned(); }
                Some(info)
            }
            Some(NodeKind::Function(p)) => Some(crate::Inspection::new("function", p.glyph().to_string())),
            Some(NodeKind::Operator(op)) => Some(crate::Inspection::new("operator", op.glyph().to_string())),
            _ => None,
        };
        info.or_else(|| crate::inspection::documentation(name).map(|_| crate::Inspection::new("syntax", name.into())))
    }
    pub fn name_source(&self, name: &str) -> Result<String, ErrorKind> {
        match self.name_class(name) {
            FUNCTION_CLASS | OPERATOR_CLASS => Ok(self.inspect(name).unwrap().source), SUBJECT_CLASS => Err(Kind::Domain.into()), _ => Err(ErrorKind::Value)
        }
    }
    /// The module whose names code sees after its frames: the module that defined the running function, or the module whose top level is running. Module 0 is the session's own names.
    fn module(&self) -> usize { self.current.map_or(self.module, |i| self.frames[i].module) }
    fn table(&self, module: usize) -> &HashMap<String, Binding> { if module == 0 { &self.names } else { &self.modules[module - 1] } }
    fn table_mut(&mut self, module: usize) -> &mut HashMap<String, Binding> { if module == 0 { &mut self.names } else { &mut self.modules[module - 1] } }
    /// Binds `name` in the frame `owner`, or in the running module's names when `owner` is `None`.
    pub(super) fn insert_name(&mut self, owner: Option<usize>, name: &str, value: Binding) {
        match owner { Some(i) => self.frames[i].insert(Key::of(name), value), None => { self.table_mut(self.module()).insert(name.to_owned(), value); } }
    }
    /// Removes the binding of `name` from the frame `owner`, or from the running module's names when `owner` is `None`.
    pub(super) fn remove_name(&mut self, owner: Option<usize>, name: &str) -> Option<Binding> {
        match owner { Some(i) => self.frames[i].remove(Key::of(name)), None => self.table_mut(self.module()).remove(name) }
    }
    /// Whether a level whose kind wraps values is running (`transform::Levels::transforming`).
    pub(crate) fn transforming(&self) -> bool { self.levels.transforming() }
    /// Starts a level of `kind` and returns its id, which is larger than every running level's. A `∂` call's id is its tag, and a record level's id is the level its records carry.
    pub(crate) fn begin(&mut self, kind: crate::transform::Kind) -> u64 { self.levels.begin(kind, self.frames.len()) }
    /// Ends the innermost level, applying its kind's exit rule to each binding outside its frames that it wrote (`transform::Kind::exit`).
    pub(crate) fn end(&mut self) {
        let level = self.levels.end();
        let Some(exit) = level.kind.exit() else { return };
        for (owner, module, name) in level.writes {
            let value = match owner { Some(i) => self.frames.get(i).and_then(|f| f.get(Key::of(&name))), None => self.table(module).get(&name) };
            let Some(Binding::Value(v)) = value else { continue };
            let v = Binding::Value(exit(v, level.id));
            match owner { Some(_) => self.insert_name(owner, &name, v), None => { self.table_mut(module).insert(name, v); } }
        }
    }
    /// Records a write to the binding of `name` in `owner`, or in the running module's names when `owner` is `None`, for each running level whose exit acts on it, or stops a batched map's run before it (`transform::Levels::record_write`).
    pub(super) fn record_write(&mut self, owner: Option<usize>, name: &str) -> Result<(), crate::transform::Stop> {
        let module = self.module();
        self.levels.record_write(owner, module, name)
    }
    pub fn erase(&mut self, name: &str) -> bool {
        let Some(NodeKind::Name(name)) = crate::inspection::item(name, self.hanzi_aliases) else { return false; };
        if implicit_name(&name) { return false; }
        if let Some((owner, _)) = self.binding(&name) { self.remove_name(owner, &name); }
        true
    }
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        let mut names = self.name_list(&[2, 3, 4], prefix);
        names.extend(crate::system::names().filter(|name| name.starts_with(&prefix.to_lowercase())).map(str::to_owned));
        names.sort_unstable();
        names
    }
    /// `f` applied to the name that `right` holds. The system table maps `•nc`, `•ex` and `•src` over an array of names.
    fn named(&mut self, right: &Value, span: &Span, f: impl FnOnce(&mut Self, &str) -> Result<Value, ErrorKind>) -> Result<Value, Error> {
        let name = crate::keyed::name(right).ok_or_else(|| span.domain_error("expected a name string"))?;
        f(self, &name).map_err(|kind| span.error(kind, format!("cannot inspect name: {name}")))
    }
    pub(crate) fn system_nc(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.named(right, span, |s, name| Ok(Value::integer(s.name_class(name))))
    }
    pub(crate) fn system_ex(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.named(right, span, |s, name| Ok(Value::integer(s.erase(name) as i64)))
    }
    pub(crate) fn system_src(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.named(right, span, |s, name| s.name_source(name).map(|text| crate::keyed::text(&text)))
    }
    /// `•explain text` runs `text` in the caller's scope, as `⍎` does, and gives a record of its value and of what happened while it ran (`explain`).
    pub(crate) fn system_explain(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let outer = crate::explain::start();
        let explaining = std::mem::replace(&mut self.explaining, true);
        let result = self.execute(None, right, span);
        self.explaining = explaining;
        let events = crate::explain::finish(outer);
        events.report(result?.array(span)?, span)
    }
    /// `•literal text` reads a value from BPL source, as `•literal⁻¹` writes it, without running code. The text may hold literals and the functions that `•literal⁻¹` writes: `⍴`, `⊂`, `,`, `:` and `•ucs`.
    pub(crate) fn system_literal(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let parsed = self.parse(Source::new("<•literal>", Self::source_text(right, span)?)).complete()?;
        let [statement] = parsed.statements.as_slice() else { return Err(span.domain_error("•literal reads one value")) };
        let written = |kind: &NodeKind| {
            matches!(kind, NodeKind::Function(Primitive::Shape | Primitive::Enclose | Primitive::Ravel))
                || matches!(kind, NodeKind::System(name) if name.eq_ignore_ascii_case("•ucs"))
        };
        if let Some(node) = crate::syntax::non_literal(&statement.nodes, &written) {
            return Err(span.domain_error(format!("•literal reads data, not code: {}", &node.span.source.text[node.span.range.clone()])));
        }
        self.bind(&statement.nodes)?.array(span)
    }
    /// `•time t` gives the seconds since `t`, counting from the Unix epoch. `F •time x` gives each function's fastest time per call on `x`, in the layout of `F`.
    pub(super) fn system_time(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        // Times stay 64-bit, as `•date` gives them, because a 32-bit float holds seconds since 1970 only to within about two minutes.
        let second = |s| Value::Number(crate::Number::float(s, FloatWidth::F64));
        let Some(functions) = left else {
            let now = second(crate::date::now());
            return Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(Some(&now), right, &self.at(span));
        };
        let times = functions.elements().map(|f| match f {
            Value::Function(f) => self.fastest(&f, right, span), _ => Err(span.domain_error("•time needs functions on its left"))
        }).collect::<Result<Vec<_>, _>>()?;
        if functions.is_atom() { return Ok(second(times[0])); }
        Value::floats(functions.shape().to_vec(), FloatWidth::F64, times).and_then(|t| t.with_layout(functions.layout().clone()))
            .error_at(span, "invalid •time result")
    }
    /// The fastest time per call of `f` on `x`, in seconds. Calls repeat for about 0.1 s, in batches that double until one takes 1 ms.
    fn fastest(&mut self, f: &Function, x: &Value, span: &Span) -> Result<f64, Error> {
        use {crate::host::Instant, std::time::Duration};
        let start = Instant::now();
        let (mut batch, mut best) = (1u32, f64::INFINITY);
        loop {
            let begun = Instant::now();
            for _ in 0..batch { f.call(None, x, &mut self.at(span))?; }
            let taken = begun.elapsed();
            best = best.min(taken.as_secs_f64() / f64::from(batch));
            if start.elapsed() >= Duration::from_millis(100) { return Ok(best); }
            if taken < Duration::from_millis(1) { batch *= 2; }
        }
    }
    pub(crate) fn system_nl(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let prefix = match left {
            Some(value) => crate::keyed::name(value).ok_or_else(|| span.domain_error("•nl prefix must be a string"))?, None => "".into()
        };
        if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "•nl needs a class or class vector")); }
        let classes = right.as_items().integers().error_at(span, "invalid name class")?;
        if classes.iter().any(|n| !(SUBJECT_CLASS..=OPERATOR_CLASS).contains(n)) { return Err(span.domain_error("name classes are 2, 3 and 4")); }
        let values = self.name_list(&classes, &prefix).iter().map(|n| crate::keyed::text(n)).collect::<Vec<_>>();
        crate::keyed::texts(&[values.len()], values).error_at(span, "invalid name list")
    }
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), ErrorKind> { self.set_value(name, Binding::from_element(value)) }
    pub fn set_function(&mut self, name: &str, value: Function) -> Result<(), ErrorKind> { self.set_value(name, Binding::Function(value)) }
    fn set_value(&mut self, name: &str, value: Binding) -> Result<(), ErrorKind> {
        let Some(NodeKind::Name(name)) = crate::inspection::item(name, self.hanzi_aliases) else { return Err(ErrorKind::Syntax); };
        if implicit_name(&name) { return Err(ErrorKind::Syntax); }
        self.names.insert(name, value);
        Ok(())
    }
    pub fn eval(&mut self, code: &str) -> Evaluation { self.eval_with(code, crate::EvalOptions::default()) }
    pub fn eval_with(&mut self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.evaluate_source(Source::new("<input>", code)))
    }
    /// One evaluation under `options`: `evaluate`'s result, with the output it produced. Its rustymath calls split across the session's `threads` where `•prefs` set them, and across the thread's own setting otherwise. It makes the session's numeric settings the thread's current ones, and leaves them in force after it returns. It restores the thread's own thread count.
    fn evaluation(&mut self, options: crate::EvalOptions, evaluate: impl FnOnce(&mut Self) -> Evaluation) -> Evaluation {
        let threads = rustymath::parallel::thread_setting();
        if let Some(n) = self.threads { rustymath::parallel::set_threads(n); }
        self.numeric.set_current();
        self.execution.begin(options);
        let result = evaluate(self);
        self.execution.flush();
        self.stacks = Vec::new();
        rustymath::parallel::set_threads(threads);
        Evaluation { output: self.execution.take_output(), ..result }
    }
    pub fn eval_timeout(&mut self, code: &str, timeout: std::time::Duration) -> Evaluation {
        self.eval_with(code, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() })
    }
    /// Resolve a function expression in this session and call it with one (right) or two (left, right) arrays. No function or lexical-frame handle escapes the call, and no temporary names are bound.
    pub fn call(&mut self, function: &str, args: &[Value]) -> Evaluation { self.call_with(function, args, crate::EvalOptions::default()) }
    pub fn call_with(&mut self, function: &str, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        match self.late_bound(function) { Ok(f) => self.call_function_with(&f, args, options), Err(error) => Evaluation::failed(error) }
    }
    /// The function that `expression` writes, late-bound, with its numbers read at the session's default width.
    pub fn late_bound(&self, expression: &str) -> Result<Function, Error> {
        let span = Span::whole(Source::new("<call>", expression));
        let parsed = self.parse(span.source.clone()).complete()?;
        if parsed.statements.len() != 1 { return Err(span.error(ErrorKind::Syntax, "call requires one function expression")); }
        Function::new(FunctionNode::LateBound(Arc::new(parsed), span.clone()), &span)
    }
    pub fn call_function_with(&mut self, function: &Function, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.call_function(function, args))
    }
    fn call_function(&mut self, function: &Function, args: &[Value]) -> Evaluation {
        let span = Span::whole(Source::new("<call>", function.bpl()));
        let mut result = Evaluation::default();
        let called = (|| {
            let (left, right) = match args {
                [right] => (None, right), [left, right] => (Some(left), right),
                _ => return Err(span.error(Kind::Length, "call requires one or two arguments"))
            };
            function.call(left, right, &mut self.at(&span))?.result(&span)
        })();
        match called {
            Ok(Bound { value: Binding::Value(a), shy, .. }) => {
                if self.execution.echo && !shy { if let Err(e) = self.display_value(&a, &span) { result.error = Some(e); } }
                result.value = Some(a);
            }
            Ok(Bound { value: Binding::Function(f), shy, .. }) => {
                if self.execution.echo && !shy { self.display_function(&f, None); }
                result.function = Some(f);
            }
            Ok(_) => (),
            Err(e) => result.error = Some(e),
        }
        result
    }
    pub fn eval_source(&mut self, source: Arc<Source>, options: crate::EvalOptions) -> Evaluation { self.evaluation(options, |s| s.evaluate_source(source)) }
    /// A line that starts with `]` is a command, unless it closes brackets that the code before it left open. Each part keeps its line numbers, so errors point at the right line.
    fn evaluate_source(&mut self, source: Arc<Source>) -> Evaluation {
        let (mut parts, mut start, mut offset) = (Vec::new(), 0, 0);
        for line in source.text.split_inclusive('\n') {
            let pending = &source.text[start..offset];
            if line.trim_start().starts_with(']')
                && (pending.trim().is_empty() || !matches!(self.parse(Source::new("", pending)), ParseStatus::Incomplete(_))) {
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
    fn evaluate_code(&mut self, source: Arc<Source>) -> Evaluation {
        match self.parse(source).complete() { Ok(parsed) => self.eval_display(&parsed, false, false), Err(e) => Evaluation::failed(e) }
    }
    /// The program in `source`, with numbers that have no marker read as floats of the session's default width, or of the width that a leading `•prefs` statement on a literal record sets. The parse makes the session's numeric settings the thread's current ones.
    pub fn parse(&self, source: Arc<Source>) -> crate::ParseStatus { self.numeric.set_current(); crate::syntax::parse_with_aliases(source, self.hanzi_aliases) }
    /// The default width of floats, which the `float` pref sets.
    pub fn float_width(&self) -> FloatWidth { self.numeric.width }
    /// Whether this session reads Hanzi as glyph aliases instead of name characters.
    pub fn hanzi_aliases(&self) -> bool { self.hanzi_aliases }
    pub fn eval_parsed(&mut self, parsed: &Parsed, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.eval_display(parsed, false, false))
    }
    fn eval_display(&mut self, parsed: &Parsed, diagram: bool, dissect: bool) -> Evaluation {
        let mut result = Evaluation::default();
        let mut leading = true;
        for (i, statement) in parsed.statements.iter().enumerate() {
            (result.function, result.operator) = (None, None);
            let nodes = &statement.nodes;
            let previous = self.capture;
            let dissecting = self.depth == 0 && (dissect || self.display.dissect && self.execution.echo);
            self.capture = dissecting;
            leading &= crate::syntax::prefs_record(nodes).is_some();
            let bound = self.bind_statement(statement, leading);
            self.capture = previous;
            match bound {
                Ok(bound) => {
                    let dissected = if dissecting && (dissect || !bound.shy && !matches!(bound.value, Binding::NoResult | Binding::Absent)) {
                        bound.expression.map(|e| *e).or_else(|| Expression::binding(&bound.value))
                            .map(|e| e.display(self.display.trees, self.display.elide.notation))
                    } else { None };
                    if let Some(text) = &dissected { self.execution.output(crate::OutputKind::Display, text.clone()); }
                    result.value = match bound.value {
                        Binding::NoResult | Binding::Absent => None,
                        Binding::Value(a) => {
                            if diagram && i + 1 == parsed.statements.len() {
                                self.execution.output(crate::OutputKind::Display, self.display_settings().diagram(&a));
                            } else if self.execution.echo && !bound.shy {
                                if let Err(e) = self.display_value(&a, &nodes[0].span) { result.error = Some(e); return result; }
                            }
                            Some(a)
                        }
                        _ if bound.shy => None,
                        Binding::Function(f) => {
                            if self.execution.echo { self.display_function(&f, dissected.as_deref()); }
                            if i + 1 == parsed.statements.len() { result.function = Some(f); }
                            None
                        }
                        Binding::Operator(op) => {
                            let op = Operator(Arc::new(op));
                            if self.execution.echo { self.execution.output(crate::OutputKind::Display, op.text(self.display.elide.notation)); }
                            if i + 1 == parsed.statements.len() { result.operator = Some(op); }
                            None
                        }
                    };
                }
                Err(e) => {
                    result.value = None;
                    result.error = Some(e);
                    break;
                }
            }
        }
        result
    }
    fn command(&mut self, source: Arc<Source>) -> Evaluation {
        let span = Span::whole(source);
        let failed = |kind, message: String| Evaluation::failed(span.error(kind, message));
        let code = span.source.text.trim();
        let (command, args) = code.split_once(char::is_whitespace).unwrap_or((code, ""));
        if command.eq_ignore_ascii_case("]help") {
            let usage = "usage: ]help name [-source], or ]help llms for the documentation index for LLMs";
            let Some((name, detail)) = crate::inspection::help_command(code) else { return failed(ErrorKind::Syntax, usage.into()) };
            // `llms` is a topic, not a name. A variable called `llms` can't hide the reference.
            let (text, markdown) = if name == "llms" { (crate::inspection::LLMS.to_owned(), crate::inspection::LLMS.to_owned()) } else {
                let Some(info) = self.inspect(name) else { return failed(ErrorKind::Value, format!("name not found: {name}")) };
                (info.text(detail), info.markdown(detail))
            };
            let data = [("text/plain", text), ("text/markdown", markdown)].map(|(kind, text)| (kind.to_string(), crate::MimeData::Text(text))).into();
            self.execution.emit(crate::Output { kind: crate::OutputKind::Display, data });
            return Evaluation::default();
        }
        if command.eq_ignore_ascii_case("]clear") {
            if !args.is_empty() { return failed(ErrorKind::Syntax, "usage: ]clear".into()); }
            let display = self.display_defaults;
            *self = Self { execution: std::mem::take(&mut self.execution), display, display_defaults: display, ..Self::default() };
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
    pub(super) fn bind(&mut self, nodes: &[Node]) -> Result<Bound, Error> { self.bound(nodes, None) }
    /// `nodes` evaluated as one statement, one level deeper (`nested`), through `slot` as `statement_step` uses it.
    pub(super) fn bound(&mut self, nodes: &[Node], slot: Option<&std::sync::OnceLock<Option<super::Recording>>>) -> Result<Bound, Error> {
        self.nested(nodes, |session| {
            let Step::Done(result) = session.statement_step(slot, nodes, false)? else { unreachable!() };
            Ok(result)
        })
    }
    /// `f` evaluating `nodes` one level deeper, after an interrupt check, within the depth limit and on a stack that grows as needed.
    pub(super) fn nested<T>(&mut self, nodes: &[Node], f: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, Error> {
        self.execution.check(&nodes[0].span)?;
        if self.depth == MAX_CALL_DEPTH { return Err(depth_error(&nodes[0].span)); }
        self.depth += 1;
        let result = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || f(self));
        self.depth -= 1;
        result
    }

    /// `•mime Y` is the MIME bundle that displays `Y`. `X •mime Y` is `Y` displayed through `X`, which holds a function or names a MIME type. A MIME type shows `Y` itself as that type.
    pub(super) fn mime(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let Some(left) = left else { return self.mime_value(right, span) };
        let renderer = match left.is_singleton().then(|| left.at(0)) {
            Some(Value::Function(f)) => f,
            _ => {
                let Some(kind) = crate::keyed::name(left) else { return Err(span.domain_error("•mime needs a function or a MIME type on its left")) };
                before(crate::keyed::text(&kind), Function::primitive(Primitive::Keys), span)?
            }
        };
        right.clone().with_renderer(renderer).error_at(span, "invalid •mime result")
    }
    fn mime_value(&mut self, right: &Value, span: &Span) -> Result<Value, Error> {
        let fallback = crate::keyed::vector(vec!["text/plain".into()], vec![crate::keyed::text(&self.show(right))]).unwrap();
        let Some(renderer) = right.renderer().cloned() else { return Ok(fallback); };
        let echo = std::mem::replace(&mut self.execution.echo, false);
        let rendered = renderer.call_array(None, right, &mut self.at(span));
        self.execution.echo = echo;
        let bundle = crate::keyed::merge(&fallback, &rendered?).error_at(span, "MIME renderer must return a keyed vector")?;
        crate::display::bundle(&bundle).error_at(span, "MIME bundle must map MIME types to text or bytes")?;
        Ok(bundle)
    }

    /// A value with a renderer displays through it. Other values, and a renderer that fails, display as text.
    fn display_value(&mut self, value: &Value, span: &Span) -> Result<(), Error> {
        if value.renderer().is_some() {
            match self.mime_value(value, span) {
                Ok(bundle) => {
                    let data = crate::display::bundle(&bundle).expect("•mime checks its bundle");
                    self.execution.emit(Output { kind: crate::OutputKind::Display, data });
                    return Ok(());
                }
                Err(e) if e.kind.is_control() => return Err(e),
                Err(_) => (),
            }
        }
        self.execution.output(crate::OutputKind::Display, self.show(value));
        Ok(())
    }

    /// Displays a function using the trees setting, unless that view has already been shown.
    fn display_function(&self, f: &Function, shown: Option<&str>) {
        let notation = self.display.elide.notation;
        let text = if self.display.trees { f.tree(notation, &mut 1000).render() } else { f.text(notation, &mut 1000) };
        if shown != Some(&text) { self.execution.output(crate::OutputKind::Display, text); }
    }

    pub(super) fn execute(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        let source = Source::new("<execute>", Self::source_text(right, span)?);
        match left { Some(record) => self.scoped(record, span, |session| session.execute_source(source, span)), None => self.execute_source(source, span) }
    }

    /// The text that `right` holds, as `⍎` and `•load` read it: an observing read, which stops a batched run where `right` is a batch of each cell's text (`Value::observed`).
    fn source_text(right: &Value, span: &Span) -> Result<String, Error> {
        right.observed().at(span)?;
        if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "expected a unit or vector of characters")); }
        if !matches!(right.prototype(), Value::Character(_)) { return Err(span.domain_error("expected characters")); }
        right.elements().map(|e| match e { Value::Character(c) => Ok(c), _ => Err(span.domain_error("expected characters")) }).collect()
    }

    /// `•load path` runs a file in a module of its own and returns a record of the module's public names: the names its top level assigns that don't start with `_`, in alphabetical order. The file adds no names to the caller, and shows nothing except explicit output.
    pub(super) fn load(&mut self, right: &Value, span: &Span) -> Result<Bound, Error> {
        let text = Self::source_text(right, span)?;
        let path = span.path(&text);
        let code = crate::host::read(&path).and_then(|bytes| String::from_utf8(bytes).map_err(std::io::Error::other)).map_err(|e| span.io_error(&text, e))?;
        let file = crate::host::canonical(&path);
        if self.loading.contains(&file) { return Err(span.domain_error(format!("load cycle: {text} is already loading"))); }
        self.modules.push(HashMap::new());
        let module = self.modules.len();
        let caller = (self.current, self.module, std::mem::replace(&mut self.execution.echo, false));
        (self.current, self.module) = (None, module);
        self.loading.push(file);
        let result = self.execute_source(Source::file(path.to_string_lossy(), code), span);
        self.loading.pop();
        (self.current, self.module, self.execution.echo) = caller;
        result?;
        let mut names: Vec<_> = self.modules[module - 1].iter().filter(|(name, _)| !name.starts_with('_')).collect();
        names.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let (keys, values): (Vec<_>, Vec<_>) = names.into_iter().map(|(name, value)| (Arc::<str>::from(name.as_str()), value.clone())).unzip();
        let values = values.into_iter().map(|v| v.into_value(span)).collect::<Result<Vec<_>, _>>()?;
        Ok(Bound::new(Binding::Value(crate::keyed::vector(keys, values).error_at(span, "invalid module record")?)))
    }

    fn execute_source(&mut self, source: Arc<Source>, span: &Span) -> Result<Bound, Error> {
        let parsed = self.parse(source).complete()?;
        let mut result = Bound::new(Binding::NoResult);
        let mut leading = true;
        for statement in &parsed.statements {
            if let Binding::Value(a) = &result.value { if self.execution.echo && !result.shy { self.display_value(a, span)?; } }
            leading &= crate::syntax::prefs_record(&statement.nodes).is_some();
            result = self.bind_statement(statement, leading).map_err(|mut e| { e.calls.push(span.clone()); e })?;
        }
        Ok(result)
    }

    fn bind_statement(&mut self, statement: &Statement, leading: bool) -> Result<Bound, Error> {
        let prefacing = std::mem::replace(&mut self.prefacing, leading);
        let result = self.bind(&statement.nodes);
        self.prefacing = prefacing;
        result
    }

    // Resolving one structural item may execute a group, but never derives an operator or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    pub(super) fn resolve(&mut self, node: &Node) -> Result<Bound, Error> {
        Ok(Bound::new(match &node.kind {
            NodeKind::Literal(a) => Binding::Value(a.clone()),
            NodeKind::Function(p) => Binding::Function(self.primitives.entry(*p).or_insert_with(|| Function::primitive(*p)).clone()),
            NodeKind::Operator(op) => Binding::Operator(OperatorNode::Primitive(*op)),
            NodeKind::Name(name) => match self.lookup(name) {
                Some(value) => value.clone(), None if name == "⍺" && self.current.is_some() => Binding::Absent,
                None if name == "$e" => return Err(node.span.error(ErrorKind::Value, "$e is the error a handler caught, and exists only in a handler")),
                None => return Err(node.span.error(ErrorKind::Value, format!("undefined name: {name}")))
            },
            NodeKind::System(name) => {
                Binding::Function(crate::system::lookup(name).ok_or_else(|| node.span.error(ErrorKind::Value, format!("undefined name: {name}")))?)
            }
            NodeKind::Group(_) | NodeKind::Run(_) => return Binder::nested(node, self),
            NodeKind::Scope(root, body) => {
                let record = self.resolve(root)?.array(&root.span)?;
                return self.scoped(&record, &node.span, |session| session.resolve(body));
            }
            NodeKind::ArrayLiteral { cells, record: true, .. } => {
                let mut entries = Vec::new();
                for nodes in cells {
                    let Some(colon) = crate::syntax::key_colon(nodes) else { entries.extend(self.item_result(nodes)?.map(|item| (None, item))); continue; };
                    // The value and the key evaluate separately, value first. As one expression, `key:f` would be a train. A key with no value is a name, which gives both: `[a:]` is `["a":a]`.
                    let name = match &nodes[..colon] {
                        _ if colon + 1 < nodes.len() => None,
                        [Node { kind: NodeKind::Name(name), .. }] => Some(name),
                        _ => return Err(nodes[colon].span.error(ErrorKind::Syntax, "a key with no value is a name, as in [a:]")),
                    };
                    let Some(value) = self.item_result(if name.is_some() { &nodes[..colon] } else { &nodes[colon + 1..] })? else { continue };
                    let key = match name { Some(name) => crate::keyed::text(name), None => self.array_result(&nodes[..colon])? };
                    let item = Function::primitive(Primitive::Keys).call(Some(&key), &value, &mut self.at(&nodes[colon].span))?.array(&node.span)?;
                    entries.extend(crate::keyed::entries(&item).error_at(&node.span, "key:value items must give keyed vectors")?);
                }
                let (names, values) = entries.into_iter().unzip();
                Binding::Value(crate::keyed::partial_vector(names, values).error_at(&node.span, "keys must be unique")?)
            }
            NodeKind::ArrayLiteral { cells, form, .. } => {
                let mut arrays = Vec::with_capacity(cells.len());
                for nodes in cells { arrays.extend(self.item_result(nodes)?); }
                if arrays.is_empty() { return Ok(Bound::from(crate::syntax::zilde(crate::Number::float(0.0, self.numeric.width)))); }
                let result = if *form == ListForm::Cells {
                    // Each row is a major cell, so unit rows give a vector: `[1⋄2]` is `1 2`.
                    Value::assemble(&[arrays.len()], &arrays, &arrays[0])
                } else { Value::new(vec![arrays.len()], arrays) };
                Binding::Value(result.error_at(&node.span, "invalid array literal")?)
            }
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.current, module: self.module() };
                match definition.kind {
                    DefinitionKind::Function => Binding::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Binding::Operator(OperatorNode::Defined(closure)),
                }
            }
            NodeKind::Output => {
                self.levels.effect(crate::transform::Effect::Acts).at(&node.span)?;
                match self.execution.line(&node.span)? {
                    Some(line) => Binding::Value(crate::keyed::text(&line)),
                    None => Binding::Value(crate::syntax::zilde(crate::Number::float(0.0, self.numeric.width))),
                }
            }
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment symbol")),
        }))
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&Binding> { self.binding(name).map(|(_, value)| value) }

    pub(super) fn binding(&self, name: &str) -> Option<(Option<usize>, &Binding)> {
        let key = Key::of(name);
        if key == Key::ALPHA { return self.current.and_then(|i| self.frames[i].get(key).map(|value| (Some(i), value))); }
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].get(key) { return Some((scope, value)); }
            scope = self.frames[i].parent;
        }
        self.table(self.module()).get(name).map(|value| (None, value))
    }

    // Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
    #[inline(never)]
    pub(super) fn call_defined(&mut self, function: &Function, left: Option<&Value>, right: &Value) -> Result<Bound, Error> {
        let return_span = function.definition().expect("call_defined calls a dfn").span.clone();
        let caller = self.current;
        let base = self.frames.len();
        let (mut function, mut left, mut right) = (function.clone(), left.cloned(), right.clone());
        let mut tail_span = None;
        let mut unshy = false;
        let mut result = loop {
            let (closure, operand, right_operand) = match function.node() {
                FunctionNode::Defined(c) => (c, None, None), FunctionNode::Derived(c, operand, right) => (c, Some(operand), right.as_ref()), _ => unreachable!()
            };
            let mut frame = Frame::new(closure.environment, closure.module, closure.definition.names());
            frame.insert(Key::OMEGA, Binding::Value(right));
            frame.insert(Key::RECURSION, Binding::Function(function.clone()));
            if let Some(a) = left { frame.insert(Key::ALPHA, Binding::Value(a)); }
            if let Some(f) = operand {
                frame.insert(Key::LEFT_OPERAND, f.value());
                frame.insert(Key::OPERATOR_RECURSION, Binding::Operator(OperatorNode::Defined(closure.clone())));
            }
            if let Some(f) = right_operand { frame.insert(Key::RIGHT_OPERAND, f.value()); }
            if let Err(error) = self.enter(frame, &closure.definition.span) { break Err(error); }
            match self.run_definition(&closure.definition) {
                Ok(Step::Done(bound)) => break Ok(bound),
                Err(error) => break Err(error),
                Ok(Step::Tail(call)) => {
                    // Keep lexical dependencies, not the tail caller's execution frame.
                    let environment = call.function.environment().max(call.right.environment()).max(call.left.as_ref().and_then(Value::environment));
                    let keep = base.max(environment.map_or(0, |i| i + 1));
                    self.frames.truncate(keep);
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
        self.leave(base, caller, result, &return_span)
    }

    /// Pushes `frame` and makes it current. Fails at the depth limit, before pushing.
    fn enter(&mut self, frame: Frame, span: &Span) -> Result<(), Error> {
        if self.frames.len() == MAX_FRAME_DEPTH { return Err(span.error(Kind::Limit, format!("lexical frame depth exceeds {MAX_FRAME_DEPTH}"))); }
        self.current = Some(self.frames.len());
        self.frames.push(frame);
        Ok(())
    }

    /// Drops the frames from `base` on, makes `caller` current and gives back `result`. A result that refers to a dropped frame is an error.
    fn leave(&mut self, base: usize, caller: Option<usize>, result: Result<Bound, Error>, span: &Span) -> Result<Bound, Error> {
        let result = match result {
            Ok(bound) if bound.value.environment().is_some_and(|i| i >= base) => Err(span.domain_error("result would return a local closure")), result => result
        };
        self.frames.truncate(base);
        self.current = caller;
        result
    }

    /// Runs `f` in a new frame in which each key on the first axis of `record` names what `T.key` selects: an item of a vector, or a major cell of a higher-rank array. Other names resolve in the caller's scope, and assignments stay in the new frame. `T.(expr)` and `T⍎text` evaluate this way.
    fn scoped(&mut self, record: &Value, span: &Span, f: impl FnOnce(&mut Self) -> Result<Bound, Error>) -> Result<Bound, Error> {
        let entries = record.keys(0).ok_or(ErrorKind::from(Kind::Domain)).and_then(|keys| {
            let items = if record.shape().len() == 1 { record.elements().collect() } else { record.major_cells()? };
            Ok(keys.names().iter().cloned().zip(items).collect::<Vec<_>>())
        }).error_at(span, "a scope needs keys on its first axis")?;
        let (caller, base) = (self.current, self.frames.len());
        let mut frame = Frame::new(caller, self.module(), entries.len());
        for (key, item) in entries { if let Some(key) = key { frame.insert(Key::of(&key), Binding::from_element(item)); } }
        // Lookup finds `⍺` only in the current frame, so the scope holds the caller's.
        if frame.get(Key::ALPHA).is_none() { if let Some(a) = caller.and_then(|i| self.frames[i].get(Key::ALPHA)) { frame.insert(Key::ALPHA, a.clone()); } }
        self.enter(frame, span)?;
        let result = f(self);
        self.leave(base, caller, result, span)
    }

    pub(super) fn array_result(&mut self, nodes: &[Node]) -> Result<Value, Error> { self.bind(nodes)?.array(&nodes[0].span) }
    /// An item of a bracket list, or `None` for an absent `⍺`, which drops out of the list.
    fn item_result(&mut self, nodes: &[Node]) -> Result<Option<Value>, Error> {
        let bound = self.bind(nodes)?;
        if matches!(bound.value, Binding::Absent) { Ok(None) } else { bound.array(&nodes[0].span).map(Some) }
    }

    /// A body's last statement, or an error handler's expression, as the dfn's result. `slot` holds a statement's recording of its binder steps (`tree.rs`).
    fn return_expression(&mut self, mut nodes: &[Node], tail: bool, slot: Option<&std::sync::OnceLock<Option<super::Recording>>>) -> Result<Step, Error> {
        if nodes.is_empty() { return Ok(Step::Done(Bound::new(Binding::NoResult))); }
        let mut unshy = false;
        while let [Node { kind: NodeKind::Group(inner), .. }] = nodes {
            nodes = inner;
            unshy = true;
        }
        let step = if nodes.iter().any(|n| matches!(n.kind, NodeKind::Assign)) { Step::Done(self.bind(nodes)?) } else { self.statement_step(slot, nodes, tail)? };
        match step {
            Step::Done(mut bound) => { if unshy { bound.shy = false; } bound.result(&nodes[0].span).map(Step::Done) }
            Step::Tail(mut call) => { call.unshy = unshy; Ok(Step::Tail(call)) }
        }
    }

    fn run_definition(&mut self, definition: &Definition) -> Result<Step, Error> {
        let frame = self.current.unwrap();
        let mut handlers = Vec::new();
        let result = (|| {
            'bodies: for body in &definition.bodies {
                for (position, statement) in body.iter().enumerate() {
                    let nodes = &statement.nodes;
                    match statement.kind {
                        StatementKind::DefaultArgument => {
                            if nodes.len() == 2 { return Err(nodes[1].span.error(ErrorKind::Syntax, "default argument needs a value")); }
                            if self.frames[frame].get(Key::ALPHA).is_none() {
                                let value = self.bind(&nodes[2..])?.value;
                                if matches!(value, Binding::NoResult | Binding::Absent) {
                                    return Err(nodes[0].span.error(ErrorKind::Value, "default argument requires a value"));
                                }
                                self.frames[frame].insert(Key::ALPHA, value);
                            }
                        }
                        StatementKind::ErrorGuard { index: i } => {
                            let catch = Catch::new(&self.array_result(&nodes[..i])?, &nodes[i].span)?;
                            handlers.push((&nodes[i + 1..], catch));
                        }
                        StatementKind::Predicate => {
                            let span = crate::syntax::cover(nodes);
                            if !self.bound(nodes, Some(&statement.recording))?.array(&nodes[0].span)?.boolean().error_at(&span, "a predicate requires a Boolean singleton")? { continue 'bodies; }
                        }
                        StatementKind::Expression => {
                            if position + 1 == body.len() { return self.return_expression(nodes, handlers.is_empty(), Some(&statement.recording)); }
                            self.bound(nodes, Some(&statement.recording))?;
                        }
                    }
                }
                return Ok(Step::Done(Bound::new(Binding::NoResult)));
            }
            Err(definition.span.domain_error("every predicate failed, so no body applies"))
        })();
        let mut result = result;
        while let Err(error) = &result {
            let Some((handler, catch)) = handlers.pop() else { break };
            if !catch.catches(&error.kind) { continue; }
            if let Err(stop) = self.levels.effect(crate::transform::Effect::Catches) { let span = error.span.clone(); result = Err(stop).at(&span); break; }
            if matches!(error.kind, ErrorKind::Interrupt) { self.execution.clear_interrupt(); }
            self.frames[frame].insert(Key::Named("$e"), Binding::Value(caught(error)));
            result = self.return_expression(handler, false, None);
        }
        definition.bound(self.frames[frame].names.len());
        result
    }
}

/// The errors a guard catches: the kinds it names, or with `∞` every kind but an interrupt, a timeout or an unsupported feature. Unsupported features must not turn into plausible results. No guard catches a batched run's stop, even one that names its kind, because only the level that owns the run catches it. Inside a batched run, a guard that would catch an error stops the run instead (`transform::Levels::effect` with `Effect::Catches`). The run over cells then catches the error in the cells that raise it.
pub(super) enum Catch { All, Kinds(Vec<ErrorKind>) }
impl Catch {
    fn new(value: &Value, span: &Span) -> Result<Self, Error> {
        if value.observed().at(span)?.infinity() == Some(false) { return Ok(Self::All); }
        let invalid = || span.domain_error("a guard names error kinds, as in \"DOMAIN\"::, or catches every kind with ∞::");
        let (_, names) = crate::keyed::text_items(value).ok_or_else(invalid)?;
        Ok(Self::Kinds(names.iter().map(|n| ErrorKind::named(n)).collect::<Option<_>>().ok_or_else(invalid)?))
    }
    pub(super) fn catches(&self, kind: &ErrorKind) -> bool {
        match self {
            Self::All => !kind.is_control() && !matches!(kind, ErrorKind::Unsupported), Self::Kinds(kinds) => !matches!(kind, ErrorKind::Stop { .. }) && kinds.iter().any(|k| k.name() == kind.name())
        }
    }
}

/// `$e` in a handler: the caught error's kind and message, and the source, line and column where it happened.
fn caught(error: &Error) -> Value {
    let (line, column) = error.span.position();
    let number = |n: usize| Value::integer(n as i64);
    let text = crate::keyed::text;
    let values = vec![text(error.kind.name()), text(&error.message), text(&error.span.source.name), number(line), number(column)];
    crate::keyed::record(["kind", "message", "source", "line", "column"].map(Into::into).to_vec(), values).expect("an error fits in a record")
}
