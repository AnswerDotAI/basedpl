use basedpl::{Buffer, EvalOptions, Evaluation, Function, InterruptHandle, Number, Operand, Session, Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use pyo3::{
    buffer::PyBuffer,
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    sync::MutexExt,
    types::{PyBool, PyBoolMethods, PyByteArray, PyComplex, PyComplexMethods, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
    IntoPyObjectExt,
};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

fn import_array(raw: &Bound<'_, PyDict>) -> PyResult<Value> {
    let element = |o: Bound<'_, PyAny>| -> PyResult<Value> {
        if let Ok(a) = o.extract::<PyRef<'_, PyArray>>() { return Ok(a.inner.clone()); }
        if let Ok(f) = o.extract::<PyRef<'_, PyFunction>>() { return Ok(Value::Function(f.inner.clone())); }
        if let Ok(op) = o.extract::<PyRef<'_, PyOperator>>() { return Ok(Value::Operator(op.inner.clone())); }
        if let Ok(d) = o.cast::<PyDict>() { return import_array(d); }
        if let Ok(s) = o.cast::<PyString>() {
            return basedpl::protocol::character(s.to_str()?).map(Value::Character).ok_or_else(|| PyValueError::new_err("expected one character"));
        }
        let number = if let Ok(b) = o.cast::<PyBool>() { Ok(Number::from_bool(b.is_true())) } else if o.is_instance_of::<PyFloat>() { Ok(o.extract::<f64>()?.into()) } else if let Ok(z) = o.cast::<PyComplex>() { Ok(num_complex::Complex64::new(z.real(), z.imag()).into()) } else if o.is_instance_of::<PyInt>() { Ok(o.extract::<BigInt>()?.into()) } else if o.is_instance_of::<PyTuple>() { let (n, d) = o.extract::<(BigInt, BigInt)>()?; Number::try_from(BigRational::new_raw(n, d)) } else { return Err(PyTypeError::new_err("unsupported BPL element")); };
        number.map(Value::Number).map_err(|k| PyValueError::new_err(k.to_string()))
    };
    let field = |key| raw.get_item(key)?.ok_or_else(|| PyValueError::new_err("missing array field"));
    if let Some(atom) = raw.get_item("atom")? { return element(atom); }
    let shape = field("shape")?.extract::<Vec<usize>>()?;
    let data = field("data")?.try_iter()?.map(|o| element(o?)).collect::<PyResult<Vec<_>>>()?;
    let prototype = element(field("prototype")?)?;
    // Python data takes the import rules. A dict keeps each value's kind, as a JSON object does. Other data becomes floats only when
    // every number converts exactly.
    let record = raw.get_item("axis_keys")?.is_some();
    let mut result = if record { Value::mixed(shape, data, prototype) } else { Value::imported(shape, data, prototype) }
        .map_err(|k| PyValueError::new_err(k.to_string()))?;
    if let Some(names) = raw.get_item("axis_names")? { result = basedpl::keyed::with_names(result, names.extract()?).map_err(PyValueError::new_err)?; }
    let Some(keys) = raw.get_item("axis_keys")? else { return Ok(result); };
    basedpl::keyed::with_key_lists(result, keys.extract()?).map_err(PyValueError::new_err)
}

/// The NumPy dtype and bytes of storage that NumPy holds as it is.
fn numpy(a: &Value) -> Option<(&'static str, Cow<'_, [u8]>)> {
    /// The bytes of `v`, borrowed when `v` is.
    fn bytes<T: bytemuck::NoUninit>(v: Cow<'_, [T]>) -> Cow<'_, [u8]> {
        match v { Cow::Borrowed(v) => Cow::Borrowed(bytemuck::cast_slice(v)), Cow::Owned(v) => Cow::Owned(bytemuck::cast_slice(&v).to_vec()) }
    }
    Some(match a.buffer()? {
        Buffer::Booleans(v) => ("?", bytes(v)),
        Buffer::U8(v) => ("u1", bytes(v)),
        Buffer::I16(v) => ("i2", bytes(v)),
        Buffer::I32(v) => ("i4", bytes(v)),
        Buffer::I64(v) => ("i8", bytes(v)),
        Buffer::Floats(v) => ("f8", bytes(v)),
    })
}

fn array(py: Python<'_>, a: &Value) -> PyResult<Py<PyDict>> {
    fn element(py: Python<'_>, e: &Value) -> PyResult<Py<PyAny>> {
        Ok(match e {
            Value::Number(n) => {
                if let Some(b) = n.as_bool() { PyBool::new(py, b).to_owned().into_any().unbind() } else if let Some(n) = n.as_integer() { n.into_pyobject(py)?.into_any().unbind() } else if let Some(n) = n.as_exact() {
                    if n.is_integer() { n.numer().into_pyobject(py)?.into_any().unbind() } else { (n.numer(), n.denom()).into_pyobject(py)?.into_any().unbind() }
                } else if let Some(n) = n.as_complex() { PyComplex::from_doubles(py, n.re, n.im).into_any().unbind() } else { PyFloat::new(py, n.as_float().unwrap()).into_any().unbind() }
            }
            Value::Character(c) => PyString::new(py, &c.to_string()).into_any().unbind(),
            a @ Value::Array(_) => array(py, a)?.into_any(),
            Value::Function(f) => Py::new(py, PyFunction { inner: f.clone() })?.into_any(),
            Value::Operator(op) => Py::new(py, PyOperator { inner: op.clone() })?.into_any(),
        })
    }
    let result = PyDict::new(py);
    if a.is_atom() {
        result.set_item("atom", element(py, a)?)?;
        return Ok(result.unbind());
    }
    let data = PyList::empty(py);
    for e in a.elements() { data.append(element(py, &e)?)?; }
    result.set_item("shape", a.shape())?;
    result.set_item("data", data)?;
    result.set_item("prototype", element(py, &a.prototype())?)?;
    if let Some((dtype, _)) = numpy(a) { result.set_item("dtype", dtype)?; }
    if !a.axis_names().is_empty() { result.set_item("axis_names", a.axis_names().iter().map(|n| n.as_deref()).collect::<Vec<_>>())?; }
    if a.has_keys() { result.set_item("axis_keys", basedpl::keyed::key_lists(a))?; }
    Ok(result.unbind())
}

#[pyclass(frozen, name = "_Array")]
struct PyArray { inner: Value }

#[pymethods]
impl PyArray {
    #[new]
    fn new(raw: &Bound<'_, PyDict>) -> PyResult<Self> { Ok(Self { inner: import_array(raw)? }) }
    #[staticmethod]
    fn numeric(shape: Vec<usize>, boolean: bool, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        fn ints<T: pyo3::buffer::Element>(data: &Bound<'_, PyAny>, buffer: fn(Cow<'static, [T]>) -> Buffer<'static>) -> Option<PyResult<Buffer<'static>>> {
            PyBuffer::<T>::get(data).ok().map(|b| Ok(buffer(b.to_vec(data.py())?.into())))
        }
        // A NumPy Boolean array arrives as its bytes, one for each item.
        let buffer = if boolean { Buffer::Booleans(PyBuffer::<u8>::get(data)?.to_vec(data.py())?.into_iter().map(|x| x != 0).collect()) } else {
            let ints = ints(data, Buffer::U8).or_else(|| ints(data, Buffer::I16)).or_else(|| ints(data, Buffer::I32)).or_else(|| ints(data, Buffer::I64));
            match ints { Some(ints) => ints?, None => Buffer::Floats(PyBuffer::<f64>::get(data)?.to_vec(data.py())?.into()) }
        };
        let inner = Value::from_buffer(shape, buffer);
        inner.map(|inner| Self { inner }).map_err(|k| PyValueError::new_err(k.to_string()))
    }
    #[getter]
    fn shape(&self) -> Vec<usize> { self.inner.shape().to_vec() }
    #[getter]
    fn axis_names(&self) -> Vec<Option<String>> { (0..self.inner.shape().len()).map(|a| self.inner.axis_name(a).map(|n| n.to_string())).collect() }
    fn with_axis_names(&self, names: Vec<Option<String>>) -> PyResult<Self> {
        Ok(Self { inner: basedpl::keyed::with_names(self.inner.clone(), names).map_err(PyValueError::new_err)? })
    }
    #[getter]
    fn axis_keys(&self) -> Vec<Option<Vec<Option<&str>>>> { basedpl::keyed::key_lists(&self.inner) }
    fn with_axis_keys(&self, keys: Vec<Option<Vec<Option<String>>>>) -> PyResult<Self> {
        Ok(Self { inner: basedpl::keyed::with_key_lists(self.inner.clone(), keys).map_err(PyValueError::new_err)? })
    }
    #[getter]
    fn is_atom(&self) -> bool { self.inner.is_atom() }
    fn parts(&self, py: Python<'_>) -> PyResult<Py<PyDict>> { array(py, &self.inner) }
    fn buffer<'py>(&self, py: Python<'py>) -> PyResult<Option<(&'static str, Bound<'py, PyByteArray>)>> {
        Ok(numpy(&self.inner).map(|(dtype, bytes)| (dtype, PyByteArray::new(py, &bytes))))
    }
    fn __repr__(&self) -> String { self.inner.to_string() }
    fn literal(&self) -> String { self.inner.literal() }
    fn atom(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        if !self.inner.is_singleton() { return Err(PyValueError::new_err("conversion requires a singleton array")); }
        if matches!(self.inner.at(0), Value::Array(_)) { return Err(PyTypeError::new_err("conversion requires an atom")); }
        array(py, &self.inner.at(0))
    }
    fn cells(&self) -> PyResult<Vec<Self>> {
        if self.inner.shape().is_empty() { return Err(PyTypeError::new_err("a unit has no major cells")); }
        let cells = self.inner.major_cells().map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(cells.into_iter().map(|inner| Self { inner }).collect())
    }
    fn select(&self, py: Python<'_>, parts: Vec<Option<PyRef<'_, PyArray>>>) -> PyResult<Py<PyDict>> {
        let parts = parts.into_iter().map(|a| a.map(|a| a.inner.clone())).collect::<Vec<_>>();
        let mut result = Evaluation::default();
        match self.inner.select(&parts) {
            Ok(Value::Function(f)) => result.function = Some(f),
            Ok(a) => result.value = Some(a),
            Err(e) => result.error = Some(e),
        }
        response(py, result)
    }
}

#[pyclass(frozen, name = "_Function")]
struct PyFunction { inner: Function }

fn operand(value: &Bound<'_, PyAny>) -> PyResult<Operand> {
    if let Ok(f) = value.extract::<PyRef<'_, PyFunction>>() { return Ok(Operand::Function(f.inner.clone())); }
    Ok(Operand::Value(value.extract::<PyRef<'_, PyArray>>()?.inner.clone()))
}

#[pymethods]
impl PyFunction {
    #[staticmethod]
    fn builtin(name: &str) -> PyResult<Self> {
        Function::builtin(name).map(|inner| Self { inner }).ok_or_else(|| PyValueError::new_err("unknown builtin function"))
    }
    #[staticmethod]
    fn late_bound(py: Python<'_>, expression: &str) -> PyResult<Py<PyDict>> {
        let result = match Function::late_bound(expression) {
            Ok(function) => Evaluation { function: Some(function), ..Evaluation::default() },
            Err(error) => Evaluation { error: Some(error), ..Evaluation::default() },
        };
        response(py, result)
    }
    #[staticmethod]
    fn build(kind: &str, values: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let operands = values.iter().map(operand).collect::<PyResult<Vec<_>>>()?;
        Function::build(kind, operands).map(|inner| Self { inner }).map_err(|e| PyValueError::new_err(e.to_string()))
    }
    fn __repr__(&self) -> String { self.inner.bpl() }
    fn parts(&self, py: Python<'_>) -> PyResult<Option<(String, Vec<Py<PyAny>>)>> {
        let Some((kind, operands)) = self.inner.parts() else { return Ok(None); };
        let args = operands
            .into_iter()
            .map(|a| match a {
                Operand::Function(inner) => Py::new(py, PyFunction { inner }).map(|f| f.into_any()),
                Operand::Value(inner) => Py::new(py, PyArray { inner }).map(|a| a.into_any()),
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(Some((kind, args)))
    }
}

/// An operator held as a value, as a module record holds it.
#[pyclass(frozen, name = "_Operator")]
struct PyOperator { inner: basedpl::Operator }

#[pymethods]
impl PyOperator {
    /// The function this operator derives from its operand, or from both operands of a dyadic operator.
    #[pyo3(signature = (left, right=None))]
    fn derive(&self, left: &Bound<'_, PyAny>, right: Option<&Bound<'_, PyAny>>) -> PyResult<PyFunction> {
        let right = right.map(operand).transpose()?;
        self.inner.derive(operand(left)?, right).map(|inner| PyFunction { inner }).map_err(|e| PyValueError::new_err(e.to_string()))
    }
    fn __repr__(&self) -> String { self.inner.to_string() }
}
/// What a request runs: source code, or a function called with arguments.
enum Run { Code(String), Call(Function, Vec<Value>) }
struct EvalRequest { run: Option<Run>, bindings: Vec<(String, Operand)>, options: EvalOptions }
impl EvalRequest {
    fn run(&mut self, session: &mut Session) -> Result<Evaluation, &'static str> {
        for (name, value) in self.bindings.drain(..) {
            match value { Operand::Value(a) => session.set(&name, a), Operand::Function(f) => session.set_function(&name, f) }
            .map_err(|_| "binding requires an ordinary BPL name and an exportable value")?;
        }
        let options = std::mem::take(&mut self.options);
        Ok(match &self.run {
            Some(Run::Code(code)) => session.eval_with(code, options),
            Some(Run::Call(function, args)) => session.call_function_with(function, args, options),
            None => Evaluation::default(),
        })
    }
}

#[pyclass(frozen, name = "_Session")]
struct PySession { session: Mutex<Session>, active: Mutex<Option<InterruptHandle>> }

#[pymethods]
impl PySession {
    #[new]
    fn new() -> Self { Self { session: Mutex::new(Session::new()), active: Mutex::new(None) } }
    /// Runs BPL `code` after binding `bindings`. `show` is called with each output event as BPL produces it, so output and input
    /// prompts appear in order.
    #[pyo3(signature = (*, code=None, bindings=Vec::new(), timeout=None, echo=false, show=None))]
    fn eval(
        &self,
        py: Python<'_>,
        code: Option<String>,
        bindings: Vec<(String, Bound<'_, PyAny>)>,
        timeout: Option<f64>,
        echo: bool,
        show: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyDict>> {
        let bindings = bindings.into_iter().map(|(n, a)| Ok((n, operand(&a)?))).collect::<PyResult<_>>()?;
        self.request(py, EvalRequest { run: code.map(Run::Code), bindings, options: options(timeout, echo)? }, show)
    }
    /// Calls `function` with `args`, as `eval` runs code.
    #[pyo3(signature = (function, args, *, timeout=None, show=None))]
    fn call(
        &self,
        py: Python<'_>,
        function: PyRef<'_, PyFunction>,
        args: Vec<PyRef<'_, PyArray>>,
        timeout: Option<f64>,
        show: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyDict>> {
        let run = Run::Call(function.inner.clone(), args.iter().map(|a| a.inner.clone()).collect());
        self.request(py, EvalRequest { run: Some(run), bindings: Vec::new(), options: options(timeout, false)? }, show)
    }
    #[pyo3(signature = (*, name=None, function=None))]
    fn inspect(&self, py: Python<'_>, name: Option<String>, function: Option<PyRef<'_, PyFunction>>) -> PyResult<Option<Py<PyDict>>> {
        let info = {
            let session = self.session.lock_py_attached(py).unwrap();
            match (name, function) { (Some(name), _) => session.inspect(&name), (_, Some(f)) => Some(f.inner.inspect(&session)), _ => None }
        };
        info.map(|i| {
            let d = PyDict::new(py);
            d.set_item("kind", i.kind)?;
            d.set_item("source", i.source)?;
            d.set_item("help", i.help)?;
            Ok(d.unbind())
        })
        .transpose()
    }
    #[pyo3(signature = (prefix="", classes=vec![2, 3, 4], complete=false))]
    fn names(&self, py: Python<'_>, prefix: &str, classes: Vec<i64>, complete: bool) -> Vec<String> {
        let session = self.session.lock_py_attached(py).unwrap();
        if complete { session.complete(prefix) } else { session.name_list(&classes, prefix) }
    }
    fn interrupt(&self) { if let Some(active) = &*self.active.lock().unwrap() { active.interrupt(); } }
}

impl PySession {
    /// Runs `request` with Python's Ctrl-C, `input()` and the `show` callback attached, and gives its result as a dict.
    fn request(&self, py: Python<'_>, mut request: EvalRequest, show: Option<Py<PyAny>>) -> PyResult<Py<PyDict>> {
        let signal = Arc::new(Mutex::new(None));
        let shown = Arc::new(Mutex::new(Vec::new()));
        request.options.poll = Some(ctrl_c(signal.clone()));
        request.options.input = Some(Arc::new(PythonInput { caught: signal.clone() }));
        request.options.output = show.map(|show| stream(show, shown.clone(), signal.clone()));
        let mut result = {
            let mut guard = self.session.lock_py_attached(py).unwrap();
            let session = &mut *guard;
            *self.active.lock().unwrap() = Some(request.options.interrupt.clone());
            let result = py.detach(move || request.run(session));
            *self.active.lock().unwrap() = None;
            result.map_err(PyValueError::new_err)?
        };
        result.output.append(&mut shown.lock().unwrap());
        if let Some(e) = signal.lock().unwrap().take() {
            e.value(py).setattr("output", result.output_text())?;
            e.value(py).setattr("events", output(py, &result.output)?)?;
            return Err(e);
        }
        response(py, result)
    }
}

/// A poll that interrupts the evaluation when Python has a pending signal, such as Ctrl-C, or when an output callback raised. It keeps the
/// error to raise.
fn ctrl_c(caught: Arc<Mutex<Option<PyErr>>>) -> basedpl::Poll {
    Arc::new(move || {
        if caught.lock().unwrap().is_some() { return true; }
        let Err(e) = Python::attach(|py| py.check_signals()) else { return false };
        *caught.lock().unwrap() = Some(e);
        true
    })
}

/// An output sink that keeps each event in `shown` and calls `show` with it. An error that `show` raises goes to `caught`, which stops
/// the evaluation through `ctrl_c`.
fn stream(show: Py<PyAny>, shown: Arc<Mutex<Vec<basedpl::Output>>>, caught: Arc<Mutex<Option<PyErr>>>) -> basedpl::OutputSink {
    Arc::new(move |event| {
        shown.lock().unwrap().push(event.clone());
        let called = Python::attach(|py| python(py, &basedpl::protocol::output(event)).and_then(|e| show.call1(py, (e,))).map(drop));
        if let Err(e) = called { caught.lock().unwrap().get_or_insert(e); }
    })
}

/// Python's standard input. Each line comes from `input()`, which shows a notebook's input box under IPython. A Ctrl-C while `input()`
/// waits goes to `caught`, as a signal does in `ctrl_c`.
struct PythonInput { caught: Arc<Mutex<Option<PyErr>>> }
impl basedpl::Input for PythonInput {
    fn line(&self, prompt: &str) -> std::io::Result<Option<String>> {
        Python::attach(|py| match py.import("builtins").and_then(|b| b.call_method1("input", (prompt,))).and_then(|line| line.extract::<String>()) {
            Ok(line) => Ok(Some(line)),
            Err(e) if e.is_instance_of::<pyo3::exceptions::PyEOFError>(py) => Ok(None),
            Err(e) if e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py) => {
                *self.caught.lock().unwrap() = Some(e);
                Err(std::io::ErrorKind::Interrupted.into())
            }
            Err(e) => Err(std::io::Error::other(e.to_string())),
        })
    }
    fn rest(&self) -> std::io::Result<Vec<u8>> {
        Python::attach(|py| -> PyResult<String> { py.import("sys")?.getattr("stdin")?.call_method0("read")?.extract() })
            .map(String::into_bytes)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

fn options(timeout: Option<f64>, echo: bool) -> PyResult<basedpl::EvalOptions> {
    let timeout = timeout
        .map(|seconds| std::time::Duration::try_from_secs_f64(seconds).map_err(|_| PyValueError::new_err("timeout must be finite and nonnegative")))
        .transpose()?;
    Ok(basedpl::EvalOptions { timeout, echo, ..basedpl::EvalOptions::default() })
}

/// A JSON value as the matching Python object.
fn python(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    use serde_json::Value as Json;
    match value {
        Json::Null => Ok(py.None()),
        Json::Bool(b) => b.into_py_any(py),
        Json::Number(n) => n.as_i64().map_or_else(|| n.as_f64().into_py_any(py), |i| i.into_py_any(py)),
        Json::String(s) => s.into_py_any(py),
        Json::Array(items) => items.iter().map(|v| python(py, v)).collect::<PyResult<Vec<_>>>()?.into_py_any(py),
        Json::Object(fields) => {
            let d = PyDict::new(py);
            for (k, v) in fields { d.set_item(k, python(py, v)?)?; }
            d.into_py_any(py)
        }
    }
}

/// A result as the dict that the JSON protocol sends, holding native arrays and functions.
fn response(py: Python<'_>, result: Evaluation) -> PyResult<Py<PyDict>> {
    let value = if let Some(inner) = result.value { Some(Py::new(py, PyArray { inner })?.into_any()) } else if let Some(inner) = result.function { Some(Py::new(py, PyFunction { inner })?.into_any()) } else if let Some(inner) = result.operator { Some(Py::new(py, PyOperator { inner })?.into_any()) } else { None };
    let d = PyDict::new(py);
    d.set_item("value", value)?;
    d.set_item("output", output(py, &result.output)?)?;
    d.set_item("error", result.error.as_ref().map(|e| error(py, e)).transpose()?)?;
    Ok(d.unbind())
}

fn output(py: Python<'_>, events: &[basedpl::Output]) -> PyResult<Py<PyAny>> { python(py, &events.iter().map(basedpl::protocol::output).collect()) }

#[pyfunction]
fn _check_reference(case: &str, timeout: f64) -> PyResult<String> {
    let case = serde_json::from_str(case).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(basedpl::reference::check(&case, options(Some(timeout), false)?).to_string())
}

/// BPL source for a value captured from another interpreter, given as JSON.
#[pyfunction]
fn _captured_literal(value: &str) -> PyResult<String> {
    let value = serde_json::from_str(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    basedpl::reference::expected_array(&value).map(|v| v.literal()).ok_or_else(|| PyValueError::new_err("invalid captured array"))
}

/// The character offset of each byte offset in `text` that starts a character or ends the text. Python indexes strings by character.
fn char_offsets(text: &str) -> Vec<usize> {
    let mut chars = vec![0; text.len() + 1];
    for (i, (byte, _)) in text.char_indices().enumerate() { chars[byte] = i; }
    chars[text.len()] = text.chars().count();
    chars
}

/// `protocol::error` with character spans. The JSON worker protocol keeps byte spans.
fn error(py: Python<'_>, e: &basedpl::Error) -> PyResult<Py<PyAny>> {
    let span = |s: &basedpl::Span| { let chars = char_offsets(&s.source.text); serde_json::json!([chars[s.range.start], chars[s.range.end]]) };
    let mut encoded = basedpl::protocol::error(e);
    encoded["span"] = span(&e.span);
    if let Some(calls) = encoded["calls"].as_array_mut() { for (call, s) in calls.iter_mut().zip(&e.calls) { call["span"] = span(s); } }
    python(py, &encoded)
}

/// The parse tree of `text` as nested `(kind, start, end, children, error)` tuples, with character offsets. `error` is the syntax error of an `Error` node.
#[pyfunction]
fn _parse(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    fn node(py: Python<'_>, n: &basedpl::SyntaxNode, chars: &[usize]) -> PyResult<Py<PyAny>> {
        let children = n.children.iter().map(|c| node(py, c, chars)).collect::<PyResult<Vec<_>>>()?;
        let error = n.error.as_ref().map(|e| error(py, e)).transpose()?;
        (n.kind, chars[n.range.start], chars[n.range.end], children, error).into_py_any(py)
    }
    node(py, &basedpl::syntax_tree(text), &char_offsets(text))
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_class::<PyArray>()?;
    m.add_class::<PyFunction>()?;
    m.add_class::<PyOperator>()?;
    m.add_function(wrap_pyfunction!(_check_reference, m)?)?;
    m.add_function(wrap_pyfunction!(_captured_literal, m)?)?;
    m.add_function(wrap_pyfunction!(_parse, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("symbols", python(m.py(), &basedpl::symbols::rows())?)?;
    m.add("_system_functions", basedpl::system::names().collect::<Vec<_>>())?;
    m.add("_superscripts", basedpl::superscripts())?;
    m.add("_subscripts", basedpl::subscripts())?;
    Ok(())
}
