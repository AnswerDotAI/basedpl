//! Native reference work queue, shared by the test binary and `check_file`.
use super::{File, checked, crashed, incomplete, load, reply, with_skips};
use serde_json::{Value, json};
use std::{collections::{HashMap, VecDeque}, fs, io::{BufRead, BufReader, Read, Seek, Write}, path::Path,
    process::{self, Command, Stdio}, sync::{Mutex, mpsc}, thread, time::{Duration, Instant}};

/// A worker must answer one item within this time, even when its evaluation does not poll cancellation.
const STALL: Duration = Duration::from_secs(10);

struct Child {
    process: process::Child, input: mpsc::Sender<String>, output: mpsc::Receiver<Result<Value, String>>, errors: fs::File,
}

impl Child {
    fn spawn(mut command: Command, threads: &str) -> Result<Self, String> {
        let errors = tempfile::tempfile().map_err(|e| e.to_string())?;
        let stderr = errors.try_clone().map_err(|e| e.to_string())?;
        let mut process = command.env("RAYON_NUM_THREADS", threads).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(stderr)
            .spawn().map_err(|e| e.to_string())?;
        let (mut stdin, stdout) = (process.stdin.take().unwrap(), process.stdout.take().unwrap());
        let (input, requests) = mpsc::channel::<String>();
        let (send, output) = mpsc::channel();
        thread::spawn(move || {
            let mut lines = BufReader::new(stdout).lines();
            for request in requests {
                let result = (|| {
                    writeln!(stdin, "{request}").map_err(|e| e.to_string())?;
                    // A libtest child also writes its harness output.
                    let line = lines.by_ref().map_while(Result::ok).find(|line| line.starts_with('{'))
                        .ok_or("the worker stopped before replying")?;
                    serde_json::from_str::<Value>(&line).map_err(|e| e.to_string()).and_then(|mut message|
                        message.get_mut("result").map(Value::take).ok_or_else(|| "worker reply has no result".into()))
                })();
                if send.send(result).is_err() { break; }
            }
            // EOF on input lets the child finish; keep reading until it closes its output.
            drop(stdin);
            for _ in lines.map_while(Result::ok) {}
        });
        Ok(Self { process, input, output, errors })
    }

    fn request(&mut self, request: &Value, deadline: Option<Instant>) -> Result<Value, String> {
        let wait = deadline.map_or(STALL, |end| STALL.min(end.saturating_duration_since(Instant::now())));
        if wait.is_zero() { return Err("the check's deadline passed".into()); }
        self.input.send(request.to_string()).map_err(|e| e.to_string())?;
        let result = self.output.recv_timeout(wait).map_err(|e| match e {
            mpsc::RecvTimeoutError::Timeout => format!("the item ran past {} s", STALL.as_secs()),
            mpsc::RecvTimeoutError::Disconnected => "the worker stopped before replying".into(),
        })??;
        if let Some(error) = result["error"]["message"].as_str() { return Err(error.into()); }
        Ok(result)
    }

    fn end(mut self, kill: bool) -> Result<(), String> {
        drop(self.input);
        if kill { let _ = self.process.kill(); }
        let status = self.process.wait().map_err(|e| e.to_string())?;
        if status.success() { return Ok(()); }
        let mut errors = Vec::new();
        self.errors.seek(std::io::SeekFrom::Start(0)).and_then(|_| self.errors.read_to_end(&mut errors)).map_err(|e| e.to_string())?;
        let errors = String::from_utf8_lossy(&errors);
        let tail: Vec<_> = errors.lines().rev().take(20).collect();
        Err(format!("{status}: {}", tail.into_iter().rev().collect::<Vec<_>>().join("\n")))
    }
}

/// A worker request, its timing key and the source location used in diagnostics.
pub struct Job { pub request: Value, pub key: String, pub place: String }

impl Job {
    pub fn case(file: &File, case: &Value, request: Value) -> Self {
        let name = file.name.trim_end_matches(".bpl");
        let line = case["line"].to_string();
        let id = case["id"].as_str().filter(|id| !id.is_empty()).unwrap_or(&line);
        Self { request, key: format!("{name}.bpl:{id}"), place: format!("{name}.bpl:{line}") }
    }
}

