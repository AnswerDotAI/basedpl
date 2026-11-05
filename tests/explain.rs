//! `•explain` and `•memory`, which report what a run did, what the thread that runs a session holds, and what a value keeps alive.
use basedpl::{InterruptHandle, Session, settings::{Sizes, set_sizes}, timing};

/// Checks that `code` gives the value that `expected` gives, both run in `session`. A failure is added to `wrong` instead of stopping the checks.
fn check(wrong: &mut Vec<String>, session: &Session, code: &str, expected: &str) {
    let [actual, wanted] = [code, expected].map(|c| session.eval(c));
    let failure = if let Some(error) = &actual.error { format!("{code}: {error:?}") }
        else if let Some(error) = &wanted.error { format!("{expected}: {error:?}") }
        else if actual.value.as_ref().zip(wanted.value.as_ref()).is_some_and(|(a, w)| a.same(w)) { return }
        else { format!("{code} didn't give {expected}") };
    wrong.push(format!("[explain_and_memory] {failure}"));
}

/// Captures `case` in `session` as `timing::route` does. A case that fails to evaluate or panics is added to `wrong` instead of stopping the checks.
fn capture(wrong: &mut Vec<String>, session: &Session, case: serde_json::Value) -> Option<timing::Capture> {
    timing::route(session, &case, &InterruptHandle::default()).map_err(|failure| wrong.push(format!("[explain_and_memory] {failure}"))).ok()
}

/// Runs selected matrix, idiom and annotated reference cases with their original setups and route contracts.
/// Unexpected evaluation errors and panics fail. Captures run serially because they observe every thread.
/// Select with BASEDPL_SOURCE (coverage, explain, or a reference name), BASEDPL_SECTION and BASEDPL_CASE.
#[test]
fn cases_take_their_routes() {
    let _lock = rustymath::timing::shared().unwrap();
    rustymath::parallel::set_threads(1);
    let mut wrong = Vec::new();
    let mut selection = basedpl::reference::Selection::from_env();
    let mut routes = explain_and_memory(&mut selection, &mut wrong);
    let gaps = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/coverage.txt");
    routes.extend(basedpl::reference::coverage(&gaps, &mut selection, &mut wrong, &InterruptHandle::default()).unwrap());
    for mut file in basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference")).unwrap() {
        basedpl::reference::mark_skipped(&mut file);
        for case in file.cases.iter().filter(|case| basedpl::reference::routed(case) && selection.case(&file.name, case)) {
            if case.get("skip_reason").is_some() { continue; }
            match basedpl::reference::route(&file, case, &InterruptHandle::default()) {
                Ok(route) => routes.push(route), Err(failure) => wrong.push(format!("[{}] {failure}", case["section"].as_str().unwrap_or_default()))
            }
        }
    }
    selection.assert_selected();
    timing::contracts(&routes, wrong);
}

fn selected<'a, T>(selection: &mut basedpl::reference::Selection, section: &str, cases: impl IntoIterator<Item = (&'a str, T)>) -> Vec<(&'a str, T)> {
    cases.into_iter().filter(|(code, _)| selection.includes("explain", section, code, code)).collect()
}

