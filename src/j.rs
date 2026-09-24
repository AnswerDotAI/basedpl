//! J sessions through libj, J's engine library, loaded at run time, and a Jupyter kernel for J.
use kernmini::{ExecuteOutcome, ExecuteRequest, ExecutionContext, KernelInfo, LanguageError, LanguageEvent, LanguageSession, ThreadWorker};
use libloading::Library;
use serde_json::json;
use std::{
    cell::RefCell,
    collections::VecDeque,
    ffi::{c_char, c_int, c_void, CStr, CString},
    path::Path,
    ptr::null,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

type Jt = *mut c_void;

/// The libj calls an engine makes after starting, as declared in `jsource/jsrc/jlib.h`.
struct Api {
    run: unsafe extern "C" fn(Jt, *const c_char) -> c_int,
    free: unsafe extern "C" fn(Jt),
    get: unsafe extern "C" fn(Jt, *const c_char, *mut i64, *mut i64, *mut i64, *mut i64) -> c_int,
    set: unsafe extern "C" fn(Jt, *const c_char, *mut i64, *mut i64, *mut i64, *mut i64) -> c_int,
    interrupt: unsafe extern "C" fn(Jt),
}

/// The items of a J noun: characters, integers (booleans read as integers) or floats.
#[derive(pyo3::FromPyObject, pyo3::IntoPyObject)]
pub(crate) enum Data { Chars(String), Ints(Vec<i64>), Floats(Vec<f64>) }

/// Output and input for the `JDo` call in progress on this thread. J calls the callbacks on the thread that called `JDo`.
#[derive(Default)]
struct Call {
    output: Vec<(c_int, String)>,
    input: VecDeque<String>,
    line: CString,
    exit: Option<i64>,
}

thread_local! { static CALL: RefCell<Call> = RefCell::default(); }

/// Type 1 is formatted output and type 2 an error display. Type 5 requests exit, with the exit code as the pointer value.
unsafe extern "C" fn output(_: Jt, kind: c_int, text: *const c_char) {
    CALL.with_borrow_mut(|call| match kind {
        5 => call.exit = Some(text as i64),
        _ if text.is_null() => {}
        _ => call.output.push((kind, unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned())),
    });
}

/// Supply the next line of a multi-line definition, or `)` to end it. J reads the line after this returns.
unsafe extern "C" fn input(_: Jt, _prompt: *const c_char) -> *const c_char {
    CALL.with_borrow_mut(|call| {
        call.line = CString::new(call.input.pop_front().unwrap_or_else(|| ")".into())).unwrap_or_default();
        call.line.as_ptr()
    })
}

/// The collected output, leaving out error displays unless `errors`.
fn output_text(errors: bool) -> String {
    CALL.with_borrow_mut(|call| call.output.drain(..).filter(|(kind, _)| errors || *kind != 2).map(|(_, text)| text).collect())
}

fn symbol<T: Copy>(library: &Library, name: &str) -> Result<T, String> { unsafe { library.get::<T>(name.as_bytes()) }.map(|s| *s).map_err(|e| e.to_string()) }

/// Copy `n` items of type `T` from address `at`. J can give a null address when `n` is 0.
unsafe fn items<T: Copy>(at: i64, n: usize) -> Vec<T> { if n == 0 { vec![] } else { unsafe { std::slice::from_raw_parts(at as *const T, n) }.to_vec() } }

/// A J engine with J's standard library loaded.
pub(crate) struct Engine {
    api: Api,
    jt: Jt,
    thread: std::thread::ThreadId,
    /// The exit code that `exit` requested. The engine runs no more lines after an exit request.
    pub(crate) exited: Option<i64>,
}

// Only the thread that created the engine runs J with it. Other threads can drop it. `Interrupter` shares it only for `JInterrupt`.
unsafe impl Send for Engine {}

impl Engine {
    /// Load libj from `lib`, start an engine, and run `profile.ijs` from the library's directory.
    pub(crate) fn new(lib: &Path) -> Result<Self, String> {
        // Engines can outlive any one owner of the library, so libj stays loaded.
        let library: &'static Library = Box::leak(Box::new(unsafe { Library::new(lib) }.map_err(|e| format!("{}: {e}", lib.display()))?));
        let init: unsafe extern "C" fn(*const c_char) -> Jt = symbol(library, "JInit2")?;
        let callbacks: unsafe extern "C" fn(Jt, *const *const c_void) = symbol(library, "JSM")?;
        let api = Api {
            run: symbol(library, "JDo")?,
            free: symbol(library, "JFree")?,
            get: symbol(library, "JGetM")?,
            set: symbol(library, "JSetM")?,
            interrupt: symbol(library, "JInterrupt")?,
        };
        let bin = lib.parent().ok_or("the libj path has no directory")?.display().to_string();
        let path = CString::new(bin.as_str()).map_err(|e| e.to_string())?;
        // J needs the installation path to find libgmp. Never mix `JInit` with `JInit2` in one process: it can crash.
        let jt = unsafe { init(path.as_ptr()) };
        if jt.is_null() { return Err("J failed to start".into()); }
        // The slots are {output, wd, input, unused, options}. Option 3 (SMCON) identifies a console, as `jconsole` does.
        let slots: [*const c_void; 5] = [output as *const c_void, null(), input as *const c_void, null(), 3 as *const c_void];
        unsafe { callbacks(jt, slots.as_ptr()) };
        let mut engine = Engine { api, jt, thread: std::thread::current().id(), exited: None };
        // As `jefirst` in `jsource/jsrc/jeload.c` does: set BINPATH and ARGV, then run profile.ijs.
        engine.run(&format!("(3 : '0!:0 y')<BINPATH,'/profile.ijs'[ARGV_z_=:<'jconsole'[BINPATH_z_=:'{}'", bin.replace('\'', "''")))?;
        Ok(engine)
    }

    /// J sets its recursion limit from the stack of the thread that starts the engine. On another thread, deep recursion overflows the stack.
    fn check_thread(&self) -> Result<(), String> {
        if std::thread::current().id() == self.thread { Ok(()) } else { Err("a J session runs only on the thread that created it".into()) }
    }

    /// Run `code`, one or more lines of J, returning its output. A J error returns the output, including the error display.
    /// After an exit request, the remaining lines are skipped and the output leaves out J's error display from unwinding.
    pub(crate) fn run(&mut self, code: &str) -> Result<String, String> {
        self.check_thread()?;
        CALL.with_borrow_mut(|call| *call = Call { input: code.trim().lines().map(String::from).collect(), ..Call::default() });
        while self.exited.is_none() {
            let Some(line) = CALL.with_borrow_mut(|call| call.input.pop_front()) else { break };
            let line = CString::new(line).map_err(|_| "J code cannot contain NUL characters".to_string())?;
            let failed = unsafe { (self.api.run)(self.jt, line.as_ptr()) } != 0;
            self.exited = CALL.with_borrow_mut(|call| call.exit);
            if failed && self.exited.is_none() { return Err(output_text(true)); }
        }
        Ok(output_text(self.exited.is_none()))
    }

    /// The shape and items of noun `name`.
    pub(crate) fn get(&mut self, name: &str) -> Result<(Vec<i64>, Data), String> {
        self.check_thread()?;
        let c = CString::new(name).map_err(|e| e.to_string())?;
        let (mut kind, mut rank, mut shape, mut at) = (0, 0, 0, 0);
        if unsafe { (self.api.get)(self.jt, c.as_ptr(), &mut kind, &mut rank, &mut shape, &mut at) } != 0 { return Err(format!("no noun: {name}")); }
        let shape: Vec<i64> = unsafe { items(shape, rank as usize) };
        let n = shape.iter().product::<i64>() as usize;
        let data = match kind {
            1 => Data::Ints(unsafe { items::<u8>(at, n) }.into_iter().map(i64::from).collect()),
            2 => Data::Chars(String::from_utf8_lossy(&unsafe { items::<u8>(at, n) }).into_owned()),
            4 => Data::Ints(unsafe { items(at, n) }),
            8 => Data::Floats(unsafe { items(at, n) }),
            _ => return Err(format!("unsupported J type {kind} for: {name}")),
        };
        Ok((shape, data))
    }

    /// Assign `data`, with `shape`, to noun `name`. J copies the items. Character shapes count UTF-8 bytes.
    pub(crate) fn set(&mut self, name: &str, shape: &[i64], data: &Data) -> Result<(), String> {
        self.check_thread()?;
        let c = CString::new(name).map_err(|e| e.to_string())?;
        let (mut kind, mut at, len) = match data {
            Data::Chars(s) => (2, s.as_ptr() as i64, s.len()),
            Data::Ints(v) => (4, v.as_ptr() as i64, v.len()),
            Data::Floats(v) => (8, v.as_ptr() as i64, v.len()),
        };
        if shape.iter().product::<i64>() != len as i64 { return Err(format!("{name}: the shape does not match the number of items")); }
        let (mut rank, mut shape) = (shape.len() as i64, shape.as_ptr() as i64);
        // `JSetM` returns J's error flag, which a failed sentence or `JGetM` leaves set. An empty sentence clears it.
        unsafe { (self.api.run)(self.jt, c"".as_ptr()) };
        if unsafe { (self.api.set)(self.jt, c.as_ptr(), &mut kind, &mut rank, &mut shape, &mut at) } != 0 { return Err(format!("JSetM failed: {name}")); }
        Ok(())
    }

    pub(crate) fn interrupter(&self) -> Interrupter { Interrupter { interrupt: self.api.interrupt, jt: self.jt as usize } }
}

impl Drop for Engine { fn drop(&mut self) { unsafe { (self.api.free)(self.jt) } } }

/// Stops the sentence an engine is running with an attention interrupt. J allows this from another thread.
/// Use it only while its engine exists.
#[derive(Clone, Copy)]
pub(crate) struct Interrupter { interrupt: unsafe extern "C" fn(Jt), jt: usize }

impl Interrupter { pub(crate) fn interrupt(&self) { unsafe { (self.interrupt)(self.jt as Jt) } } }

#[derive(Clone)]
struct JSession {
    worker: ThreadWorker<Engine>,
    interrupter: Interrupter,
    banner: String,
    count: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl LanguageSession for JSession {
    fn kernel_info(&self) -> anyhow::Result<KernelInfo> {
        Ok(KernelInfo {
            implementation: "basedpl-j".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            banner: format!("J {}", self.banner),
            language_info: json!({"name": "J", "version": self.banner.split('/').next().unwrap_or_default().trim_start_matches('j'), "mimetype": "text/x-j", "file_extension": ".ijs"}),
        })
    }

    fn execution_count(&self) -> u64 { self.count.load(Ordering::Acquire) }

    async fn execute(&self, request: ExecuteRequest, context: ExecutionContext) -> anyhow::Result<ExecuteOutcome> {
        let count = crate::kernel::next_count(&self.count, &request);
        let interrupter = self.interrupter;
        context.set_interrupt_handler(Arc::new(move || {
            interrupter.interrupt();
            Ok(())
        }))?;
        let silent = request.silent;
        let error = match self.worker.call(move |engine| engine.run(&request.code)).await? {
            Ok(text) => {
                if !silent && !text.is_empty() { context.emit(LanguageEvent::Stream { name: "stdout".into(), text }).await?; }
                None
            }
            Err(text) => {
                let text = text.trim_end().to_string();
                Some(LanguageError { ename: "JError".into(), evalue: text.clone(), traceback: vec![text] })
            }
        };
        Ok(ExecuteOutcome { execution_count: count, result: None, result_metadata: json!({}), error, user_expressions: json!({}), payload: json!([]) })
    }

    async fn shutdown(&self) -> anyhow::Result<()> { self.worker.shutdown().await }
}

/// Serve a J kernel on the connection in `file`, with libj from `lib`. `startup` runs first, and its errors go to stderr.
pub(crate) fn run_kernel(file: &str, lib: &Path, startup: Option<&str>) -> anyhow::Result<()> {
    let (lib, startup) = (lib.to_path_buf(), startup.map(String::from));
    crate::kernel::serve(file, async move {
        // J's recursion check assumes a larger stack than a thread's default. With the default, deep recursion crashes the kernel.
        let worker = ThreadWorker::start(std::thread::Builder::new().name("j".into()).stack_size(64 << 20), move || {
            let mut engine = Engine::new(&lib).map_err(anyhow::Error::msg)?;
            if let Some(Err(e)) = startup.map(|code| engine.run(&code)) { eprintln!("startup.ijs failed: {e}"); }
            Ok(engine)
        })
        .await?;
        let (interrupter, banner) = worker.call(|engine| (engine.interrupter(), engine.run("9!:14 ''").unwrap_or_default().trim().to_string())).await?;
        Ok(JSession { worker, interrupter, banner, count: Arc::default() })
    })
}