// A reference request with setup reports a summary; a case request reports the case itself.
fn failed_case(result: &Value) -> &Value { result["failures"].as_array().and_then(|f| f.first()).unwrap_or(result) }
type Served = (usize, bool, f64, Result<Value, String>);

fn serve(jobs: &[Job], queue: &Mutex<VecDeque<(usize, bool)>>, command: &impl Fn() -> Command, threads: &str,
    deadline: Option<Instant>) -> (Vec<Served>, Vec<String>) {
    let (mut served, mut errors, mut child) = (Vec::new(), Vec::new(), None::<Child>);
    loop {
        if deadline.is_some_and(|end| Instant::now() >= end) { break; }
        let next = queue.lock().unwrap().pop_front();
        let Some((i, rerun)) = next else { break };
        let mut fresh = None;
        let worker = if rerun { &mut fresh } else { &mut child };
        let started = Instant::now();
        let mut result = (|| {
            if worker.is_none() { *worker = Some(Child::spawn(command(), threads)?); }
            worker.as_mut().unwrap().request(&jobs[i].request, deadline)
        })();
        let seconds = started.elapsed().as_secs_f64();
        if result.is_err() || rerun {
            if let Some(Err(reason)) = worker.take().map(|worker| worker.end(result.is_err())) {
                if let Err(message) = &mut result { *message = format!("{message} ({reason})"); }
                else { errors.push(format!("{}: its worker stopped after replying ({reason})", jobs[i].place)); }
            }
        }
        if result.is_err() && deadline.is_some_and(|end| Instant::now() >= end) { break; }
        if !rerun && result.as_ref().is_ok_and(|r| failed_case(r)["status"] == "timeout") { queue.lock().unwrap().push_back((i, true)); }
        served.push((i, rerun, seconds, result));
    }
    if let Some(Err(reason)) = child.map(|worker| worker.end(false)) { errors.push(format!("a worker stopped after its last item ({reason})")); }
    (served, errors)
}

/// Pull from one queue, longest first where `costs` records a time. Each worker keeps its process except after a crash or for a fresh deadline rerun.
/// Return each job's reply or crash in input order, `None` where the check's deadline left it unfinished, and errors after replies.
pub fn run(jobs: &[Job], command: impl Fn() -> Command + Sync, costs: Option<&Path>, deadline: Option<Instant>)
    -> (Vec<Option<Result<Value, String>>>, Vec<String>) {
    if jobs.is_empty() { return (Vec::new(), Vec::new()); }
    let saved: HashMap<String, f64> = costs.and_then(|p| fs::read_to_string(p).ok()).and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    let mut order: Vec<_> = (0..jobs.len()).collect();
    let seconds = |i: usize| saved.get(&jobs[i].key).copied().unwrap_or(f64::INFINITY);
    order.sort_by(|&a, &b| seconds(b).total_cmp(&seconds(a)));
    let queue = Mutex::new(order.into_iter().map(|i| (i, false)).collect());
    let cores = thread::available_parallelism().map_or(1, |n| n.get());
    let children = cores.min(jobs.len());
    let threads = (cores / children).to_string();
    let served = thread::scope(|scope| {
        let workers: Vec<_> = (0..children).map(|_| scope.spawn(|| serve(jobs, &queue, &command, &threads, deadline))).collect();
        workers.into_iter().map(|worker| worker.join().unwrap()).collect::<Vec<_>>()
    });
    let (mut results, mut reruns, mut times, mut errors) = (vec![None; jobs.len()], Vec::new(), Vec::new(), Vec::new());
    for (served, failures) in served {
        errors.extend(failures);
        for (i, rerun, seconds, result) in served {
            times.push((i, rerun, seconds));
            if rerun { reruns.push((i, seconds, result)); } else { results[i] = Some((seconds, result)); }
        }
    }
    if let Some(path) = costs {
        let saved: HashMap<_, _> = times.iter().filter(|t| !t.1).map(|&(i, _, seconds)| (&jobs[i].key, seconds)).collect();
        let _ = fs::write(path, json!(saved).to_string());
    }
    for (i, seconds, result) in reruns {
        let (missed, first) = results[i].replace((seconds, result)).expect("a rerun follows the first attempt");
        let first = failed_case(first.as_ref().unwrap());
        let again = if results[i].as_ref().unwrap().1.as_ref().is_ok_and(|r| r["failures"].as_array().map_or_else(|| !super::fails(r), Vec::is_empty)) { "passed" } else { "failed" };
        eprintln!("{}: missed its deadline in {}, {missed:.2} s; a rerun {again}, {seconds:.2} s", jobs[i].place, first["mode"]);
    }
    if let Ok(slowest) = std::env::var("BASEDPL_TIMES") {
        times.sort_by(|a, b| b.2.total_cmp(&a.2));
        for (i, _, seconds) in times.into_iter().take(slowest.parse().expect("BASEDPL_TIMES holds a count")) { eprintln!("{seconds:6.3} s  {}", jobs[i].place); }
    }
    (results.into_iter().map(|r| r.map(|(_, result)| result)).collect(), errors)
}