fn explain_and_memory(selection: &mut basedpl::reference::Selection, wrong: &mut Vec<String>) -> Vec<timing::Capture> {
    let mut routes = Vec::new();
    if !selection.source("explain") { return routes; }
    assert!(std::mem::size_of::<basedpl::Number>() <= 32);
    // Reference cases check the same idioms through dfns; each spelling here requires its fast combination.
    let cases = selected(selection, "idioms", [
        ("(10↑)∘⍋ v", "partial grade"), ("¯10↑< v", "partial sort"), ("↑> v", "partial sort"), ("(3⌷)∘⍒ v", "partial grade"),
        ("⍋⍋ v", "ranks"), ("⍋⍒ v", "ranks"), ("k((+/⊢)⌸)v", "key classes"), ("k{+/⍵}⌸v", "key classes"), ("k((⌈⌿⊢)⌸)v", "key classes"),
        ("k((⌊/⊢)⌸)b", "key classes"), ("k((+/⊢)⌸)k", "key classes"), ("-.× m", "determinant"), ("+.× m", "permanent"), ("⍟1+v", "accuracy rule"),
        ("¯1+*v", "accuracy rule"), ("k÷ₓ3", "mark rule"),
        ("{⍺+⍵}/1+v", "fused fold"),
        ("(0.5<)⍚[⊢ -] v", "trace"), ("-@[0.5<] v", "trace"), ("(0.5<)⍚[0 √ₐ] v", "trace"),
        ("(0.5<)⍚[⊢ {-⍵}] v", "trace"), ("{(0.5<)⍚[⊢ -]⍵}⍤1⁝100 10⍴v", "trace"), ("{+/(⍵>0.5)⍚[0 ÷]⍵}∂v", "trace"),
    ]);
    if !cases.is_empty() {
        let session = Session::new();
        check(wrong, &session, "v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3 ⋄ m←[2 1 0⋄1 3 1⋄0 1 4]ₓ ⋄ $t", "$t");
        for (code, route) in cases { routes.extend(capture(wrong, &session, serde_json::json!({"code":code, "route":[route]}))); }
    }
    for (code, ()) in selected(selection, "inverse folds", ["-/1+v", "÷/1+v", "{⍺-⍵}/1+v"].into_iter().map(|code| (code, ()))) {
        let session = Session::new();
        check(wrong, &session, "v←¿1000⍴0 ⋄ $t", "$t");
        routes.extend(capture(wrong, &session, serde_json::json!({"code":code, "route":["alternated fold", "fused fold"]})));
    }

    // Records are freed after returned, outer-name, nested, custom-rule and optimizer linearizations.
    for (code, ()) in selected(selection, "records", [
        "{+/⍵×⍵}∂ 1 2 3", "{+/,⍵+.×⍵}∂ [1 2⋄3 4]", "x←0.5×⍳300000 ⋄ ≢{+/⊼1○⍵}∂ x",
        "s←0 ⋄ g←{s+←⍵×2 ⋄ +/⍵×⍵}∂ 1 2 ⋄ s", "{+/,{+/⍵×⍵×⍵}∂ ⍵}∂ 1 2",
        "r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ {+/r ⍵}∂ 1 2", "{+/⊼⍵-2}⇄ 0 0",
    ].into_iter().map(|code| (code, ()))) {
        let session = Session::new();
        if let Some(error) = session.eval(code).error { wrong.push(format!("[explain_and_memory] {code}: {error:?}")); }
        check(wrong, &session, "(•memory 0).records", "0ₓ");
    }
    // Reversal, transpose, strided windows, broadcasting and maxima read back without scatter; repeated positions accumulate.
    let cases = selected(selection, "read-back", [
        ("{+/v×⌽⍵}∂ v", 0), ("{+/,(10 10⍴v)×⍉10 10⍴⍵}∂ 100↑v", 0), ("{+/,(998 3⍴v)×3↕⍵}∂ v", 0), ("{+/+/3↕⍵}∂ v", 0),
        ("{+/,(1000 2⍴v)×⍵+⊗0 0}∂ v", 0), ("{+/⌈⌿⍵}∂ 10 100⍴v", 0), ("{+/,(499 3⍴v)×[[3]⋄[2]]↕⍵}∂ v", 0),
        ("{+/⍵.[2|⍳≢⍵]}∂ v", 1),
    ]);
    if !cases.is_empty() {
        let session = Session::new();
        check(wrong, &session, "v←¿1000⍴0 ⋄ $t", "$t");
        for (code, scatters) in cases { check(wrong, &session, &format!(r#"≢(•explain "{code}").accumulations.items"#), &format!("{scatters}ₓ")); }
    }
    // Additive writes distinguish in-place, logged, converted, repeated-position and distinct-position work.
    let cases = selected(selection, "accumulation", [
        ("z.[i]+←1", "in place"), ("zx.[i]+←1ₓ", "logged"), ("zh.[i]+←1", "on a copy"), ("zr.[i]+←1ᵣ3", "rounds"),
        ("zr.[⍳10]+←1ᵣ3", "one call"), ("z.[i] {⍺+⍵}←1", "rounds"),
    ]);
    if !cases.is_empty() {
        let session = Session::new();
        check(wrong, &session, "i←¿1000⍴1000 ⋄ z←1000⍴0 ⋄ zx←1000⍴0ₓ ⋄ zh←16 •float 1000⍴0 ⋄ zr←1000⍴1ᵣ2 ⋄ $t", "$t");
        for (code, path) in cases { check(wrong, &session, &format!(r#"(•explain "{code}").accumulations.path≡["{path}"]"#), "$t"); }
    }
    // Long cycles split blocks into runs; read costs distinguish transpose copying from a compiled reversed read.
    for (code, expected) in selected(selection, "reads", [
        (r#"big←¿[256 5000]⍴0 ⋄ f←(•explain "q←2×big(+⍤1)¿5000⍴0 ⋄ q≡q").fused ⋄ ∧/f.runs>f.blocks"#, "$t"),
        (r#"sv←¿65536⍴0 ⋄ sq←[256 256]⍴sv ⋄ (•explain "q←2×1+⍉sq ⋄ q≡q").fused.copied"#, "[65536]ₓ"),
        (r#"sv←¿4194304⍴0 ⋄ (•explain "q←2×⌽sv+1 ⋄ q≡q").fused.copied"#, "[0]ₓ"),
        (r#"x←¿1000⍴0 ⋄ e←•explain "s←x-0.25 ⋄ q←s×1-s ⋄ q≡q" ⋄ (0=≢e.stored.reason)∧e.fused.glyphs≡["--×"]"#, "$t"),
    ]) { check(wrong, &Session::new(), code, expected); }
    set_sizes(Sizes::DEFAULT);
    // A considered fast route can decline with a reason without becoming a required route.
    for (code, (route, reason, setup)) in selected(selection, "declines", [
        ("+/⌽sv", ("fused fold", "a copy that no call moves under", "sv←¿4194304⍴0 ⋄ $t")),
        ("(10↑)∘⍋ 30 4⍴⍳120", ("partial grade", "arguments that don't suit it", "$t")),
        ("3↑< 4 1 3 2", ("partial sort", "arguments that don't suit it", "$t")),
    ]) {
        let session = Session::new();
        check(wrong, &session, setup, "$t");
        routes.extend(capture(wrong, &session, serde_json::json!({"code":code, "allows":[["declined",format!("{route}: {reason}")]]})));
    }
    // This forward-mode rank/fold currently forces the fold and stops at a value-dependent kind.
    let code = "{+/⍋⍋⍵×3 1 2}∂ 2";
    if selection.includes("explain", "stops", code, code) {
        routes.extend(capture(wrong, &Session::new(), serde_json::json!({"code":code,
            "gap":[["forced","+/"],["stopped","a result whose type depends on values"]]})));
    }
    // Pending counts include a value dropped on another thread.
    if selection.includes("explain", "memory", "pending-lifetime", "") {
        let held = Session::new();
        check(wrong, &held, "w←⍳1000 ⋄ n←(•memory 0).pending ⋄ p←⌽w ⋄ (•memory 0).pending-n", "1ₓ");
        check(wrong, &held, "p←0 ⋄ (•memory 0).pending-n", "0ₓ");
        match held.eval("⌽w").value {
            Some(copy) => {
                check(wrong, &held, "(•memory 0).pending-n", "1ₓ");
                std::thread::spawn(move || drop(copy)).join().expect("another thread drops the copy");
                check(wrong, &held, "(•memory 0).pending-n", "0ₓ");
            }
            None => wrong.push("[explain_and_memory] ⌽w: no value".into()),
        }
    }
    set_sizes(Sizes::DEFAULT);
    // Kept bytes count a shared base once, include views and accept values active under differentiation.
    if selection.includes("explain", "memory", "kept-bytes", "") {
        let held = Session::new();
        check(wrong, &held, "w←⍳1000 ⋄ v←⍳1000 ⋄ r←2 500⍴v ⋄ (•storage r).view∧(•memory r).bytes=(•memory v).bytes", "$t");
        check(wrong, &held, "((•memory [r v]).bytes<2×(•memory v).bytes)∧(•memory ⌽w).bytes=(•memory w).bytes", "$t");
        check(wrong, &held, "b←0 ⋄ g←{b+←(•memory ⍵).bytes ⋄ +/⍵×⍵}∂ 1 2 3 ⋄ b>0", "$t");
    }
    // Detaching a branch avoids its record nodes without suppressing the active branch.
    if selection.includes("explain", "memory", "detached-records", "") {
        check(wrong, &Session::new(),
            "d←0 ⋄ e←0 ⋄ g←{r←(•memory 0).records ⋄ t←(•detach ⍵)×2 ⋄ d+←(•memory 0).records-r ⋄ u←⍵×2 ⋄ e+←(•memory 0).records-r ⋄ +/⍵}∂ 1 2 3 ⋄ (d=0)∧e=1",
            "$t");
    }
    routes
}
