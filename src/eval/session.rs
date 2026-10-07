//! The session's evaluation API, names, display, file loading and name lookup, and calls of dfns with their error guards.

use super::*;

impl Session {
    pub fn new() -> Self { Self::default() }
    /// This session with `args` as the program's command-line arguments, which `•host "args"` gives.
    pub fn with_args(self, args: Vec<String>) -> Self { Self { args, ..self } }
    /// This session at `span`, for a call that takes a `Context`.
    pub(crate) fn at<'a>(&'a mut self, span: &'a Span) -> Context<'a> { Context { span, session: self } }
    /// A session for a person at a terminal, with boxed display elided to the terminal's width.
    pub fn interactive() -> Self { let display = crate::display::Settings::interactive(); Self { display, display_defaults: display, ..Self::default() } }
    /// `value` as this session's implicit display shows it, as text.
    pub fn show(&self, value: &Value) -> String { self.display.array(value, false) }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let mut names = HashSet::new();
        let mut scope = self.current;
        while let Some(i) = scope {
            names.extend(self.frames[i].names.keys().map(String::as_str).filter(|name| !implicit_name(name)));
            scope = self.frames[i].parent;
        }
        names.extend(self.table(self.module()).keys().map(String::as_str));
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort_unstable();
        names.into_iter()
    }
    pub fn name_class(&self, name: &str) -> i64 {
        match crate::inspection::item(name) {
            Some(NodeKind::Name(_)) => self.lookup(name).map_or(0, Binding::class),
            Some(NodeKind::System(_)) => crate::system::lookup(name).map_or(0, |_| FUNCTION_CLASS),
            _ => -1,
        }
    }
    pub fn name_list(&self, classes: &[i64], prefix: &str) -> Vec<String> {
        self.names().filter(|name| name.starts_with(prefix) && classes.contains(&self.name_class(name))).map(str::to_owned).collect()
    }
    pub fn inspect(&self, name: &str) -> Option<crate::Inspection> {
        let info = match crate::inspection::item(name) {
            Some(NodeKind::Name(_)) => self.lookup(name).and_then(|binding| binding.inspection(self)),
            Some(NodeKind::System(_)) => {
                let mut info = crate::system::lookup(name)?.inspect(self);
                if let Some(help) = crate::inspection::documentation(name) { info.help = help.to_owned(); }
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
            FUNCTION_CLASS | OPERATOR_CLASS => Ok(self.inspect(name).unwrap().source),
            SUBJECT_CLASS => Err(ErrorKind::Domain),
            _ => Err(ErrorKind::Value),
        }
    }
    /// The module whose names code sees after its frames: the module that defined the running function, or the module whose top
    /// level is running. Module 0 is the session's own names.
    fn module(&self) -> usize { self.current.map_or(self.module, |i| self.frames[i].module) }
    fn table(&self, module: usize) -> &HashMap<String, Binding> { if module == 0 { &self.names } else { &self.modules[module - 1] } }
    /// The names of the frame that owns a binding, or the names of the current module.
    pub(super) fn names_mut(&mut self, owner: Option<usize>) -> &mut HashMap<String, Binding> {
        match (owner, self.module()) { (Some(i), _) => &mut self.frames[i].names, (None, 0) => &mut self.names, (None, m) => &mut self.modules[m - 1] }
    }
    pub fn erase(&mut self, name: &str) -> bool {
        if implicit_name(name) || !matches!(crate::inspection::item(name), Some(NodeKind::Name(_))) { return false; }
        if let Some((owner, _)) = self.binding(name) { self.names_mut(owner).remove(name); }
        true
    }
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        let mut names = self.name_list(&[2, 3, 4], prefix);
        names.extend(crate::system::names().filter(|name| name.starts_with(&prefix.to_lowercase())).map(str::to_owned));
        names.sort_unstable();
        names
    }
    fn map_names(&mut self, right: &Value, span: &Span, prototype: Value, f: impl Fn(&mut Self, &str) -> Result<Value, ErrorKind>) -> Result<Value, Error> {
        let apply = |session: &mut Self, value: &Value| {
            let name = crate::keyed::name(value).ok_or_else(|| span.domain_error("expected a name string"))?;
            f(session, &name).map_err(|kind| span.error(kind, format!("cannot inspect name: {name}")))
        };
        if crate::keyed::name(right).is_some() { return apply(self, right); }
        let values = right.elements().map(|a| apply(self, &a)).collect::<Result<Vec<_>, _>>()?;
        right.layout().collect(values, prototype).error_at(span, "invalid name results")
    }
    pub(crate) fn system_nc(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.map_names(right, span, crate::primitive::integer(0), |s, name| Ok(crate::primitive::integer(s.name_class(name))))
    }
    pub(crate) fn system_ex(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.map_names(right, span, crate::primitive::integer(0), |s, name| Ok(crate::primitive::integer(s.erase(name) as i64)))
    }
    pub(crate) fn system_src(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.map_names(right, span, crate::keyed::text(""), |s, name| s.name_source(name).map(|text| crate::keyed::text(&text)))
    }
    /// `•literal text` reads a value from BPL source, as `•literal⁻¹` writes it, without running code. The text may hold literals and the
    /// functions that `•literal⁻¹` writes: `⍴`, `⊂`, `,`, `:` and `•ucs`.
    pub(crate) fn system_literal(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let parsed = crate::parse(Source::new("<•literal>", Self::source_text(right, span)?)).complete()?;
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
    /// `•time t` gives the seconds since `t`, counting from the Unix epoch. `F •time x` gives each function's fastest time per call
    /// on `x`, in the layout of `F`.
    pub(super) fn system_time(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let Some(functions) = left else {
            let now = Value::Number(crate::date::now().into());
            return Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(Some(&now), right, &self.at(span));
        };
        let times = functions
            .elements()
            .map(|f| match f { Value::Function(f) => self.fastest(&f, right, span), _ => Err(span.domain_error("•time needs functions on its left")) })
            .collect::<Result<Vec<_>, _>>()?;
        if functions.is_atom() { return Value::number(times[0]).error_at(span, "invalid •time result"); }
        Value::floats(functions.shape().to_vec(), times).and_then(|t| t.with_layout(functions.layout().clone())).error_at(span, "invalid •time result")
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
            Some(value) => crate::keyed::name(value).ok_or_else(|| span.domain_error("•nl prefix must be a string"))?,
            None => "".into(),
        };
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•nl needs a class or class vector")); }
        let classes = right.as_items().integers().error_at(span, "invalid name class")?;
        if classes.iter().any(|n| !(SUBJECT_CLASS..=OPERATOR_CLASS).contains(n)) { return Err(span.domain_error("name classes are 2, 3 and 4")); }
        let values = self.name_list(&classes, &prefix).iter().map(|n| crate::keyed::text(n)).collect::<Vec<_>>();
        crate::keyed::texts(&[values.len()], values).error_at(span, "invalid name list")
    }
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), ErrorKind> { self.set_value(name, Binding::from_element(value)) }
    pub fn set_function(&mut self, name: &str, value: Function) -> Result<(), ErrorKind> { self.set_value(name, Binding::Function(value)) }
    fn set_value(&mut self, name: &str, value: Binding) -> Result<(), ErrorKind> {
        if !matches!(crate::inspection::item(name), Some(NodeKind::Name(_))) || implicit_name(name) { return Err(ErrorKind::Syntax); }
        self.names.insert(name.to_owned(), value);
        Ok(())
    }
    pub fn eval(&mut self, code: &str) -> Evaluation { self.eval_with(code, crate::EvalOptions::default()) }
    pub fn eval_with(&mut self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.evaluate_source(Source::new("<input>", code)))
    }
    /// One evaluation under `options`: `evaluate`'s result, with the output it produced.
    fn evaluation(&mut self, options: crate::EvalOptions, evaluate: impl FnOnce(&mut Self) -> Evaluation) -> Evaluation {
        self.execution.begin(options);
        let result = evaluate(self);
        self.execution.flush();
        Evaluation { output: self.execution.take_output(), ..result }
    }
    pub fn eval_timeout(&mut self, code: &str, timeout: std::time::Duration) -> Evaluation {
        self.eval_with(code, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() })
    }
    /// Resolve a function expression in this session and call it with one (right) or two (left, right) arrays.
    /// No function or lexical-frame handle escapes the call, and no temporary names are bound.
    pub fn call(&mut self, function: &str, args: &[Value]) -> Evaluation { self.call_with(function, args, crate::EvalOptions::default()) }
    pub fn call_with(&mut self, function: &str, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        match Function::late_bound(function) { Ok(f) => self.call_function_with(&f, args, options), Err(error) => Evaluation::failed(error) }
    }
    pub fn call_function_with(&mut self, function: &Function, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        self.evaluation(options, |s| s.call_function(function, args))
    }
    fn call_function(&mut self, function: &Function, args: &[Value]) -> Evaluation {
        let span = Span::whole(Source::new("<call>", function.bpl()));
        let mut result = Evaluation::default();
        let called = (|| {
            let (left, right) = match args {
                [right] => (None, right),
                [left, right] => (Some(left), right),
                _ => return Err(span.error(ErrorKind::Length, "call requires one or two arguments")),
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
    /// A line that starts with `]` is a command, unless it closes brackets that the code before it left open. Each part
    /// keeps its line numbers, so errors point at the right line.
    fn evaluate_source(&mut self, source: Arc<Source>) -> Evaluation {
        let (mut parts, mut start, mut offset) = (Vec::new(), 0, 0);
        for line in source.text.split_inclusive('\n') {
            let pending = &source.text[start..offset];
            if line.trim_start().starts_with(']')
                && (pending.trim().is_empty() || !matches!(crate::parse(Source::new("", pending)), ParseStatus::Incomplete(_)))
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
            if next.error.is_some() {
                result.error = next.error;
                break;
            }
        }
        result
    }
    fn evaluate_code(&mut self, source: Arc<Source>) -> Evaluation {
        match crate::parse(source).complete() { Ok(parsed) => self.eval_display(&parsed, false, false), Err(e) => Evaluation::failed(e) }
    }
    pub fn eval_parsed(&mut self, parsed: &Parsed, options: crate::EvalOptions) -> Evaluation { self.evaluation(options, |s| s.eval_display(parsed, false, false)) }
    fn eval_display(&mut self, parsed: &Parsed, diagram: bool, dissect: bool) -> Evaluation {
        let mut result = Evaluation::default();
        for (i, statement) in parsed.statements.iter().enumerate() {
            (result.function, result.operator) = (None, None);
            let nodes = &statement.nodes;
            let previous = self.capture;
            let dissecting = self.depth == 0 && (dissect || self.display.dissect && self.execution.echo);
            self.capture = dissecting;
            let bound = self.bind(nodes);
            self.capture = previous;
            match bound {
                Ok(bound) => {
                    let dissected = if dissecting && (dissect || !bound.shy && !matches!(bound.value, Binding::NoResult | Binding::Absent)) {
                        bound.expression.or_else(|| Expression::binding(&bound.value)).map(|e| e.display(self.display.trees))
                    } else { None };
                    if let Some(text) = &dissected { self.execution.output(crate::OutputKind::Display, text.clone()); }
                    result.value = match bound.value {
                        Binding::NoResult | Binding::Absent => None,
                        Binding::Value(a) => {
                            if diagram && i + 1 == parsed.statements.len() {
                                self.execution.output(crate::OutputKind::Display, self.display.diagram(&a));
                            }
                            else if self.execution.echo && !bound.shy {
                                if let Err(e) = self.display_value(&a, &nodes[0].span) {
                                    result.error = Some(e);
                                    return result;
                                }
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
                            if self.execution.echo { self.execution.output(crate::OutputKind::Display, op.to_string()); }
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
            let Some((name, detail)) = crate::inspection::help_command(code) else { return failed(ErrorKind::Syntax, "usage: ]help name [-source]".into()) };
            let Some(info) = self.inspect(name) else { return failed(ErrorKind::Value, format!("name not found: {name}")) };
            let data = [("text/plain", info.text(detail)), ("text/markdown", info.markdown(detail))]
                .map(|(kind, text)| (kind.to_string(), crate::MimeData::Text(text)))
                .into();
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
            return match crate::parse(Source::new("<display>", args)).complete() {
                Ok(parsed) => self.eval_display(&parsed, true, false),
                Err(e) => Evaluation::failed(e),
            };
        }
        if command.eq_ignore_ascii_case("]dissect") {
            if args.trim().is_empty() { return failed(ErrorKind::Syntax, "usage: ]dissect expression".into()); }
            return match crate::parse(Source::new("<dissect>", args)).complete() {
                Ok(parsed) => self.eval_display(&parsed, false, true),
                Err(e) => Evaluation::failed(e),
            };
        }
        failed(ErrorKind::Syntax, "unknown user command".into())
    }
    pub(super) fn bind(&mut self, nodes: &[Node]) -> Result<Bound, Error> {
        self.execution.check(&nodes[0].span)?;
        if self.depth == MAX_CALL_DEPTH { return Err(depth_error(&nodes[0].span)); }
        self.depth += 1;
        let result = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || self.bind_expression(nodes));
        self.depth -= 1;
        result
    }

    /// `•mime Y` is the MIME bundle that displays `Y`. `X •mime Y` is `Y` displayed through `X`, which holds a function or names a MIME
    /// type. A MIME type shows `Y` itself as that type.
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
                Err(e) if matches!(e.kind, ErrorKind::Interrupt | ErrorKind::Timeout) => return Err(e),
                Err(_) => (),
            }
        }
        self.execution.output(crate::OutputKind::Display, self.show(value));
        Ok(())
    }

    /// Displays a function using the trees setting, unless that view has already been shown.
    fn display_function(&self, f: &Function, shown: Option<&str>) {
        let text = if self.display.trees { f.tree(&mut 1000).render() } else { f.text(&mut 1000) };
        if shown != Some(&text) { self.execution.output(crate::OutputKind::Display, text); }
    }

    pub(super) fn execute(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        let source = Source::new("<execute>", Self::source_text(right, span)?);
        match left { Some(record) => self.scoped(record, span, |session| session.execute_source(source, span)), None => self.execute_source(source, span) }
    }

    fn source_text(right: &Value, span: &Span) -> Result<String, Error> {
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "expected a unit or vector of characters")); }
        if !matches!(right.prototype(), Value::Character(_)) { return Err(span.domain_error("expected characters")); }
        right
            .elements()
            .map(|e| match e { Value::Character(c) => Ok(c), _ => Err(span.domain_error("expected characters")) })
            .collect()
    }

    /// `•load path` runs a file in a module of its own and returns a record of the module's public names: the names its top level
    /// assigns that don't start with `_`, in alphabetical order. The file adds no names to the caller, and shows nothing except
    /// explicit output.
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
        let parsed = crate::parse(source).complete()?;
        let mut result = Bound::new(Binding::NoResult);
        for statement in &parsed.statements {
            if let Binding::Value(a) = &result.value { if self.execution.echo && !result.shy { self.display_value(a, span)?; } }
            result = self.bind(&statement.nodes).map_err(|mut e| { e.calls.push(span.clone()); e })?;
        }
        Ok(result)
    }

    fn bind_expression(&mut self, nodes: &[Node]) -> Result<Bound, Error> {
        let Step::Done(result) = Binder::evaluate(nodes, self, false)? else { unreachable!() };
        Ok(result)
    }

    // Resolving one structural item may execute a group, but never derives an operator
    // or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    pub(super) fn resolve(&mut self, node: &Node) -> Result<Bound, Error> {
        Ok(Bound::new(match &node.kind {
            NodeKind::Literal(a) => Binding::Value(a.clone()),
            NodeKind::Function(p) => Binding::Function(Function::primitive(*p)),
            NodeKind::Operator(op) => Binding::Operator(OperatorNode::Primitive(*op)),
            NodeKind::Name(name) => match self.lookup(name) {
                Some(value) => value.clone(),
                None if name == "⍺" && self.current.is_some() => Binding::Absent,
                None if name == "$e" => return Err(node.span.error(ErrorKind::Value, "$e is the error a handler caught, and exists only in a handler")),
                None => return Err(node.span.error(ErrorKind::Value, format!("undefined name: {name}"))),
            },
            NodeKind::System(name) => {
                Binding::Function(crate::system::lookup(name).ok_or_else(|| node.span.error(ErrorKind::Unsupported, format!("{name} is not supported yet")))?)
            }
            NodeKind::Group(nodes) => return self.bind(nodes),
            NodeKind::Scope(root, body) => {
                let record = self.resolve(root)?.array(&root.span)?;
                return self.scoped(&record, &node.span, |session| session.resolve(body));
            }
            // A dot path such as `m.op` names an operator as a name does, so its run can reduce to one.
            NodeKind::Run(nodes) => {
                let path = matches!(&self.members(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]);
                let bound = self.bind(nodes)?;
                match bound.value {
                    Binding::Operator(_) if !path => {
                        return Err(node.span.error(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph"))
                    }
                    _ => return Ok(bound),
                }
            }
            NodeKind::Pipeline(stages) => {
                let mut result = self.bind(&stages[0])?;
                for stage in &stages[1..] {
                    let expression = result.expression.take();
                    let right = result.array(&node.span)?;
                    let function = Function::from_value(self.bind(stage)?.value, &stage[0].span)?;
                    result = function.call(None, &right, &mut self.at(&stage[0].span))?;
                    if self.capture { result.expression = Some(Expression::call(&function, None, expression.unwrap_or_else(|| Expression::array(&right)))); }
                }
                return Ok(result);
            }
            NodeKind::ArrayLiteral { cells, record: true, .. } => {
                let mut entries = Vec::new();
                for nodes in cells {
                    let Some(colon) = crate::syntax::key_colon(nodes) else {
                        entries.extend(self.item_result(nodes)?.map(|item| (None, item)));
                        continue;
                    };
                    // The value and the key evaluate separately, value first. As one expression, `key:f` would be a train.
                    let Some(value) = self.item_result(&nodes[colon + 1..])? else { continue };
                    let key = self.array_result(&nodes[..colon])?;
                    let item = Function::primitive(Primitive::Keys).call(Some(&key), &value, &mut self.at(&nodes[colon].span))?.array(&node.span)?;
                    entries.extend(crate::keyed::entries(&item).error_at(&node.span, "key:value items must give keyed vectors")?);
                }
                let (names, values) = entries.into_iter().unzip();
                Binding::Value(crate::keyed::partial_vector(names, values).error_at(&node.span, "keys must be unique")?)
            }
            NodeKind::ArrayLiteral { cells, form, .. } => {
                let mut arrays = Vec::with_capacity(cells.len());
                for nodes in cells { arrays.extend(self.item_result(nodes)?); }
                if arrays.is_empty() { return Ok(Bound::from(crate::syntax::zilde(false))); }
                let result = if *form == ListForm::Cells {
                    // Each row is a major cell, so unit rows give a vector: `[1 ⋄ 2]` is `1 2`.
                    Value::assemble_written(&[arrays.len()], &arrays, &arrays[0])
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
            NodeKind::Output => match self.execution.line(&node.span)? {
                Some(line) => Binding::Value(crate::keyed::text(&line)),
                None => Binding::Value(crate::syntax::zilde(false)),
            },
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment symbol")),
        }))
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&Binding> { self.binding(name).map(|(_, value)| value) }

    pub(super) fn binding(&self, name: &str) -> Option<(Option<usize>, &Binding)> {
        if name == "⍺" { return self.current.and_then(|i| self.frames[i].names.get(name).map(|value| (Some(i), value))); }
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].names.get(name) { return Some((scope, value)); }
            scope = self.frames[i].parent;
        }
        self.table(self.module()).get(name).map(|value| (None, value))
    }

    // Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
    #[inline(never)]
    pub(super) fn call_defined(&mut self, function: &Function, left: Option<&Value>, right: &Value) -> Result<Bound, Error> {
        let return_span = match function.node() { FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => c.definition.span.clone(), _ => unreachable!() };
        let caller = self.current;
        let base = self.frames.len();
        let (mut function, mut left, mut right) = (function.clone(), left.cloned(), right.clone());
        let mut tail_span = None;
        let mut unshy = false;
        let mut result = loop {
            let (closure, operand, right_operand) = match function.node() {
                FunctionNode::Defined(c) => (c, None, None),
                FunctionNode::Derived(c, operand, right) => (c, Some(operand), right.as_ref()),
                _ => unreachable!(),
            };
            let mut names = HashMap::from_iter([("⍵".into(), Binding::Value(right)), ("∇".into(), Binding::Function(function.clone()))]);
            if let Some(a) = left { names.insert("⍺".into(), Binding::Value(a)); }
            if let Some(f) = operand {
                names.insert("⍶".into(), f.value());
                names.insert("⍢".into(), Binding::Operator(OperatorNode::Defined(closure.clone())));
            }
            if let Some(f) = right_operand { names.insert("⍹".into(), f.value()); }
            if let Err(error) = self.enter(Frame { names, parent: closure.environment, module: closure.module }, &closure.definition.span) { break Err(error); }
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
        if self.frames.len() == MAX_FRAME_DEPTH { return Err(span.error(ErrorKind::Limit, format!("lexical frame depth exceeds {MAX_FRAME_DEPTH}"))); }
        self.current = Some(self.frames.len());
        self.frames.push(frame);
        Ok(())
    }

    /// Drops the frames from `base` on, makes `caller` current and gives back `result`. A result that refers to a dropped frame is
    /// an error.
    fn leave(&mut self, base: usize, caller: Option<usize>, result: Result<Bound, Error>, span: &Span) -> Result<Bound, Error> {
        let result = match result {
            Ok(bound) if bound.value.environment().is_some_and(|i| i >= base) => Err(span.domain_error("result would return a local closure")),
            result => result,
        };
        self.frames.truncate(base);
        self.current = caller;
        result
    }

    /// Runs `f` in a new frame in which each key of the keyed vector `record` names its item. Other names resolve in the caller's
    /// scope, and assignments stay in the new frame. `T.(expr)` and `T⍎text` evaluate this way.
    fn scoped(&mut self, record: &Value, span: &Span, f: impl FnOnce(&mut Self) -> Result<Bound, Error>) -> Result<Bound, Error> {
        let entries = record.keys(0).ok_or(ErrorKind::Domain).and_then(|_| crate::keyed::entries(record)).error_at(span, "a scope must be a keyed vector")?;
        let mut names: HashMap<String, Binding> = entries.into_iter().filter_map(|(key, item)| Some((key?.to_string(), Binding::from_element(item)))).collect();
        // Lookup finds `⍺` only in the current frame, so the scope holds the caller's.
        if let Some(a) = self.current.and_then(|i| self.frames[i].names.get("⍺")) { names.entry("⍺".into()).or_insert_with(|| a.clone()); }
        let (caller, base) = (self.current, self.frames.len());
        self.enter(Frame { names, parent: caller, module: self.module() }, span)?;
        let result = f(self);
        self.leave(base, caller, result, span)
    }

    pub(super) fn array_result(&mut self, nodes: &[Node]) -> Result<Value, Error> { self.bind(nodes)?.array(&nodes[0].span) }
    /// An item of a bracket list, or `None` for an absent `⍺`, which drops out of the list.
    fn item_result(&mut self, nodes: &[Node]) -> Result<Option<Value>, Error> {
        let bound = self.bind(nodes)?;
        if matches!(bound.value, Binding::Absent) { Ok(None) } else { bound.array(&nodes[0].span).map(Some) }
    }

    fn return_expression(&mut self, mut nodes: &[Node], tail: bool) -> Result<Step, Error> {
        if nodes.is_empty() { return Ok(Step::Done(Bound::new(Binding::NoResult))); }
        let mut unshy = false;
        while let [Node { kind: NodeKind::Group(inner), .. }] = nodes {
            nodes = inner;
            unshy = true;
        }
        let step = if nodes.iter().any(|n| matches!(n.kind, NodeKind::Assign)) { Step::Done(self.bind(nodes)?) } else { Binder::evaluate(nodes, self, tail)? };
        match step {
            Step::Done(mut bound) => {
                if unshy { bound.shy = false; }
                bound.result(&nodes[0].span).map(Step::Done)
            }
            Step::Tail(mut call) => {
                call.unshy = unshy;
                Ok(Step::Tail(call))
            }
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
                            if !self.frames[frame].names.contains_key("⍺") {
                                let value = self.bind(&nodes[2..])?.value;
                                if matches!(value, Binding::NoResult | Binding::Absent) {
                                    return Err(nodes[0].span.error(ErrorKind::Value, "default argument requires a value"));
                                }
                                self.frames[frame].names.insert("⍺".into(), value);
                            }
                        }
                        StatementKind::ErrorGuard { index: i } => {
                            let catch = Catch::new(&self.array_result(&nodes[..i])?, &nodes[i].span)?;
                            handlers.push((&nodes[i + 1..], catch));
                        }
                        StatementKind::Predicate => {
                            let span = crate::syntax::cover(nodes);
                            if !self.array_result(nodes)?.boolean().error_at(&span, "a predicate requires a Boolean singleton")? { continue 'bodies; }
                        }
                        StatementKind::Expression => {
                            if position + 1 == body.len() { return self.return_expression(nodes, handlers.is_empty()); }
                            self.bind(nodes)?;
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
            if error.kind == ErrorKind::Interrupt { self.execution.clear_interrupt(); }
            self.frames[frame].names.insert("$e".into(), Binding::Value(caught(error)));
            result = self.return_expression(handler, false);
        }
        result
    }
}

/// The errors a guard catches: the kinds it names, or with `∞` every kind but an interrupt, a timeout or an unsupported feature.
/// Unsupported features must not turn into plausible results.
enum Catch { All, Kinds(Vec<ErrorKind>) }
impl Catch {
    fn new(value: &Value, span: &Span) -> Result<Self, Error> {
        if value.as_number().is_some_and(|n| n.is_infinite() && n.as_float().is_some_and(|x| x > 0.)) { return Ok(Self::All); }
        let invalid = || span.domain_error("a guard names error kinds, as in \"DOMAIN\"::, or catches every kind with ∞::");
        let (_, names) = crate::keyed::text_items(value).ok_or_else(invalid)?;
        Ok(Self::Kinds(names.iter().map(|n| ErrorKind::named(n)).collect::<Option<_>>().ok_or_else(invalid)?))
    }
    fn catches(&self, kind: &ErrorKind) -> bool {
        match self {
            Self::All => !matches!(kind, ErrorKind::Interrupt | ErrorKind::Timeout | ErrorKind::Unsupported),
            Self::Kinds(kinds) => kinds.contains(kind),
        }
    }
}

/// `$e` in a handler: the caught error's kind and message, and the source, line and column where it happened.
fn caught(error: &Error) -> Value {
    let (line, column) = error.span.position();
    let number = |n: usize| Value::Number(crate::Number::from_integer(n as i64));
    let text = crate::keyed::text;
    let values = vec![text(error.kind.name()), text(&error.message), text(&error.span.source.name), number(line), number(column)];
    crate::keyed::record(["kind", "message", "source", "line", "column"].map(Into::into).to_vec(), values).expect("an error fits in a record")
}
