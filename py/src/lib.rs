use basedpl::{buffer::{Buffer, DType, FloatWidth, IntBuf, View}, EvalOptions, Evaluation, Function, InterruptHandle, Number, Operand, Session, Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use pyo3::{
    exceptions::{PyBufferError, PyKeyError, PyTypeError, PyValueError}, ffi, prelude::*, sync::{MutexExt, PyOnceLock},
    types::{PyBool, PyBoolMethods, PyComplex, PyComplexMethods, PyDict, PyFloat, PyInt, PyList, PyRange, PySlice, PyString, PyTuple, PyType}, IntoPyObjectExt
};
use rustymath::{layout::Keys, structure::IndexPart};
use rustymath_buffer::walk::{Collect, walked, within};
use std::{ffi::c_int, sync::{atomic::{AtomicPtr, Ordering}, Arc, Mutex}};

/// The collector for Python data that becomes BPL values: each item converts on its own, and a level whose items differ in shape nests them, with Python floats at `width`.
struct Items { width: FloatWidth }

impl Collect for Items {
    type Piece = Value;
    fn int(&mut self, n: i128) -> Value { Value::Number(BigInt::from(n).into()) }
    fn piece(&mut self, obj: &Bound<'_, PyAny>) -> PyResult<Option<(Value, Vec<usize>)>> { Ok(Some((element(obj, self.width)?, vec![]))) }
    fn element(&mut self, obj: &Bound<'_, PyAny>) -> PyResult<Value> { element(obj, self.width) }
}

fn value_error(e: impl ToString) -> PyErr { PyValueError::new_err(e.to_string()) }

/// A float zero at `width`, as the prototype of imported numbers.
fn zero(width: FloatWidth) -> Value { Value::Number(Number::float(0.0, width).marked(true)) }

/// The array that the Python data `obj` gives, with Python floats at `width`. Nested lists, tuples and ranges give axes, a `str` gives a character vector, and any other object gives the value it gives as an item. Numbers take the import rules: they become floats only when every number converts exactly.
fn import(obj: &Bound<'_, PyAny>, width: FloatWidth) -> PyResult<Value> {
    if let Ok(s) = obj.cast::<PyString>() { return characters(s); }
    let (layout, mut items) = walked(obj, &mut Items { width })?;
    if layout.shape().is_empty() { return Ok(items.pop().expect("a unit holds one item")); }
    Value::imported(layout.shape().to_vec(), items, zero(width)).map_err(value_error)
}

/// The characters of `s`, as a character vector.
fn characters(s: &Bound<'_, PyString>) -> PyResult<Value> {
    let chars: Vec<Value> = s.to_str()?.chars().map(Value::Character).collect();
    Value::imported(vec![chars.len()], chars, Value::Character(' ')).map_err(value_error)
}

/// The item that the `str` `s` gives: one character, or a character vector.
fn text(s: &Bound<'_, PyString>) -> PyResult<Value> {
    let mut chars = s.to_str()?.chars();
    match (chars.next(), chars.next()) { (Some(c), None) => Ok(Value::Character(c)), _ => characters(s) }
}

/// A keyed vector of `data`, as a dict's values or a frame's columns give, with its axis keyed by `keys` and named `name`. It reads as a literal of its items reads, so exact and approximate numbers stay apart, as in a JSON object.
fn record(keys: Arc<Keys>, data: Vec<Value>, name: Option<Arc<str>>, width: FloatWidth) -> PyResult<Value> {
    labelled(Value::from_parts(vec![data.len()], data, zero(width)).map_err(value_error)?, vec![Some(keys)], vec![name])
}

/// `value` with `keys` and `names` on its axes.
fn labelled(value: Value, keys: Vec<Option<Arc<Keys>>>, names: Vec<Option<Arc<str>>>) -> PyResult<Value> {
    let layout = value.core_layout().clone().with_keys(keys).and_then(|l| l.with_names(names)).map_err(rustymath_buffer::py_err)?;
    value.with_core_layout(layout).map_err(value_error)
}

/// The value of an object that exports a buffer, such as a rustymath array or a `memoryview`, with its dtype and labels.
fn numeric(obj: &Bound<'_, PyAny>) -> PyResult<Option<Value>> {
    let Some((read, layout)) = rustymath_buffer::read_array(obj)? else { return Ok(None) };
    Value::imported_buffer(layout.shape(), read.into_buffer()).and_then(|v| v.with_core_layout(layout)).map(Some).map_err(value_error)
}

/// NumPy's `generic` and `ndarray` types, once NumPy is imported. Until then no object is NumPy's, so NumPy needn't be installed.
fn numpy_types(py: Python<'_>) -> PyResult<Option<&'static (Py<PyType>, Py<PyType>)>> {
    static TYPES: PyOnceLock<(Py<PyType>, Py<PyType>)> = PyOnceLock::new();
    if let Some(types) = TYPES.get(py) { return Ok(Some(types)); }
    let Ok(np) = py.import("sys")?.getattr("modules")?.get_item("numpy") else { return Ok(None) };
    let types = (np.getattr("generic")?.cast_into::<PyType>()?.unbind(), np.getattr("ndarray")?.cast_into::<PyType>()?.unbind());
    Ok(Some(TYPES.get_or_init(py, || types)))
}

/// The value of a NumPy scalar or array, or `None` for any other object. A scalar of a numeric dtype keeps its dtype. Arrays of `str` or objects convert item by item, as do unsigned 64-bit values beyond `int64`, which stay exact.
fn numpy(obj: &Bound<'_, PyAny>, width: FloatWidth) -> PyResult<Option<Value>> {
    let Some((generic, ndarray)) = numpy_types(obj.py())? else { return Ok(None) };
    let dtype = |o: &Bound<'_, PyAny>| -> PyResult<(char, usize)> { let d = o.getattr("dtype")?; Ok((d.getattr("kind")?.extract()?, d.getattr("itemsize")?.extract()?)) };
    if obj.is_instance(generic.bind(obj.py()))? {
        if !"biufc".contains(dtype(obj)?.0) { return element(&obj.call_method0("item")?, width).map(Some); }
        let value = numeric(&obj.call_method0("__array__")?)?.ok_or_else(|| PyTypeError::new_err("unsupported NumPy dtype"))?;
        return Ok(Some(value.at(0)));
    }
    if !obj.is_instance(ndarray.bind(obj.py()))? { return Ok(None); }
    let (mut kind, size) = dtype(obj)?;
    if !"biufcUO".contains(kind) || kind == 'f' && size > 8 || kind == 'c' && size > 16 { return Err(PyTypeError::new_err("unsupported NumPy dtype")); }
    let mut obj = obj.clone();
    if kind == 'u' && size == 8 && obj.getattr("size")?.extract::<usize>()? > 0 && obj.call_method0("max")?.ge(1u64 << 63)? { (obj, kind) = (obj.call_method1("astype", ("O",))?, 'O'); }
    if !"UO".contains(kind) { return numeric(&obj); }
    let data = obj.call_method0("ravel")?.call_method0("tolist")?.try_iter()?.map(|o| element(&o?, width)).collect::<PyResult<Vec<_>>>()?;
    let prototype = if kind == 'U' { Value::Character(' ') } else { zero(width) };
    Value::imported(obj.getattr("shape")?.extract()?, data, prototype).map(Some).map_err(value_error)
}

/// Python's `fractions.Fraction`.
fn fraction(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
    static FRACTION: PyOnceLock<Py<PyType>> = PyOnceLock::new();
    FRACTION.import(py, "fractions", "Fraction")
}

/// The BPL value of `obj` as an item of an array, with Python floats at `width`. A Python float or complex number is weak, a complex number follows the current settings' kinds, and `None` reads as NaN. A `dict` and a data frame give keyed vectors, and a list gives a nested array.
fn element(obj: &Bound<'_, PyAny>, width: FloatWidth) -> PyResult<Value> {
    if let Ok(b) = obj.cast_exact::<PyBool>() { return Ok(Value::Number(Number::from_bool(b.is_true()))); }
    if obj.is_exact_instance_of::<PyInt>() { return Ok(Value::Number(obj.extract::<BigInt>()?.into())); }
    if obj.is_exact_instance_of::<PyFloat>() { return Ok(Value::Number(Number::float(obj.extract()?, width).marked(true))); }
    if let Ok(z) = obj.cast_exact::<PyComplex>() {
        return Ok(Value::Number(Number::complex_in_session(num_complex::Complex64::new(z.real(), z.imag()), width).marked(true)));
    }
    if obj.is_none() { return Ok(Value::Number(Number::float(f64::NAN, width))); }
    if let Ok(s) = obj.cast::<PyString>() { return text(s); }
    // A native value, or Python's `Array`, `Function` or `Operator` around one.
    for o in [Some(obj.clone()), obj.getattr_opt("_inner")?].into_iter().flatten() {
        if let Ok(a) = o.extract::<PyRef<'_, PyArray>>() { return a.value().cloned(); }
        if let Ok(f) = o.extract::<PyRef<'_, PyFunction>>() { return Ok(Value::Function(f.inner.clone())); }
        if let Ok(op) = o.extract::<PyRef<'_, PyOperator>>() { return Ok(Value::Operator(op.inner.clone())); }
    }
    if let Ok(d) = obj.cast::<PyDict>() {
        let keys = Keys::partial(rustymath_buffer::labels(&d.keys())?).map_err(rustymath_buffer::py_err)?;
        let data = within(obj, || d.values().iter().map(|v| element(&v, width)).collect::<PyResult<Vec<_>>>())?;
        return record(keys, data, None, width);
    }
    if obj.is_instance_of::<PyList>() || obj.is_instance_of::<PyTuple>() || obj.is_instance_of::<PyRange>() { return import(obj, width); }
    if obj.is_instance(fraction(obj.py())?)? {
        let rational = BigRational::new_raw(obj.getattr("numerator")?.extract()?, obj.getattr("denominator")?.extract()?);
        return Number::try_from(rational).map(Value::Number).map_err(value_error);
    }
    if let Some(value) = numpy(obj, width)? { return Ok(value); }
    if let Some(frame) = rustymath_buffer::frame::frame(obj)? {
        let ([rows, cols], [row, col]) = (frame.keys, frame.names);
        let columns = frame.columns.iter().map(|c| labelled(import(c, width)?, vec![rows.clone()], vec![row.clone()])).collect::<PyResult<Vec<_>>>()?;
        return record(cols.ok_or_else(|| PyValueError::new_err("a frame's columns need labels"))?, columns, col, width);
    }
    if let Some(value) = numeric(obj)? { return Ok(value); }
    if obj.hasattr("__array__")? { return within(obj, || import(&obj.call_method0("__array__")?, width)); }
    Err(PyTypeError::new_err(format!("cannot convert {} to BPL", obj.get_type().name()?)))
}

/// `a`'s parts for Python: its `_Array` for compact items, which Python reads through `tolist` or the buffer protocol, or else a dict of its shape, items, prototype, keys and names.
fn array(py: Python<'_>, a: &Value) -> PyResult<Py<PyAny>> {
    fn element(py: Python<'_>, e: &Value) -> PyResult<Py<PyAny>> {
        Ok(match e {
            Value::Number(n) => {
                if let Some(b) = n.as_bool() { PyBool::new(py, b).to_owned().into_any().unbind() }
                else if let Some(n) = n.as_integer() { n.into_pyobject(py)?.into_any().unbind() }
                else if let Some(n) = n.as_exact() {
                    if n.is_integer() { n.numer().into_pyobject(py)?.into_any().unbind() }
                    else { (n.numer(), n.denom()).into_pyobject(py)?.into_any().unbind() }
                } else if let Some(n) = n.as_complex() { PyComplex::from_doubles(py, n.re, n.im).into_any().unbind() }
                else { PyFloat::new(py, n.as_float().unwrap()).into_any().unbind() }
            }
            Value::Character(c) => PyString::new(py, &c.to_string()).into_any().unbind(),
            a @ Value::Array(_) => array(py, a)?,
            Value::Function(f) => Py::new(py, PyFunction { inner: f.clone() })?.into_any(),
            Value::Operator(op) => Py::new(py, PyOperator { inner: op.clone() })?.into_any(),
            Value::Carried(_) => return Err(PyValueError::new_err("a value that a running ∂ carries has no Python value")),
        })
    }
    if !a.is_atom() && a.view().is_some() { return Ok(Py::new(py, PyArray::from(a.clone()))?.into_any()); }
    let result = PyDict::new(py);
    if a.is_atom() { result.set_item("atom", element(py, a)?)?; return Ok(result.into_any().unbind()); }
    let data = PyList::empty(py);
    for e in a.elements() { data.append(element(py, &e)?)?; }
    result.set_item("shape", a.shape())?;
    result.set_item("data", data)?;
    result.set_item("prototype", element(py, &a.prototype())?)?;
    if !a.axis_names().is_empty() { result.set_item("axis_names", a.axis_names().iter().map(|n| n.as_deref()).collect::<Vec<_>>())?; }
    if a.has_keys() { result.set_item("axis_keys", basedpl::keyed::key_lists(a))?; }
    Ok(result.into_any().unbind())
}

/// A BPL value, or nothing once `own` has moved its items into NumPy.
#[pyclass(name = "_Array")]
struct PyArray { inner: Option<Value> }

impl From<Value> for PyArray {
    fn from(inner: Value) -> Self { Self { inner: Some(inner) } }
}

impl PyArray {
    /// The value, or an error once `own` has moved its items.
    fn value(&self) -> PyResult<&Value> { self.inner.as_ref().ok_or_else(|| PyValueError::new_err("own() moved this array's items into NumPy")) }
}

enum IndexSelection { Planned(rustymath::structure::Plan, bool), Rich(Vec<IndexPart<Value>>) }

/// The Python exception for an indexing error: `KeyError` for a missing key, and otherwise the exception for the error's kind, with BPL's message.
fn index_error(error: basedpl::Error) -> PyErr {
    let basedpl::ErrorKind::Array(e) = &error.kind else { return PyValueError::new_err(error.to_string()) };
    let text = if error.message.is_empty() { e.to_string() } else { error.message.clone() };
    if let Some(key) = text.strip_prefix("missing key: ") { return PyKeyError::new_err(key.to_owned()); }
    rustymath_buffer::py_err(rustymath::error::Error::new(e.kind, text))
}

/// Decodes a Python index of `value` once, for reads and writes, through `⌷`'s planner. Python's own kinds, slices, keys, integers and lists of keys, decode without converting. `convert` turns other selectors into arrays, which decode as `⌷` decodes them, except Boolean masks, which select where they hold true. Pick paths allow positions, keys, full slices, None and ... as suffixes, but not partial slices or masks.
fn selection(value: &Value, index: &Bound<'_, PyAny>, convert: &Bound<'_, PyAny>) -> PyResult<IndexSelection> {
    let converted = |obj: &Bound<'_, PyAny>| -> PyResult<Value> { Ok(convert.call1((obj,))?.extract::<PyRef<'_, PyArray>>()?.value()?.clone()) };
    let parts = rustymath_buffer::selection::parts(index)?;
    let values = parts.iter().map(|p| p.as_ref().try_map(|p| if rustymath_buffer::selection::native(p) { Ok(None) } else { converted(p).map(Some) })).collect::<PyResult<Vec<_>>>()?;
    if let Some(IndexPart::Axis(Some(first))) = values.iter().find(|p| matches!(p, IndexPart::Axis(_) | IndexPart::All)) && value.picks_paths(first).map_err(index_error)? {
        let paths = parts.iter().zip(values).map(|(p, v)| v.try_map(|v| match (p.axis(), v) {
            (_, Some(v)) if !matches!(v.view(), Some(View::Booleans(_))) => Ok(v),
            (Some(p), None) if !p.is_instance_of::<PySlice>() => converted(p),
            _ => Err(PyTypeError::new_err("an index with pick paths takes positions, keys, full slices, None and ..., not partial slices or masks")),
        })).collect::<PyResult<Vec<_>>>()?;
        return Ok(IndexSelection::Rich(paths));
    }
    let layout = value.core_layout();
    value.planned(&values, |i, axis| {
        let (IndexPart::Axis(p), IndexPart::Axis(v)) = (&parts[i], &values[i]) else { unreachable!("planned decodes axis parts") };
        match v {
            None => Some(rustymath_buffer::selection::selector(layout, axis, p, || unreachable!("a native selector decodes without converting"))),
            Some(v) => match v.view() {
                Some(mask @ View::Booleans(_)) => Some(rustymath_buffer::selection::array_selector(layout, axis, v.core_layout().clone(), mask, false)),
                _ => None,
            },
        }
    }, index_error).map(|(plan, atom)| IndexSelection::Planned(plan, atom))
}

/// The array whose compact items an export lends for `value`, with the offset of `value`'s first item in them and, for a structural copy read through an affine map, each axis's stride in items. A copy that hasn't been built lends its source's items this way. Otherwise `value` lends its own items where they're compact. `None` where neither applies.
fn lendable(value: &Value) -> Option<(Value, usize, Option<Vec<isize>>)> {
    if let Some((source, offset, strides)) = value.affine().filter(|(source, ..)| source.view().is_some()) { return Some((source, offset, Some(strides))); }
    value.view().map(|_| (value.clone(), 0, None))
}

/// Items that `own` moved out of BPL, which NumPy views writably through `data`, their address, as `dtype`. NumPy can write any bytes into them, so Booleans are held as `u8` and characters as `i32`, which have no invalid values.
#[pyclass(frozen)]
struct Owned { dtype: DType, items: Buffer, data: AtomicPtr<u8>, shape: Box<[isize]>, strides: Box<[isize]> }

impl Owned {
    fn new(items: Buffer, shape: &[usize]) -> Self {
        let dtype = items.dtype();
        // SAFETY: `bool` and `u8`, and `char` and `i32`, have the same size and alignment, and every `bool` is a valid `u8` and every `char` a valid `i32`.
        let mut items = match items {
            Buffer::Booleans(v) => Buffer::Integers(IntBuf::U8(unsafe { retyped(v) })),
            Buffer::Characters(v) => Buffer::Integers(IntBuf::I32(unsafe { retyped(v) })),
            items => items,
        };
        let data = AtomicPtr::new(items.as_mut_ptr());
        let (shape, strides) = rustymath_buffer::geometry(dtype, shape);
        Self { dtype, items, data, shape, strides }
    }
}

/// `items` as a `Vec<U>` over the same allocation, with no copy.
///
/// # Safety
///
/// `T` and `U` must have the same size and alignment, and every `T` must be a valid `U`.
unsafe fn retyped<T, U>(items: Vec<T>) -> Vec<U> {
    let mut items = std::mem::ManuallyDrop::new(items);
    // SAFETY: the caller guarantees that the layouts match and that each item is a valid `U`.
    unsafe { Vec::from_raw_parts(items.as_mut_ptr().cast(), items.len(), items.capacity()) }
}

#[pymethods]
impl Owned {
    unsafe fn __getbuffer__(slf: Bound<'_, Self>, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
        let this = slf.get();
        // SAFETY: `data` addresses `items`, which the object owns with the shape and strides and never changes, and the export holds the object.
        unsafe { rustymath_buffer::export(view, flags, slf.as_any(), this.dtype, this.data.load(Ordering::Relaxed), this.items.len(), &this.shape, &this.strides, false) }
    }
}

#[pymethods]
impl PyArray {
    fn copy(&self) -> PyResult<Self> { Ok(self.value()?.clone().into()) }
    #[getter]
    fn shape(&self) -> PyResult<Vec<usize>> { Ok(self.value()?.shape().to_vec()) }
    #[getter]
    fn names(&self) -> PyResult<Vec<Option<String>>> {
        let value = self.value()?;
        Ok((0..value.shape().len()).map(|a| value.axis_name(a).map(|n| n.to_string())).collect())
    }
    fn with_names(&self, names: Option<Vec<Option<String>>>) -> PyResult<Self> {
        let value = self.value()?;
        let layout = value.core_layout().clone().with_names(names.unwrap_or_default().into_iter().map(|n| n.map(Arc::from)).collect()).map_err(rustymath_buffer::py_err)?;
        Ok(value.clone().with_core_layout(layout).map_err(|k| PyValueError::new_err(k.to_string()))?.into())
    }
    #[getter]
    fn keys(&self) -> PyResult<Vec<Option<Vec<Option<&str>>>>> { Ok(basedpl::keyed::key_lists(self.value()?)) }
    fn with_keys(&self, keys: Option<Vec<Option<Vec<Option<String>>>>>) -> PyResult<Self> {
        let value = self.value()?;
        let keys = rustymath_buffer::axis_keys(keys.unwrap_or_default())?;
        let layout = value.core_layout().clone().with_keys(keys).map_err(rustymath_buffer::py_err)?;
        Ok(value.clone().with_core_layout(layout).map_err(|k| PyValueError::new_err(k.to_string()))?.into())
    }
    fn __rustymath_array__(&self, py: Python<'_>) -> PyResult<(Self, Py<PyAny>, Vec<Option<String>>)> {
        if !self.exports()? { return Err(PyTypeError::new_err("only ordinary compact values can cross to Rustymath; convert exact or mixed values explicitly")); }
        Ok((self.copy()?, self.keys()?.into_py_any(py)?, self.names()?))
    }
    #[getter]
    fn is_atom(&self) -> PyResult<bool> { Ok(self.value()?.is_atom()) }
    /// Whether a direct item is a function, which makes the array callable.
    #[getter]
    fn holds_functions(&self) -> PyResult<bool> { Ok(self.value()?.holds_functions()) }
    fn parts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> { array(py, self.value()?) }
    /// The items as nested lists of Python scalars, or one scalar at rank 0, read without NumPy. Only compact items convert.
    fn tolist<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let value = self.value()?;
        let items = value.view().ok_or_else(|| PyTypeError::new_err("only compact items convert to a list"))?;
        rustymath_buffer::tolist(py, items, value.shape())
    }
    /// The string that an unkeyed, unnamed character vector holds, or `None`.
    #[getter]
    fn text(&self) -> PyResult<Option<String>> {
        let value = self.value()?;
        let plain = value.shape().len() == 1 && !value.has_keys() && value.axis_names().is_empty();
        Ok(match value.view() { Some(View::Characters(c)) if plain => Some(c.iter().collect()), _ => None })
    }
    /// Whether NumPy can read the items through the buffer protocol: compact storage, or a structural copy that reads compact items through an affine map.
    #[getter]
    fn exports(&self) -> PyResult<bool> { Ok(lendable(self.value()?).is_some()) }
    /// Exports the items read-only and without a copy, so that NumPy can't change a BPL value through them. An unbuilt structural copy with an affine read, such as windows or a reversal, lends its source's items with strides, without being built.
    unsafe fn __getbuffer__(slf: Bound<'_, Self>, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
        let value = slf.borrow().value()?.clone();
        let (items, offset, strides) = lendable(&value).ok_or_else(|| PyBufferError::new_err("an array of mixed or exact items has no buffer"))?;
        let dtype = items.view().expect("lendable items are compact").dtype();
        // SAFETY: the offset and strides reach only items of `items`, as the copy's map does. An atom holds its items inline, so the pointer comes from the copy that the export holds.
        unsafe { rustymath_buffer::lend(view, flags, slf.py(), items, |v| v.view().expect("the items are compact").as_ptr().add(offset * dtype.bytes()), dtype, value.shape(), strides.as_deref()) }
    }
    /// The items for NumPy to view writably: moved with no copy where nothing else holds them, and copied otherwise. For items it can't move, such as exact numbers, it still empties the array and returns `None`, so the caller copies them first.
    fn own(&mut self) -> PyResult<Option<Owned>> {
        self.value()?;
        let value = self.inner.take().expect("checked above");
        let shape = value.shape().to_vec();
        Ok(value.into_buffer().ok().map(|items| Owned::new(items, &shape)))
    }
    fn __repr__(&self) -> PyResult<String> { Ok(self.value()?.to_string()) }
    fn literal(&self) -> PyResult<String> { Ok(self.value()?.literal()) }
    fn native_atom(&self) -> PyResult<Self> {
        let value = self.value()?;
        if !value.is_singleton() { return Err(PyValueError::new_err("conversion requires a singleton array")); }
        let item = value.at(0);
        if matches!(item, Value::Array(_)) { return Err(PyTypeError::new_err("conversion requires an atom")); }
        Ok(item.into())
    }
    fn atom(&self, py: Python<'_>) -> PyResult<Py<PyAny>> { array(py, self.native_atom()?.value()?) }
    fn cells(&self) -> PyResult<Vec<Self>> {
        let value = self.value()?;
        if value.shape().is_empty() { return Err(PyTypeError::new_err("a unit has no major cells")); }
        let cells = value.major_cells().map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(cells.into_iter().map(Self::from).collect())
    }
    fn select(&self, py: Python<'_>, index: &Bound<'_, PyAny>, convert: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let value = match selection(self.value()?, index, convert)? {
            IndexSelection::Planned(plan, atom) => self.value()?.selected(plan, atom),
            IndexSelection::Rich(parts) => self.value()?.select(&parts),
        }.map_err(index_error)?;
        let mut result = Evaluation::default();
        match value { Value::Function(f) => result.function = Some(f), a => result.value = Some(a) }
        response(py, result)
    }
    fn assign(&self, index: &Bound<'_, PyAny>, values: PyRef<'_, PyArray>, convert: &Bound<'_, PyAny>) -> PyResult<Self> {
        match selection(self.value()?, index, convert)? {
            IndexSelection::Planned(plan, atom) => self.value()?.assigned(plan, atom, values.value()?),
            IndexSelection::Rich(parts) => self.value()?.assigned_parts(&parts, values.value()?),
        }.map(Self::from).map_err(index_error)
    }
    /// Whether this array holds `query`, a native array or a `str`. A one-character string is an atom. A longer one is one item where the major cells are items, as a list of strings holds them, and otherwise a character vector, which matches a row of characters.
    fn contains(&self, py: Python<'_>, query: &Bound<'_, PyAny>) -> PyResult<bool> {
        let value = self.value()?.clone();
        let query = match query.cast::<PyString>() {
            Ok(s) => match text(s)? {
                chars @ Value::Array(_) if value.shape().len() == 1 => Value::from_parts(Vec::<usize>::new(), vec![chars], Value::Character(' ')).map_err(value_error)?,
                item => item,
            },
            Err(_) => query.extract::<PyRef<'_, PyArray>>()?.value()?.clone(),
        };
        py.detach(move || value.contains(&query)).map_err(|e| match &e.kind {
            basedpl::ErrorKind::Array(e) if e.kind == rustymath::error::Kind::Rank => PyTypeError::new_err(e.message.to_string()),
            _ => PyValueError::new_err(e.to_string()),
        })
    }
}

#[pyclass(frozen, name = "_Function")]
struct PyFunction { inner: Function }

fn operand(value: &Bound<'_, PyAny>) -> PyResult<Operand> {
    if let Ok(f) = value.extract::<PyRef<'_, PyFunction>>() { return Ok(Operand::Function(f.inner.clone())); }
    Ok(Operand::Value(value.extract::<PyRef<'_, PyArray>>()?.value()?.clone()))
}

#[pymethods]
impl PyFunction {
    #[staticmethod]
    fn builtin(name: &str) -> PyResult<Self> {
        Function::builtin(name).map(|inner| Self { inner }).ok_or_else(|| PyValueError::new_err("unknown builtin function"))
    }
    #[staticmethod]
    fn build(kind: &str, values: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let operands = values.iter().map(operand).collect::<PyResult<Vec<_>>>()?;
        Function::build(kind, operands).map(|inner| Self { inner }).map_err(|e| PyValueError::new_err(e.to_string()))
    }
    fn __repr__(&self) -> String { self.inner.bpl() }
    /// This function with `g` attached as its inverse, as BPL's `f⁻¹←g` attaches it.
    fn with_inverse(&self, g: PyRef<'_, PyFunction>) -> Self { Self { inner: self.inner.with_inverse(g.inner.clone()) } }
    /// This function with `g` attached as its derivative, as BPL's `f∂←g` attaches it.
    fn with_derivative(&self, g: PyRef<'_, PyFunction>) -> Self { Self { inner: self.inner.with_derivative(g.inner.clone()) } }
    fn parts(&self, py: Python<'_>) -> PyResult<Option<(String, Vec<Py<PyAny>>)>> {
        let Some((kind, operands)) = self.inner.parts() else { return Ok(None); };
        let args = operands.into_iter().map(|a| match a {
            Operand::Function(inner) => Py::new(py, PyFunction { inner }).map(|f| f.into_any()),
            Operand::Value(inner) => Py::new(py, PyArray::from(inner)).map(|a| a.into_any()),
        }).collect::<PyResult<Vec<_>>>()?;
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
            match value {
                Operand::Value(a) => session.set(&name, a), Operand::Function(f) => session.set_function(&name, f)
            }.map_err(|_| "binding requires an ordinary BPL name and an exportable value")?;
        }
        let options = std::mem::take(&mut self.options);
        Ok(match &self.run {
            Some(Run::Code(code)) => session.eval_with(code, options), Some(Run::Call(function, args)) => session.call_function_with(function, args, options),
            None => Evaluation::default()
        })
    }
}

#[pyclass(frozen, name = "_Session")]
struct PySession { session: Mutex<Session>, active: Mutex<Option<InterruptHandle>> }

#[pymethods]
impl PySession {
    #[new]
    fn new() -> Self { Self { session: Mutex::new(Session::new()), active: Mutex::new(None) } }
    /// Runs BPL `code` after binding `bindings`. `show` is called with each output event as BPL produces it, so output and input prompts appear in order.
    #[pyo3(signature = (*, code=None, bindings=Vec::new(), timeout=None, echo=false, show=None))]
    fn eval(
        &self, py: Python<'_>, code: Option<String>, bindings: Vec<(String, Bound<'_, PyAny>)>, timeout: Option<f64>, echo: bool, show: Option<Py<PyAny>>
    ) -> PyResult<Py<PyDict>> {
        let bindings = bindings.into_iter().map(|(n, a)| Ok((n, operand(&a)?))).collect::<PyResult<_>>()?;
        self.request(py, EvalRequest { run: code.map(Run::Code), bindings, options: options(timeout, echo)? }, show)
    }
    /// Calls `function` with `args`, as `eval` runs code. A Python float or complex scalar arrives as a weak number (`Number::is_weak`).
    #[pyo3(signature = (function, args, *, timeout=None, show=None))]
    fn call(
        &self, py: Python<'_>, function: PyRef<'_, PyFunction>, args: Vec<PyRef<'_, PyArray>>, timeout: Option<f64>, show: Option<Py<PyAny>>
    ) -> PyResult<Py<PyDict>> {
        let values = args.iter().map(|a| a.value().cloned()).collect::<PyResult<Vec<_>>>()?;
        self.request(py, EvalRequest { run: Some(Run::Call(function.inner.clone(), values)), bindings: Vec::new(), options: options(timeout, false)? }, show)
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
        }).transpose()
    }
    /// The function that `expression` writes, late-bound, with its numbers read at the session's default width.
    fn late_bound(&self, py: Python<'_>, expression: &str) -> PyResult<Py<PyDict>> {
        let result = match self.session.lock_py_attached(py).unwrap().late_bound(expression) {
            Ok(function) => Evaluation { function: Some(function), ..Evaluation::default() },
            Err(error) => Evaluation { error: Some(error), ..Evaluation::default() },
        };
        response(py, result)
    }
    /// The array that the Python data `obj` gives. Python floats take the default float width.
    fn array(&self, obj: &Bound<'_, PyAny>) -> PyResult<PyArray> {
        let width = basedpl::settings::float_width();
        Ok(import(obj, width)?.into())
    }
    /// Makes the display defaults of BPL's REPL and Jupyter kernel the current ones, for the BPL magics in IPython. Settings are process-wide, so this takes no lock on the session.
    fn make_interactive(&self) { Session::make_interactive() }
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

/// A poll that interrupts the evaluation when Python has a pending signal, such as Ctrl-C, or when an output callback raised. It keeps the error to raise.
fn ctrl_c(caught: Arc<Mutex<Option<PyErr>>>) -> basedpl::Poll {
    Arc::new(move || {
        if caught.lock().unwrap().is_some() { return true; }
        let Err(e) = Python::attach(|py| py.check_signals()) else { return false };
        *caught.lock().unwrap() = Some(e);
        true
    })
}

/// An output sink that keeps each event in `shown` and calls `show` with it. An error that `show` raises goes to `caught`, which stops the evaluation through `ctrl_c`.
fn stream(show: Py<PyAny>, shown: Arc<Mutex<Vec<basedpl::Output>>>, caught: Arc<Mutex<Option<PyErr>>>) -> basedpl::OutputSink {
    Arc::new(move |event| {
        shown.lock().unwrap().push(event.clone());
        let called = Python::attach(|py| python(py, &basedpl::protocol::output(event)).and_then(|e| show.call1(py, (e,))).map(drop));
        if let Err(e) = called { caught.lock().unwrap().get_or_insert(e); }
    })
}

/// Python's standard input. Each line comes from `input()`, which shows a notebook's input box under IPython. A Ctrl-C while `input()` waits goes to `caught`, as a signal does in `ctrl_c`.
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
        Python::attach(|py| -> PyResult<String> { py.import("sys")?.getattr("stdin")?.call_method0("read")?.extract() }).map(String::into_bytes)
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
    let value = if let Some(inner) = result.value { Some(Py::new(py, PyArray::from(inner))?.into_any()) }
    else if let Some(inner) = result.function { Some(Py::new(py, PyFunction { inner })?.into_any()) }
    else if let Some(inner) = result.operator { Some(Py::new(py, PyOperator { inner })?.into_any()) }
    else { None };
    let d = PyDict::new(py);
    d.set_item("value", value)?;
    d.set_item("output", output(py, &result.output)?)?;
    d.set_item("error", result.error.as_ref().map(|e| error(py, e)).transpose()?)?;
    Ok(d.unbind())
}

fn output(py: Python<'_>, events: &[basedpl::Output]) -> PyResult<Py<PyAny>> { python(py, &events.iter().map(basedpl::protocol::output).collect()) }

/// BPL source for a value captured from another interpreter, given as JSON.
#[pyfunction]
fn _captured_literal(value: &str) -> PyResult<String> {
    let value = serde_json::from_str(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    basedpl::reference::expected_array(&value).map(|v| v.literal()).ok_or_else(|| PyValueError::new_err("invalid captured array"))
}

/// The cases of `text`, in the format of a `.bpl` reference file, as `reference::cases` reads them.
#[pyfunction]
fn _reference_cases(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    python(py, &serde_json::Value::Array(basedpl::reference::cases(text).map_err(PyValueError::new_err)?))
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
    let span = |s: &basedpl::Span| {
        let chars = char_offsets(&s.source.text);
        serde_json::json!([chars[s.range.start], chars[s.range.end]])
    };
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
    m.add_function(wrap_pyfunction!(_captured_literal, m)?)?;
    m.add_function(wrap_pyfunction!(_reference_cases, m)?)?;
    m.add_function(wrap_pyfunction!(_parse, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("symbols", python(m.py(), &basedpl::symbols::rows())?)?;
    m.add("_system_functions", basedpl::system::names().collect::<Vec<_>>())?;
    m.add("_superscripts", basedpl::superscripts())?;
    m.add("_subscripts", basedpl::subscripts())?;
    Ok(())
}
