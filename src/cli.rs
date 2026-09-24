use crate::{parse, Evaluation, ParseStatus, Session, Source};
use rustyline::error::ReadlineError;
use std::io::{self, IsTerminal, Read, Write};

const USAGE: &str = "Usage: bapl [-e EXPR | FILE | - | --json | --worker | --kernel -f CONNECTION_FILE]\n\nNo arguments: persistent APL REPL (Ctrl-D to exit, Ctrl-C to cancel input).\nType `name then Tab or a non-letter to enter a symbol, e.g. `iota5 becomes ⍳5.\nUse - to execute all of stdin as one source; --json for a JSON-lines session.\nUse --worker for structured requests with deadlines and interruption.\nUse --kernel -f CONNECTION_FILE to run a Jupyter kernel.\n";

fn show(result: Evaluation, out: &mut impl Write, err: &mut impl Write) -> io::Result<bool> {
    for line in result.output { writeln!(out, "{}", line.text())?; }
    if let Some(e) = result.error {
        writeln!(err, "{e}")?;
        return Ok(false);
    }
    Ok(true)
}

fn expression(code: &str, name: &str, out: &mut impl Write, err: &mut impl Write) -> io::Result<i32> {
    let result = Session::new().eval_source(Source::new(name, code));
    Ok(if show(result, out, err)? { 0 } else { 1 })
}

fn repl(out: &mut impl Write, err: &mut impl Write, interactive: bool) -> io::Result<i32> {
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
            failed |= !show(session.eval(&code), out, err)?;
            code.clear();
            out.flush()?;
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
                failed |= !show(session.eval_parsed(&parsed), out, err)?;
            }
        }
        out.flush()?;
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
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    let result = match args {
        [] => repl(&mut out, &mut err, stdin.is_terminal() && stdout.is_terminal()),
        [flag] if flag == "--json" => crate::protocol::run(&mut stdin.lock(), &mut out).map(|_| 0),
        [flag] if flag == "--worker" => crate::worker::run(&mut out).map(|_| 0),
        [flag, code] if flag == "-e" => expression(code, "<expression>", &mut out, &mut err),
        [flag] if flag == "--help" || flag == "-h" => write!(out, "{USAGE}").map(|_| 0),
        [flag] if flag == "--version" => writeln!(out, "basedpl {}", env!("CARGO_PKG_VERSION")).map(|_| 0),
        [file] if file == "-" => {
            let mut code = String::new();
            stdin.lock().read_to_string(&mut code).and_then(|_| expression(&code, "<stdin>", &mut out, &mut err))
        }
        [file] if !file.starts_with('-') => std::fs::read_to_string(file).and_then(|code| expression(&code, file, &mut out, &mut err)),
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
