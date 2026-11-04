use basedpl::{InterruptHandle, reference};
use serde_json::{Value, json};

/// The memory guard, as `bpl` registers it, so that `reference::failures` can check that each case gives back the large allocations it makes.
#[global_allocator]
static GUARD: rustymath::memory::Guard = rustymath::memory::Guard::new();

const SOURCES: &[(&str, &str)] = &[
    ("core", include_str!("reference/core.bpl")), ("graphics", include_str!("reference/graphics.bpl")), ("ngn", include_str!("reference/ngn.bpl")),
    ("april", include_str!("reference/april.bpl")), ("aplcart", include_str!("reference/aplcart.bpl")), ("dyalog", include_str!("reference/dyalog.bpl")),
    ("regex", include_str!("reference/regex.bpl")), ("distributions", include_str!("reference/distributions.bpl")), ("lib", include_str!("reference/lib.bpl")),
    ("bqn", include_str!("reference/bqn.bpl")), ("static", include_str!("reference/static.bpl")), ("static-int32", include_str!("reference/static-int32.bpl")),
];

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
        (Some("mismatch"), Some("storage kind: float64 with the batch pref on, bool with the batch pref off"))
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
        assert_eq!(reference::check(&routed, &InterruptHandle::default())["status"], "pass");
        routed["route"] = json!(["missing route"]);
        assert_eq!(reference::check(&routed, &InterruptHandle::default())["status"], "route");
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

/// The variable that makes a run of `reference_cases` a child that checks the cases at these positions in `chosen`'s order, as in `3 11 19`.
const SHARD: &str = "BASEDPL_SHARD";
/// The start of each line on which a child prints one of its failures, as JSON.
const FAILURE: &str = "reference failure: ";
/// The variable that makes `reference_cases` print its slowest cases: how many to print, each with the seconds that all its runs took together.
const TIMES: &str = "BASEDPL_TIMES";
/// The start of the line on which a child prints that it finished a case: the case's position, its seconds, then its file and line.
const TIME: &str = "reference time: ";

/// Selected expected-answer cases in source order, with their file name for child reports.
fn chosen() -> Vec<reference::File> {
    let mut chosen = Vec::new();
    for mut file in SOURCES.iter().map(|&(name, text)| reference::file(name, text).unwrap_or_else(|e| panic!("{name}.bpl: {e}"))).chain(speed()) {
        let name = file.name.trim_end_matches(".bpl");
        file.cases.retain(reference::checked);
        for case in &mut file.cases { case["file"] = json!(name); }
        if !file.cases.is_empty() { chosen.push(file); }
    }
    let mut selection = reference::Selection::from_env();
    selection.files(&mut chosen);
    selection.assert_selected();
    chosen
}

