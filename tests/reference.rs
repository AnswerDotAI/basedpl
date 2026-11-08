use basedpl::{InterruptHandle, Session};
use basedpl_testkit::reference;
use serde_json::{Value, json};
use std::{collections::{HashMap, VecDeque}, io::{BufRead, Read, Seek, Write}, process::Stdio, sync::Mutex};

/// The memory guard, as `bpl` registers it, so that `reference::failures` can check that each case gives back the large allocations it makes.
#[global_allocator]
static GUARD: rustymath::memory::Guard = rustymath::memory::Guard::new();

fn cases(text: &str) -> Vec<Value> { reference::file("", text).unwrap().cases }

#[test]
fn reference_format_and_comparison() {
    let file = json!({"code":r#"testpath •nput "héllo" ⋄ •nget testpath"#, "expected_code":r#""héllo""#});
    for _ in 0..2 { assert_eq!(reference::check(&file, &InterruptHandle::default())["status"], "pass"); }
    let compact = cases(&format!("⍝ —\n{}", r#""a""⍝b" '⍝'   ⍝ "a""⍝b" '⍝'"#));
    assert_eq!(compact[0]["code"], r#""a""⍝b" '⍝'"#);
    assert_eq!(reference::check(&compact[0], &InterruptHandle::default())["status"], "pass");
    for source in ["⍝ —\n1\n1", "⍝ —\n{\n⍵\n}1\n⍝ =>\n1", "⍝ —\n⎕←1\n1\n⍝ ⎕: 1"] {
        for ending in ["", "\n", "\n\n"] { assert_eq!(cases(&format!("{source}{ending}")), cases(&format!("{source}\n\n"))); }
    }
    assert_eq!(cases("⍝ —\n{\n⍵\n}1   ⍝ 1"), cases("⍝ —\n{\n⍵\n}1\n⍝ =>\n1"));
    let parsed_output = cases("⍝⍝ Output\n\n⍝ —\n⎕←9 ⋄ ⎕←2 ⋄ 7\n7\n⍝ ⎕: 9\\n2\n\n⍝ —\n3\n3\n⍝ ⎕:\n\n⍝ —\n⍎¨[⎕ ⎕]\n3 4\n⍝ input: 3\\n4\n⍝ ⎕:\n\n");
    assert_eq!(parsed_output[0]["section"], "Output");
    assert_eq!(parsed_output[0]["expected_output"], "9\n2");
    assert_eq!(parsed_output[2]["input"], "3\n4");
    for case in parsed_output { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    let mut output_error = json!({"code":"⎕←9 ⋄ 1÷'a'", "expected_error":"DOMAIN ERROR", "expected_output":"9"});
    assert_eq!(reference::check(&output_error, &InterruptHandle::default())["status"], "pass");
    output_error["expected_output"] = json!("2");
    assert_eq!(reference::check(&output_error, &InterruptHandle::default())["status"], "mismatch");
    let exactness = json!({"code":"1", "expected_code":"1ₓ"});
    assert_eq!(reference::check(&exactness, &InterruptHandle::default())["status"], "mismatch");
    let kinds =
        reference::check(&json!({"code":r#"{⍵?1;$t}"batch"⊃•prefs ⍬"#, "expected":{"shape":[], "data":[1], "prototype":0}}), &InterruptHandle::default());
    assert_eq!(
        (kinds["status"].as_str(), kinds["message"].as_str()),
        (Some("mismatch"), Some("storage kind: bool with the batch pref off, float64 with the batch pref on"))
    );
    let captured = json!({"code":"1ₓ", "expected":{"shape":[], "data":[1], "prototype":0}});
    assert_eq!(reference::check(&captured, &InterruptHandle::default())["status"], "pass");
    let parsed = cases("⍝  — [rtol=1e-14 atol=1e-15]\nf←{\n\n⍵+1\n}\nf 2\n⍝ =>\n3\n\n⍝  —\n\"unfinished\n⍝ error: SYNTAX ERROR\n\n");
    assert_eq!(parsed[0]["code"], "f←{\n\n⍵+1\n}\nf 2");
    assert_eq!(parsed[1]["line"], 10);
    for case in parsed { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    let text =
        "⍝⍝ setup\nv←1 2 3\n\n⍝⍝ copies •prefs [\"float\":32]\n⍝ mutate — expectation gets its own setup\nv←v+1 ⋄ v\n⍝ =>\nv+1\n⍝⍝: 2 3 4\n\n-v\n⍝⍝: 1 1 1\n";
    let file = reference::file("sample.bpl", text).unwrap();
    assert_eq!(file.cases.len(), 2);
    assert_eq!(file.cases[0]["prefs"], "•prefs [\"float\":32]");
    let span = file.cases[0]["budget_span"].as_array().unwrap();
    assert_eq!(&text[span[0].as_u64().unwrap() as usize..span[1].as_u64().unwrap() as usize], "⍝⍝: 2 3 4");
    assert!(reference::failures(file.cases.iter().filter(|c| reference::checked(c)), &file.setup, false, &InterruptHandle::default()).is_empty());
    reference::check_once(&file.cases[0], &file.setup, &InterruptHandle::default()).unwrap();
    let text = "⍝ route-format —\n⍝⍝⍝ route vector kernel\n⍝⍝⍝ gap stopped \"a read of a traced value\": guard on differentiated data\n{0<+/⍵?+/⍵×⍵;0}∂1 2";
    for ending in ["   ⍝ 2 4", "\n2 4"] {
        let mut routed = cases(&format!("{text}{ending}")).remove(0);
        assert_eq!(routed["route"], json!(["vector kernel"]));
        assert_eq!(routed["gap"][0], json!(["stopped", "a read of a traced value"]));
        let checked = reference::check(&routed, &InterruptHandle::default());
        assert!(checked["status"] == "pass" && !reference::obliged(&checked), "{checked}");
        routed["route"] = json!(["missing route"]);
        let checked = reference::check(&routed, &InterruptHandle::default());
        assert!(checked["status"] == "pass" && reference::obliged(&checked), "a route violation is an obligation: {checked}");
    }
    for (code, expect, status) in [
        ("1E¯10", "0", "mismatch"), ("1E¯16", "0", "pass"), ("1+1E¯15", "1", "pass"), ("a←3", "a", "invalid"), ("1", "{}0", "mismatch"),
        ("{}0", "{}0", "pass"), ("+", "{}0", "mismatch"), ("⍬", r#""""#, "mismatch"), (",1", "1", "mismatch"), ("0⍴⊂1 2", "0⍴⊂1", "mismatch"),
        (r#""aa":1"#, r#""bb":1"#, "mismatch"), (r#""aa":1"#, ",1", "mismatch"), (r#"["row":1]⍴1"#, r#"["col":1]⍴1"#, "mismatch"),
        ("3", "÷'a'", "invalid"), ("32•float 1+3×2*¯23", "32•float 1", "pass"), ("1+1E¯8", "32•float 1+1E¯8", "mismatch"),
    ] {
        let case = json!({"code":code, "expected_code":expect});
        assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], status, "{code} vs {expect}");
    }
    let mut case = json!({"code":"1E¯16", "expected_code":"0", "relative_tolerance":1e-14, "absolute_tolerance":0.0});
    assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "mismatch");
    case["absolute_tolerance"] = json!(1e-15);
    assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass");
    case["code"] = json!("1E¯10");
    assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "mismatch");
    let exact = json!({"code":"32•float 1+3×2*¯23", "expected_code":"32•float 1", "relative_tolerance":0.0});
    assert_eq!(reference::check(&exact, &InterruptHandle::default())["status"], "mismatch");
    for code in [r#"(•explain "1+1").value"#, r#"(•explain "1+1").fused"#] {
        let checked = reference::check(&cases(&format!("⍝ —\n{code}   ⍝ 3"))[0], &InterruptHandle::default());
        assert!(reference::fails(&checked) && !reference::obliged(&checked), "{checked}");
    }
}

/// The speed files, `tests/speed/*.bpl`.
fn speed() -> Vec<reference::File> { reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap() }

/// Every speed case has its macOS, Linux and wasm budgets: the native and wasm speed tests pass a case that lacks the budget they read, without checking it.
#[test]
fn every_speed_case_has_its_three_budgets() {
    let files = speed();
    let missing: Vec<_> = files.iter().flat_map(|f| &f.cases).filter(|c| c["budgets"].as_array().is_none_or(|b| b.len() != 3)).collect();
    assert!(missing.is_empty(), "each case needs a macOS, a Linux and a wasm budget: {missing:#?}");
}

/// The variable that makes a run of `reference_cases` a child. A child reads items from standard input, one a line, and checks each: a case's position in the suite's order, a timed speed case (`SPEED`), or a documentation page's path.
const CHILD: &str = "BASEDPL_CHILD";
/// The start of each line on which a child prints a case's position and result, as JSON.
const RESULT: &str = "reference result: ";
/// The start of the line on which a child prints a page's path and its failures, as a JSON list of messages.
const PAGE: &str = "reference page: ";
/// The start of an item that names a timed speed case, as `speed:FILE:LINE`, which a child checks at the fixture's small size (`reference::speed_route`).
const SPEED: &str = "speed:";
/// The variable that makes `reference_cases` print its slowest items: how many to print, each with the seconds that all its runs took together.
const TIMES: &str = "BASEDPL_TIMES";
/// The start of the line on which a child prints that it finished an item: the item, its seconds, then where it is.
const TIME: &str = "reference time: ";
/// The variable that names a file where `reference_cases` writes every case's result, as a JSON list in the suite's order, for a results record.
const RESULTS: &str = "BASEDPL_RESULTS";
/// The file where `reference_cases` keeps the seconds that each item took in its last run, which order the next run's items.
const COSTS: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/reference-times.json");
/// The longest that a child may check one item. A child whose item runs longer exits, so a hang gets a crash result that names its item before an outer watchdog, such as nextest's 30 s limit, stops the whole run.
const STALL: std::time::Duration = std::time::Duration::from_secs(10);

/// The documentation pages whose examples `reference_cases` checks: the `.qmd` pages and `.bpl` gallery demos in `nbs`, `nbs/glyphs`, `nbs/system` and `nbs/gallery`.
fn pages() -> Vec<String> {
    let mut pages = Vec::new();
    for dir in ["nbs", "nbs/glyphs", "nbs/system", "nbs/gallery"] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "qmd" || ext == "bpl") { pages.push(path.display().to_string()); }
        }
    }
    pages
}

/// The failures of the page at `path`. A `.qmd` page fails on each example that fails (`reference::page`) and on each link to a missing file. A gallery demo is a whole program, run after the gallery's settings `prefs`, as the gallery runs it, and fails on its first error. A demo that defines `frame` also draws its first frame.
fn page_failures(path: &str, prefs: &str) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    if path.ends_with(".bpl") {
        let session = Session::new();
        let error = [prefs, text.as_str(), r#"{3=•nc "frame"?frame 0;0}0"#].into_iter().find_map(|code| session.eval(code).error);
        return error.map(|e| format!("{path}: {e}")).into_iter().collect();
    }
    let dir = std::path::Path::new(path).parent().unwrap();
    let (mut failures, mut fenced) = (Vec::new(), false);
    for line in text.lines() {
        if line.starts_with("```") { fenced = !fenced; continue; }
        if fenced { continue; }
        for link in line.split("](").skip(1) {
            let target = link.split(')').next().unwrap().split('#').next().unwrap();
            if !target.is_empty() && !target.contains("://") && !dir.join(target).exists() { failures.push(format!("{path}: missing {target}")); }
        }
    }
    failures.extend(
        reference::page(&text, None, &InterruptHandle::default(), None).into_iter()
            .map(|f| format!("{path}:{}: {}\n{}", f["line"], f["code"].as_str().unwrap(), f["message"].as_str().unwrap())));
    failures
}

/// A child of `reference_cases`: its process, the input it reads items from, the output it prints on, and the file that holds what it writes to standard error.
struct Child {
    process: std::process::Child, input: std::process::ChildStdin, output: std::io::Lines<std::io::BufReader<std::process::ChildStdout>>, errors: std::fs::File
}

impl Child {
    fn spawn(exe: &std::path::Path, threads: &str) -> Self {
        let errors = tempfile::tempfile().unwrap();
        let mut process = std::process::Command::new(exe).args(["reference_cases", "--exact", "--nocapture"]).env(CHILD, "1").env("RAYON_NUM_THREADS", threads)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(errors.try_clone().unwrap()).spawn().unwrap();
        let (input, output) = (process.stdin.take().unwrap(), process.stdout.take().unwrap());
        Self { process, input, output: std::io::BufReader::new(output).lines(), errors }
    }

    /// Whether the child succeeded once its input closes, and why it ended: its exit status and the last 20 lines it wrote to standard error.
    fn end(mut self) -> (bool, String) {
        drop(self.input);
        let status = self.process.wait().unwrap();
        let mut errors = Vec::new();
        self.errors.seek(std::io::SeekFrom::Start(0)).and_then(|_| self.errors.read_to_end(&mut errors)).unwrap();
        let errors = String::from_utf8_lossy(&errors);
        let tail: Vec<&str> = errors.lines().rev().take(20).collect();
        (status.success(), format!("{status}: {}", tail.into_iter().rev().collect::<Vec<_>>().join("\n")))
    }
}

/// What one of `reference_cases`'s threads collected from its children.
#[derive(Default)]
struct Served {
    /// Each case's position, its result, whether a rerun gave it, and the case's seconds.
    results: Vec<(usize, Value, bool, f64)>,
    /// Each finished item, whether it was a rerun, its seconds and where it is, as `TIME` gives them.
    times: Vec<(String, bool, f64, String)>,
    /// Each page's failures, each page whose child stopped, and each child that stopped after its last item.
    failures: Vec<String>,
    /// Each timed speed case's route check (`reference::speed_route`).
    routes: Vec<Value>,
}

/// Checks the items of `queue` in children until none is left. A child gets an item once it prints that it finished its last one (`TIME`). A case that misses its deadline goes back on the queue once, as a rerun that a fresh child of its own checks. A child that stops before finishing gives its case a crash result (`reference::crashed`), and a fresh child takes the next item.
fn serve(exe: &std::path::Path, threads: &str, cases: &[(&reference::File, &Value)], queue: &Mutex<VecDeque<(String, bool)>>) -> Served {
    let (mut served, mut child, mut fresh) = (Served::default(), None::<Child>, None::<Child>);
    loop {
        // The lock is released before the item runs, because a case that misses its deadline locks the queue again to go back on it.
        let next = queue.lock().unwrap().pop_front();
        let Some((item, rerun)) = next else { break };
        let running = if rerun { fresh.insert(Child::spawn(exe, threads)) } else { child.get_or_insert_with(|| Child::spawn(exe, threads)) };
        // The read below reports a child that stopped before this write.
        let _ = writeln!(running.input, "{item}");
        let (mut result, mut finished) = (None::<Value>, false);
        for line in running.output.by_ref().map_while(Result::ok) {
            if let Some((_, r)) = line.strip_prefix(RESULT).and_then(|r| r.split_once(' ')) { result = Some(serde_json::from_str(r).unwrap()); }
            if let Some((_, f)) = line.strip_prefix(PAGE).and_then(|p| p.split_once(' ')) {
                served.failures.extend(serde_json::from_str::<Vec<String>>(f).unwrap());
            }
            let Some((seconds, place)) = line.strip_prefix(TIME).and_then(|t| t.split_once(' ')).and_then(|(_, t)| t.split_once(' ')) else { continue };
            let seconds: f64 = seconds.parse().unwrap();
            served.times.push((item.clone(), rerun, seconds, place.to_owned()));
            if let Ok(i) = item.parse::<usize>() {
                let result = result.take().expect("a child prints a case's result before its time");
                if !rerun && result["status"] == "timeout" { queue.lock().unwrap().push_back((item.clone(), true)); }
                served.results.push((i, result, rerun, seconds));
            } else if item.starts_with(SPEED) { served.routes.push(result.take().expect("a child prints a speed case's route check before its time")); }
            finished = true;
            break;
        }
        if finished && !rerun { continue; }
        let (ok, reason) = (if rerun { fresh.take() } else { child.take() }).unwrap().end();
        if finished { if !ok { served.failures.push(format!("a child stopped after its last item ({reason})")); } continue; }
        let message = format!("its child stopped ({reason})");
        match item.parse::<usize>() {
            Ok(i) => served.results.push((i, reference::crashed(cases[i].1, &message), rerun, 0.0)),
            Err(_) => served.failures.push(format!("{item}: {message}")),
        }
    }
    if let Some((false, reason)) = child.map(Child::end) { served.failures.push(format!("a child stopped after its last item ({reason})")); }
    served
}

/// The key under which `COSTS` keeps an item's seconds: a page's path, or a case's file with its id, or with its line where it has no id.
fn cost_key(item: &str, cases: &[(&reference::File, &Value)]) -> String {
    let Ok(i) = item.parse::<usize>() else { return item.to_owned() };
    let case = cases[i].1;
    let id = case["id"].as_str().filter(|id| !id.is_empty()).map_or_else(|| case["line"].to_string(), str::to_owned);
    format!("{}.bpl:{id}", case["file"].as_str().unwrap())
}

/// The suite (`reference::suite`), the route checks of the timed speed cases (`SPEED`) and the documentation pages (`pages`), checked in child processes, one per core. Each child checks one item at a time, and the next item goes to whichever child finishes first. Items go longest first, by the seconds that each took in the last run (`COSTS`), after the items that it didn't check. A case that misses its deadline reruns once (`serve`), and fails only if it misses again. Prints the counts by mode (`reference::accounting`) and how many cases hold obligations, and fails on each case that fails a run (`reference::fails`), each page that fails (`page_failures`) and each child that stops.
#[test]
fn reference_cases() {
    let suite = reference::suite().unwrap();
    let cases: Vec<(&reference::File, &Value)> = suite.iter().flat_map(|file| file.cases.iter().map(move |case| (file, case))).collect();
    if std::env::var(CHILD).is_ok() {
        // Each child runs in a process of its own, where no other test allocates, so the guard's count belongs to its cases alone.
        rustymath::memory::set_checks(true);
        let prefs = std::fs::read_to_string("nbs/gallery/prefs.bpl").unwrap();
        let speed = speed();
        // The start of the item that the child is checking, which a watchdog reads: a child whose item runs past `STALL` exits, and the parent gives the item a crash result.
        let started = std::sync::Arc::new(Mutex::new(None::<std::time::Instant>));
        let watched = started.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                if watched.lock().unwrap().is_some_and(|start| start.elapsed() > STALL) {
                    eprintln!("the item ran past {} s", STALL.as_secs());
                    std::process::exit(1);
                }
            }
        });
        for item in std::io::stdin().lines() {
            let item = item.unwrap();
            let start = std::time::Instant::now();
            *started.lock().unwrap() = Some(start);
            let place = if let Some((name, line)) = item.strip_prefix(SPEED).and_then(|s| s.rsplit_once(':')) {
                let file = speed.iter().find(|f| f.name == name).unwrap();
                let case = file.cases.iter().find(|c| c["line"].to_string() == line).unwrap();
                println!("{RESULT}{item} {}", reference::speed_route(file, case, &InterruptHandle::default()));
                basedpl::settings::reset();
                format!("{name}:{line}")
            } else if let Ok(i) = item.parse::<usize>() {
                let (file, case) = cases[i];
                for result in reference::results([case], &file.setup, true, &InterruptHandle::default()) { println!("{RESULT}{i} {result}"); }
                format!("{}.bpl:{}", case["file"].as_str().unwrap(), case["line"])
            } else {
                println!("{PAGE}{item} {}", json!(page_failures(&item, &prefs)));
                basedpl::settings::reset();
                item.clone()
            };
            println!("{TIME}{item} {} {place}", start.elapsed().as_secs_f64());
            *started.lock().unwrap() = None;
        }
        return;
    }
    // Shared, so that no timing runs while the cases load the machine.
    let _lock = rustymath_timing::shared().unwrap();
    let count = cases.len();
    assert!(count > 0, "the suite has no cases");
    let costs: HashMap<String, f64> = std::fs::read_to_string(COSTS).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    let speed = speed();
    let speed_items =
        speed.iter().flat_map(|f| f.cases.iter().filter(|c| basedpl_testkit::timing::timed(c)).map(move |c| format!("{SPEED}{}:{}", f.name, c["line"])));
    let mut items: Vec<(f64, String)> = pages().into_iter().chain(speed_items).chain((0..count).map(|i| i.to_string()))
        .map(|item| (costs.get(&cost_key(&item, &cases)).copied().unwrap_or(f64::INFINITY), item)).collect();
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let children = cores.min(items.len());
    // Each child's native pool gets an equal share of the cores. A case's own thread preference still applies.
    let threads = (cores / children).to_string();
    let queue = Mutex::new(items.into_iter().map(|(_, item)| (item, false)).collect::<VecDeque<_>>());
    // macOS reads the directory that holds a process's executable as its bundle when a framework such as ImageIO first asks for the bundle's information, and `target/debug/deps` holds hundreds of thousands of files. The children run from a copy of this test in a directory of its own.
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("reference");
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let served: Vec<Served> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..children).map(|_| scope.spawn(|| serve(&exe, &threads, &cases, &queue))).collect();
        workers.into_iter().map(|worker| worker.join().unwrap()).collect()
    });
    let (mut results, mut reruns, mut times, mut others, mut routes) = (vec![None::<(Value, f64)>; count], Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for served in served {
        for (i, result, rerun, seconds) in served.results { if rerun { reruns.push((i, result, seconds)) } else { results[i] = Some((result, seconds)) } }
        times.extend(served.times);
        others.extend(served.failures);
        routes.extend(served.routes);
    }
    // An unwritable file costs the next run its order, not a result.
    let _ = std::fs::write(
        COSTS, json!(times.iter().filter(|t| !t.1).map(|(item, _, seconds, _)| (cost_key(item, &cases), *seconds)).collect::<HashMap<_, _>>()).to_string());
    for (i, result, seconds) in reruns {
        let (first, missed) = results[i].replace((result, seconds)).expect("a rerun follows its case's first result");
        let again = if reference::fails(&results[i].as_ref().unwrap().0) { "missed it again" } else { "passed" };
        eprintln!(
            "{}.bpl:{}: missed its deadline in {}, {missed:.2} s; a rerun {again}, {seconds:.2} s", cases[i].1["file"].as_str().unwrap(), cases[i].1["line"],
            first["mode"]
        );
    }
    if let Ok(slowest) = std::env::var(TIMES) {
        times.sort_by(|a, b| b.2.total_cmp(&a.2));
        for (_, _, seconds, place) in times.iter().take(slowest.parse().expect("BASEDPL_TIMES holds a count")) { eprintln!("{seconds:6.3} s  {place}"); }
    }
    let results: Vec<Value> = results.into_iter().map(|r| r.expect("each case has a result or a crash").0).collect();
    if let Ok(path) = std::env::var(RESULTS) { std::fs::write(&path, serde_json::to_string(&results).unwrap()).unwrap(); }
    // Route contracts and rustymath's route inventory (`reference::coverage`) are release-2 obligations, which fail no run. The test prints them, with the case that exercised each route and the gaps that route directives tolerate.
    let routed: Vec<Value> = results.iter().chain(&routes).cloned().collect();
    let (observed, missing) = reference::coverage(&routed);
    let place = |r: &Value| format!("{}.bpl:{}", r["file"].as_str().unwrap(), r["line"]);
    let obligations: Vec<String> =
        routed.iter().filter(|r| reference::obliged(r)).map(|r| format!("{}: {}", place(r), r["obligations"])).chain(missing).collect();
    let gaps: Vec<String> = routed.iter()
        .flat_map(|r| r["gap"].as_array().into_iter().flatten().map(move |g| format!("{}: {} {}", place(r), g[0].as_str().unwrap_or_default(), g[1])))
        .collect();
    for line in observed { eprintln!("{line}"); }
    eprintln!("gaps that route cases tolerate: {gaps:#?}");
    if !obligations.is_empty() { eprintln!("route and report contracts, release-2 obligations that fail no run: {obligations:#?}"); }
    eprintln!(
        "{count} reference cases; runs by mode (planned, checked, failed, not applicable): {}; {} release-2 obligations", reference::accounting(&results),
        obligations.len()
    );
    let mut wrong: Vec<String> = results.iter().filter(|r| reference::fails(r)).map(|f| {
        let mode = f["mode"].as_str().map(|mode| format!("{mode}: ")).unwrap_or_default();
        format!(
            "{}.bpl:{} [{}] {}: {mode}{} ({})", f["file"].as_str().unwrap(), f["line"], f["section"].as_str().unwrap(), f["id"].as_str().unwrap(), f["message"],
            f["status"]
        )
    }).collect();
    wrong.extend(others);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
