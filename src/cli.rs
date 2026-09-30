use crate::{parse, EvalOptions, Evaluation, Input, InterruptHandle, Output, ParseStatus, Session, Source};
use rustyline::error::ReadlineError;
use std::{
    io::{self, IsTerminal, Read, Write},
    sync::{Arc, Mutex},
};

const USAGE: &str = "Usage: bpl [-e EXPR | FILE | - | --worker | --kernel -f CONNECTION_FILE]\n\nNo arguments: persistent BPL REPL (Ctrl-D to exit, Ctrl-C to cancel input).\nType `name then Tab or a non-letter to enter a symbol, e.g. `iota5 becomes ⍳5.\nUse - to execute all of stdin as one source.\nUse --worker for JSON-lines requests with deadlines and interruption.\nUse --kernel -f CONNECTION_FILE to run a Jupyter kernel.\n";

/// Writes each output to stdout as the evaluation produces it. A failed write interrupts the evaluation, and `finish` then gives the
/// write's error. Evaluations can also read standard input, when the program didn't come from it.
struct Printer { interrupt: InterruptHandle, failure: Arc<Mutex<Option<io::Error>>>, input: Option<Arc<dyn Input>> }
impl Printer {
    /// A printer whose evaluations read standard input when `input` is set.
    fn new(input: bool) -> Self {
        Self { interrupt: InterruptHandle::default(), failure: Arc::default(), input: input.then(|| Arc::new(StandardInput) as Arc<dyn Input>) }
    }
    fn options(&self) -> EvalOptions {
        let (interrupt, failure) = (self.interrupt.clone(), self.failure.clone());
        let sink = move |output: &Output| {
            let mut failure = failure.lock().unwrap();
            if failure.is_some() { return; }
            if let Err(e) = writeln!(io::stdout(), "{}", output.text()) {
                *failure = Some(e);
                interrupt.interrupt();
            }
        };
        EvalOptions { interrupt: self.interrupt.clone(), output: Some(Arc::new(sink)), input: self.input.clone(), ..EvalOptions::default() }
    }
    /// Writes the evaluation's error to `err`, and gives whether the evaluation succeeded. A failed write of its output is the error
    /// instead.
    fn finish(&self, result: Evaluation, err: &mut impl Write) -> io::Result<bool> {
        if let Some(e) = self.failure.lock().unwrap().take() { return Err(e); }
        let Some(e) = result.error else { return Ok(true) };
        writeln!(err, "{e}")?;
        Ok(false)
    }
}

/// Standard input, for a program whose source comes from elsewhere.
struct StandardInput;
impl Input for StandardInput {
    fn line(&self) -> io::Result<Option<String>> {
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { return Ok(None); }
        if line.ends_with('\n') { line.pop(); }
        if line.ends_with('\r') { line.pop(); }
        Ok(Some(line))
    }
    fn rest(&self) -> io::Result<Vec<u8>> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(data)
    }
}

/// Evaluates `code`. When `input` is set, the program can read standard input.
fn expression(code: &str, name: &str, input: bool, err: &mut impl Write) -> io::Result<i32> {
    let printer = Printer::new(input);
    let result = Session::new().eval_source(Source::new(name, code), printer.options());
    Ok(if printer.finish(result, err)? { 0 } else { 1 })
}

fn repl(err: &mut impl Write, interactive: bool) -> io::Result<i32> {
    // Piped input holds the program. At a terminal, `⎕` reads a line typed while the program runs.
    let printer = Printer::new(interactive);
    let mut editor = if interactive { Some(crate::editor::LineEditor::new().map_err(io::Error::other)?) } else { None };
    let mut session = if interactive { Session::interactive() } else { Session::new() };
    let mut code = String::new();
    let mut incomplete = None;
    let mut failed = false;
    loop {
        let read = if let Some(editor) = &mut editor {
            match editor.readline(if code.is_empty() { "      " } else { "    · " }) {
                Ok(line) => {
                    code.push_str(&line);
                    code.push('\n');
                    true
                }
                Err(ReadlineError::Interrupted) => {
                    code.clear();
                    incomplete = None;
                    continue;
                }
                // Rustyline restores terminal mode and writes a newline, including on Ctrl-D.
                Err(ReadlineError::Eof) => false,
                Err(e) => return Err(io::Error::other(e)),
            }
        } else { io::stdin().read_line(&mut code)? != 0 };
        if !read {
            if let Some(e) = incomplete {
                writeln!(err, "{e}")?;
                return Ok(1);
            }
            return Ok(i32::from(failed && !interactive));
        }
        if code.trim_start().starts_with(']') {
            failed |= !printer.finish(session.eval_with(&code, printer.options()), err)?;
            code.clear();
            continue;
        }
        match parse(Source::new("<repl>", code.as_str())) {
            ParseStatus::Incomplete(e) => {
                incomplete = Some(e);
                continue;
            }
            ParseStatus::Invalid(e) => {
                writeln!(err, "{e}")?;
                failed = true;
            }
            ParseStatus::Complete(parsed) => {
                failed |= !printer.finish(session.eval_parsed(&parsed, printer.options()), err)?;
            }
        }
        code.clear();
        incomplete = None;
    }
}

/// Both the native executable and the installed console script call this runner.
pub fn run(args: &[String]) -> i32 {
    if let [mode, flag, file] = args {
        if mode == "--kernel" && flag == "-f" {
            return match crate::kernel::run(file) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Kernel error: {e}");
                    1
                }
            };
        }
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut err = stderr.lock();
    let result = match args {
        [] => repl(&mut err, stdin.is_terminal() && stdout.is_terminal()),
        [flag] if flag == "--worker" => crate::worker::run(&mut stdout.lock()).map(|_| 0),
        [flag, code] if flag == "-e" => expression(code, "<expression>", true, &mut err),
        [flag] if flag == "--help" || flag == "-h" => write!(stdout.lock(), "{USAGE}").map(|_| 0),
        [flag] if flag == "--version" => writeln!(stdout.lock(), "basedpl {}", env!("CARGO_PKG_VERSION")).map(|_| 0),
        [file] if file == "-" => {
            let mut code = String::new();
            stdin.lock().read_to_string(&mut code).and_then(|_| expression(&code, "<stdin>", false, &mut err))
        }
        [file] if !file.starts_with('-') => std::fs::read_to_string(file).and_then(|code| expression(&code, file, true, &mut err)),
        _ => write!(err, "{USAGE}").map(|_| 2),
    };
    match result {
        Ok(code) => code,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(e) => {
            let _ = writeln!(err, "I/O error: {e}");
            1
        }
    }
}
