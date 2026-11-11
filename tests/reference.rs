use basedpl::{InterruptHandle, Session};
use basedpl_testkit::reference;
use serde_json::{Value, json};
use std::io::BufRead;

/// The memory guard, as `bpl` registers it, so that `reference::failures` can check that each case gives back the large allocations it makes.
#[global_allocator]
static GUARD: rustymath::memory::Guard = rustymath::memory::Guard::new();

fn cases(text: &str) -> Vec<Value> { reference::file("", text).unwrap().cases }

#[test]
fn reference_format_and_comparison() {
    let file = json!({"code":r#"testpath •nput "héllo" ◇ •nget testpath"#, "expected_code":r#""héllo""#});
    for _ in 0..2 { assert_eq!(reference::check(&file, &InterruptHandle::default())["status"], "pass"); }
    let compact = cases(&format!("⍝ —\n{}", r#""a""⍝b" '⍝'   ⍝ "a""⍝b" '⍝'"#));
    assert_eq!(compact[0]["code"], r#""a""⍝b" '⍝'"#);
    assert_eq!(reference::check(&compact[0], &InterruptHandle::default())["status"], "pass");
    for source in ["⍝ —\n1\n1", "⍝ —\n{\nω\n}1\n⍝ =>\n1", "⍝ —\n⎕←1\n1\n⍝ ⎕: 1"] {
        for ending in ["", "\n", "\n\n"] { assert_eq!(cases(&format!("{source}{ending}")), cases(&format!("{source}\n\n"))); }
    }
    assert_eq!(cases("⍝ —\n{\nω\n}1   ⍝ 1"), cases("⍝ —\n{\nω\n}1\n⍝ =>\n1"));
    let parsed_output = cases("⍝⍝ Output\n\n⍝ —\n⎕←9 ◇ ⎕←2 ◇ 7\n7\n⍝ ⎕: 9\\n2\n\n⍝ —\n3\n3\n⍝ ⎕:\n\n⍝ —\n⍎¨[⎕ ⎕]\n3 4\n⍝ input: 3\\n4\n⍝ ⎕:\n\n");
    assert_eq!(parsed_output[0]["section"], "Output");
    assert_eq!(parsed_output[0]["expected_output"], "9\n2");
    assert_eq!(parsed_output[2]["input"], "3\n4");
    for case in parsed_output { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    let mut output_error = json!({"code":"⎕←9 ◇ 1÷'a'", "expected_error":"DOMAIN ERROR", "expected_output":"9"});
    assert_eq!(reference::check(&output_error, &InterruptHandle::default())["status"], "pass");
    output_error["expected_output"] = json!("2");
    assert_eq!(reference::check(&output_error, &InterruptHandle::default())["status"], "mismatch");
    let exactness = json!({"code":"1", "expected_code":"1ₓ"});
    assert_eq!(reference::check(&exactness, &InterruptHandle::default())["status"], "mismatch");
    let kinds =
        reference::check(&json!({"code":r#"{ω?1;$t}"batch"⊃•prefs ⍬"#, "expected":{"shape":[], "data":[1], "prototype":0}}), &InterruptHandle::default());
    assert_eq!(
        (kinds["status"].as_str(), kinds["message"].as_str()),
        (Some("mismatch"), Some("storage kind: bool with the batch pref off, float64 with the batch pref on"))
    );
    let captured = json!({"code":"1ₓ", "expected":{"shape":[], "data":[1], "prototype":0}});
    assert_eq!(reference::check(&captured, &InterruptHandle::default())["status"], "pass");
    let parsed = cases("⍝  — [rtol=1e-14 atol=1e-15]\nf←{\n\nω+1\n}\nf 2\n⍝ =>\n3\n\n⍝  —\n\"unfinished\n⍝ error: SYNTAX ERROR\n\n");
    assert_eq!(parsed[0]["code"], "f←{\n\nω+1\n}\nf 2");
    assert_eq!(parsed[1]["line"], 10);
    for case in parsed { assert_eq!(reference::check(&case, &InterruptHandle::default())["status"], "pass"); }
    let text =
        "⍝⍝ setup\nv←1 2 3\n\n⍝⍝ copies •prefs [\"float\":32]\n⍝ mutate — expectation gets its own setup\nv←v+1 ◇ v\n⍝ =>\nv+1\n⍝⍝: 2 3 4\n\n-v\n⍝⍝: 1 1 1\n";
    let file = reference::file("sample.bpl", text).unwrap();
    assert_eq!(file.cases.len(), 2);
    assert_eq!(file.cases[0]["prefs"], "•prefs [\"float\":32]");
    let span = file.cases[0]["budget_span"].as_array().unwrap();
    assert_eq!(&text[span[0].as_u64().unwrap() as usize..span[1].as_u64().unwrap() as usize], "⍝⍝: 2 3 4");
    assert!(reference::failures(file.cases.iter().filter(|c| reference::checked(c)), &file.setup, false, &InterruptHandle::default()).is_empty());
    reference::check_once(&file.cases[0], &file.setup, &InterruptHandle::default()).unwrap();
    let text = "⍝ route-format —\n⍝⍝⍝ route vector kernel\n⍝⍝⍝ gap stopped \"a read of a traced value\": guard on differentiated data\n{0<+/ω?+/ω×ω;0}∂1 2";
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
        ("{}0", "{}0", "pass"), ("+", "{}0", "mismatch"), ("⍬", r#""""#, "mismatch"), (",1", "1", "mismatch"), ("0ρ⊂1 2", "0ρ⊂1", "mismatch"),
        (r#""aa":1"#, r#""bb":1"#, "mismatch"), (r#""aa":1"#, ",1", "mismatch"), (r#"["row":1]ρ1"#, r#"["col":1]ρ1"#, "mismatch"), ("3", "÷'a'", "invalid"),
        ("32•float 1+3×2*¯23", "32•float 1", "pass"), ("1+1E¯8", "32•float 1+1E¯8", "mismatch"),
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
    let aborted = reference::reply(vec![reference::crashed(&exact, "the child stopped")]);
    assert_eq!(aborted["failures"][0]["status"], "crash");
    assert!(aborted["accounting"].as_object().unwrap().values().all(|c| c["planned"] == 1 && c["checked"] == 0 && c["failed"] == 0));
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

/// The variable that makes a run of `reference_cases` a child. A child reads items from standard input, one a line, and checks each: a case's position in the suite's order, an annotated speed case (`SPEED`), or a documentation page's path.
const CHILD: &str = "BASEDPL_CHILD";
/// The variable that names a file where `reference_cases` writes every case's result, as a JSON list in the suite's order, for a results record.
const RESULTS: &str = "BASEDPL_RESULTS";
/// The file where `reference_cases` keeps the seconds that each item took in its last run, which order the next run's items.
const COSTS: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/reference-times.json");
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

/// Check the suite, annotated speed routes and documentation pages in the shared worker queue.
#[test]
fn reference_cases() {
    use reference::runner::{Job, run};
    if std::env::var_os(CHILD).is_some() {
        let prefs = std::fs::read_to_string("nbs/gallery/prefs.bpl").unwrap();
        let speed = std::cell::LazyCell::new(speed);
        println!(); // End any partial line from the test harness.
        for line in std::io::stdin().lock().lines() {
            let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
            let result = if let Some(path) = request["page_path"].as_str() {
                let failures = page_failures(path, &prefs);
                basedpl::settings::reset();
                json!(failures)
            } else if let Some(spec) = request.get("speed_route") {
                let file = speed.iter().find(|f| spec["file"].as_str() == Some(f.name.as_str())).unwrap();
                let case = file.cases.iter().find(|c| c["line"] == spec["line"]).unwrap();
                let result = reference::speed_route(file, case, &InterruptHandle::default());
                basedpl::settings::reset();
                result
            } else {
                reference::results([&request["case"]], request["setup"].as_str().unwrap(), true, &InterruptHandle::default()).remove(0)
            };
            println!("{}", json!({"result": result}));
        }
        return;
    }
    // Shared, so that the cases do not load the machine during an exclusive timing.
    let _lock = rustymath_timing::shared().unwrap();
    let suite = reference::suite().unwrap();
    let cases: Vec<_> = suite.iter().flat_map(|file| file.cases.iter().map(move |case| (file, case))).collect();
    let count = cases.len();
    assert!(count > 0, "the suite has no cases");
    let mut jobs: Vec<_> = pages().into_iter().map(|path| Job { request: json!({"page_path": path}), key: path.clone(), place: path }).collect();
    let page_count = jobs.len();
    for file in speed() {
        for case in file.cases.iter().filter(|c| reference::routed(c)) {
            let place = format!("{}:{}", file.name, case["line"]);
            jobs.push(Job { request: json!({"speed_route": {"file": file.name, "line": case["line"]}}), key: format!("speed:{place}"), place });
        }
    }
    let route_count = jobs.len() - page_count;
    jobs.extend(cases.iter().map(|(file, case)| Job::case(file, case, json!({"case": case, "setup": file.setup}))));
    // On macOS ImageIO reads the executable's directory as a bundle. Avoid the many entries in target/debug/deps.
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("reference");
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let (replies, mut others) = run(&jobs, || {
        let mut command = std::process::Command::new(&exe);
        command.args(["reference_cases", "--exact", "--nocapture"]).env(CHILD, "1");
        command
    }, Some(std::path::Path::new(COSTS)), None);
    let mut replies = replies.into_iter().map(|reply| reply.expect("each job has a result or a crash")).zip(&jobs);
    for (reply, job) in replies.by_ref().take(page_count) {
        match reply {
            Ok(result) => others.extend(serde_json::from_value::<Vec<String>>(result).unwrap()), Err(reason) => others.push(format!("{}: {reason}", job.place))
        }
    }
    let routes: Vec<_> =
        replies.by_ref().take(route_count).filter_map(|(reply, job)| reply.map_err(|reason| others.push(format!("{}: {reason}", job.place))).ok()).collect();
    let results: Vec<_> =
        cases.into_iter().zip(replies).map(|((_, case), (reply, _))| reply.unwrap_or_else(|reason| reference::crashed(case, &reason))).collect();
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