/// The `check` request runs selected cases against a resolved worker executable, which may be a retained build.
pub fn check_file(spec: &Value, timeout: Option<Duration>) -> Result<Value, String> {
    let deadline = timeout.map(|time| Instant::now() + time);
    let path = Path::new(spec["path"].as_str().ok_or("check needs a path")?);
    let child = spec["child"].as_str().ok_or("check needs a worker executable")?;
    let ids = match spec.get("ids") { None | Some(Value::Null) => None, Some(ids) => Some(ids.as_array().ok_or("ids must be a list")?) };
    let mut files: Vec<_> = load(path)?.into_iter().map(with_skips).collect::<Result<_, _>>()?;
    for file in &mut files { for case in &mut file.cases { case["file"] = json!(file.name); } }
    let matches = |id: &Value, case: &Value| *id == case["id"] || *id == case["line"];
    let cases = || files.iter().flat_map(|f| f.cases.iter().map(move |c| (f, c)));
    let unknown: Vec<_> = ids.into_iter().flatten().filter(|id| !cases().any(|(_, c)| matches(id, c))).map(Value::to_string).collect();
    if !unknown.is_empty() { return Err(format!("{} match no case. Each id is a case's id or its header's line number.", unknown.join(", "))); }
    let selected: Vec<_> = cases().filter(|(_, c)| checked(c) && ids.is_none_or(|ids| ids.iter().any(|id| matches(id, c)))).collect();
    if selected.is_empty() { return Err(format!("{} holds no case to check", path.display())); }
    let jobs: Vec<_> = selected.iter().map(|(f, c)| Job::case(f, c, if f.setup.is_empty() { json!({"case": c}) }
        else { json!({"reference": {"name": f.name, "text": f.text, "ids": [c["line"]]}}) })).collect();
    let (results, errors) = run(&jobs, || { let mut command = Command::new(child); command.arg("--worker"); command }, None, deadline);
    if !errors.is_empty() { return Err(errors.join("\n")); }
    let no_case = "the worker reported no case, so its result can't be trusted";
    let (mut records, mut summaries) = (Vec::new(), Vec::new());
    for ((file, case), result) in selected.into_iter().zip(results) {
        match result {
            None => records.push(incomplete(case, "unchecked", "the check's deadline passed before this case returned a result")),
            Some(Err(reason)) => records.push(crashed(case, &reason)),
            Some(Ok(result)) if file.setup.is_empty() => {
                if result["modes"].as_array().is_none_or(Vec::is_empty) { return Err(no_case.into()); }
                records.push(result);
            }
            Some(Ok(mut summary)) => {
                for field in ["failures", "obligations"] {
                    for result in summary[field].as_array_mut().unwrap() { result["file"] = json!(file.name); }
                }
                summaries.push(summary);
            }
        }
    }
    let mut report = reply(records);
    for summary in summaries {
        for field in ["failures", "obligations"] { report[field].as_array_mut().unwrap().extend(summary[field].as_array().unwrap().iter().cloned()); }
        let totals = report["accounting"].as_object_mut().unwrap();
        for (mode, counts) in summary["accounting"].as_object().filter(|counts| !counts.is_empty()).ok_or(no_case)? {
            let total = totals.entry(mode).or_insert_with(|| json!({}));
            for (key, n) in counts.as_object().unwrap() { total[key] = json!(total[key].as_u64().unwrap_or(0) + n.as_u64().unwrap()); }
        }
    }
    Ok(report)
}