/// Every case, run once for each of `reference::check`'s runs: with the `batch` pref off, then on, under which maps first try one call of their operand on batches of their cells, then on with fusion's small sizes, under which every fusible call waits, a block stage splits even small arrays into blocks and every stage compiles its parts, then the same with no compiled parts where that run used one, then on with the code as the body of a dfn called twice. Each run but the last must meet the case's expectation, with values held in the same kinds of storage. In the last, the dfn's second call, which runs the statements its first call recorded, must give the first call's result. A process evaluates one case at a time, so the test runs a child process of itself for each core, each checking every n-th case (`SHARD`). A child prints its failures (`FAILURE`) and each case it finishes (`TIME`). When a child stops, as on an abort, the first case it didn't finish fails and new children run its later cases. The test reports how many cases it planned, checked and failed, and fails on any case a child skipped.
#[test]
fn reference_cases() {
    let chosen = chosen();
    let cases: Vec<(&reference::File, &Value)> = chosen.iter().flat_map(|file| file.cases.iter().map(move |case| (file, case))).collect();
    if let Ok(shard) = std::env::var(SHARD) {
        // Each child runs in a process of its own, where no other test allocates, so the guard's count belongs to its cases alone.
        rustymath::memory::set_checks(true);
        for i in shard.split(' ').map(|i| i.parse::<usize>().expect("BASEDPL_SHARD holds case positions")) {
            let (file, case) = cases[i];
            let start = std::time::Instant::now();
            for failure in reference::failures([case], &file.setup, false, &InterruptHandle::default()) { println!("{FAILURE}{failure}"); }
            println!("{TIME}{i} {} {}.bpl:{}", start.elapsed().as_secs_f64(), case["file"].as_str().unwrap(), case["line"]);
        }
        return;
    }
    // Shared, so that no timing runs while the cases load the machine.
    let _lock = rustymath::timing::shared().unwrap();
    let count = cases.len();
    assert!(count > 0, "no reference cases selected");
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    // macOS reads the directory that holds a process's executable as its bundle when a framework such as ImageIO first asks for the bundle's information, and `target/debug/deps` holds hundreds of thousands of files. The children run from a copy of this test in a directory of its own.
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("reference");
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let (mut checked, mut failures, mut times, mut skipped, mut stops) = (vec![false; count], Vec::<Value>::new(), Vec::new(), Vec::new(), Vec::new());
    let mut left: Vec<usize> = (0..count).collect();
    while !left.is_empty() {
        let shares = cores.min(left.len());
        let children: Vec<_> = (0..shares).map(|k| {
            let share: Vec<usize> = left.iter().copied().skip(k).step_by(shares).collect();
            let positions = share.iter().map(usize::to_string).collect::<Vec<_>>().join(" ");
            let child = std::process::Command::new(&exe).args(["reference_cases", "--exact", "--nocapture"]).env(SHARD, positions)
                .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
            // Every child's output is read as it comes, each on a thread of its own: a child whose pipe fills waits, inside the case it's timing, until its output is read.
            (share, std::thread::spawn(move || child.wait_with_output()))
        }).collect();
        left.clear();
        for (share, child) in children {
            let output = child.join().unwrap().unwrap();
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Some(failure) = line.strip_prefix(FAILURE) { failures.push(serde_json::from_str(failure).unwrap()); }
                if let Some((i, time)) = line.strip_prefix(TIME).and_then(|t| t.split_once(' ')) {
                    checked[i.parse::<usize>().unwrap()] = true;
                    times.push(time.to_string());
                }
            }
            let unfinished: Vec<usize> = share.into_iter().filter(|&i| !checked[i]).collect();
            if output.status.success() {
                skipped.extend(unfinished);
                continue;
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            let tail: Vec<&str> = stderr.lines().rev().take(20).collect();
            let reason = format!("{}: {}", output.status, tail.into_iter().rev().collect::<Vec<_>>().join("\n"));
            match unfinished.split_first() {
                Some((&i, later)) => {
                    let case = cases[i].1;
                    checked[i] = true;
                    failures.push(json!({"file":case["file"], "line":case["line"], "section":case["section"], "id":case["id"], "message":format!("its child stopped ({reason})"), "kind":"crash"}));
                    left.extend_from_slice(later);
                }
                None => stops.push(format!("a child stopped after its last case ({reason})")),
            }
        }
    }
    if let Ok(slowest) = std::env::var(TIMES) {
        let mut times: Vec<(f64, &str)> = times.iter().filter_map(|line| line.split_once(' ')).map(|(s, case)| (s.parse().unwrap(), case)).collect();
        times.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (seconds, case) in times.iter().take(slowest.parse().expect("BASEDPL_TIMES holds a count")) { eprintln!("{seconds:6.3} s  {case}"); }
    }
    failures.sort_by(|a, b| (a["file"].as_str(), a["line"].as_u64()).cmp(&(b["file"].as_str(), b["line"].as_u64())));
    let mut failed: Vec<_> = failures.iter().map(|f| (f["file"].as_str(), f["line"].as_u64())).collect();
    failed.dedup();
    let summary = format!("{count} reference cases planned, {} checked, {} failed", count - skipped.len(), failed.len());
    let mut wrong: Vec<String> = failures.iter().map(|f| {
        format!(
            "{}.bpl:{} [{}] {}: {} ({})", f["file"].as_str().unwrap(), f["line"], f["section"].as_str().unwrap(), f["id"].as_str().unwrap(), f["message"],
            f["kind"]
        )
    }).collect();
    wrong.extend(skipped.iter().map(|&i| format!("{}.bpl:{}: a child skipped it", cases[i].1["file"].as_str().unwrap(), cases[i].1["line"])));
    wrong.extend(stops);
    assert!(wrong.is_empty(), "{summary}:\n{}", wrong.join("\n"));
    eprintln!("{summary}: each passed with the batch pref off, on, and on with fusion at any size, with compiled parts and without");
}
