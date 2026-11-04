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

/// Every case of the coverage matrix but those `skipped.txt` lists (`reference::coverage`; `coverage.txt` lists its accepted gaps), every idiom case below and every reference case with a route directive takes the routes its work needs, or names the gap. `tests/routes.rs` checks the speed cases the same way, outside plain `cargo test`. A case that panics, and a reference case or one of `explain_and_memory`'s checks that fails to evaluate, counts as one failure without stopping the checks of the others. The matrix prints and skips a case that fails to evaluate. Run as `•explain` runs its text, after a warm-up run, a case takes each route that its `⍝⍝⍝ route` lines require, and records no slow-path event (`explain::Events::slow`) beyond the events that its directives name (`timing::Named`): `⍝⍝⍝ allows` lines, each with the principle that allows its event, and `⍝⍝⍝ gap` lines, each with what a faster route should do instead. A stopped trace needs a `gap` line. A `declined` event needs no line, because it explains another event, but a line may name one. A line whose event no longer appears fails too, so the lines stay true and a fixed gap's line goes (`timing::contracts`). It prints the gaps it tolerates (`cargo test -- --nocapture` shows them). It runs each case on one thread and holds the timing lock shared, as a test run does. It runs `explain_and_memory`'s checks first, because a capture records what every thread in its process does, so no other test may evaluate beside it.
#[test]
fn cases_take_their_routes() {
    let _lock = rustymath::timing::shared().unwrap();
    rustymath::parallel::set_threads(1);
    let mut wrong = Vec::new();
    let mut routes = explain_and_memory(&mut wrong);
    let gaps = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/coverage.txt");
    routes.extend(basedpl::reference::coverage(&gaps, &mut wrong, &InterruptHandle::default()).unwrap());
    for file in basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference")).unwrap() {
        for case in file.cases.iter().filter(|case| basedpl::reference::routed(case)) {
            match basedpl::reference::route(&file, case, &InterruptHandle::default()) {
                Ok(route) => routes.push(route), Err(failure) => wrong.push(format!("[{}] {failure}", case["section"].as_str().unwrap_or_default()))
            }
        }
    }
    timing::contracts(&routes, wrong);
}

fn explain_and_memory(wrong: &mut Vec<String>) -> Vec<timing::Capture> {
    let mut routes = Vec::new();
    assert!(std::mem::size_of::<basedpl::Number>() <= 32);
    let session = Session::new();
    check(wrong, &session, "v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3 ⋄ m←[2 1 0⋄1 3 1⋄0 1 4]ₓ ⋄ $t", "$t");
    // Each documented spelling takes its fast combination once. Reference cases in `core.bpl` check that each gives what the same functions give through a dfn.
    for (spelling, combination) in [
        ("(10↑)∘⍋ v", "partial grade"), ("¯10↑< v", "partial sort"), ("↑> v", "partial sort"), ("(3⌷)∘⍒ v", "partial grade"),
        ("⍋⍋ v", "ranks"), ("⍋⍒ v", "ranks"), ("k((+/⊢)⌸)v", "key classes"), ("k{+/⍵}⌸v", "key classes"), ("k((⌈⌿⊢)⌸)v", "key classes"),
        ("k((⌊/⊢)⌸)b", "key classes"), ("k((+/⊢)⌸)k", "key classes"), ("-.× m", "determinant"), ("+.× m", "permanent"), ("⍟1+v", "accuracy rule"),
        ("¯1+*v", "accuracy rule"), ("k÷ₓ3", "mark rule"),
    ] {
        routes.extend(capture(wrong, &session, serde_json::json!({"code":spelling, "route":[combination]})));
    }
    // `∂` frees its record when it returns: from its result, an outer name its function wrote, a nested `∂`, an attached rule and `⇄`'s cached linearisation.
    for code in [
        "{+/⍵×⍵}∂ 1 2 3", "{+/,⍵+.×⍵}∂ [1 2⋄3 4]", "x←0.5×⍳300000 ⋄ ≢{+/⊼1○⍵}∂ x",
        "s←0 ⋄ g←{s+←⍵×2 ⋄ +/⍵×⍵}∂ 1 2 ⋄ s", "{+/,{+/⍵×⍵×⍵}∂ ⍵}∂ 1 2",
        "r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ {+/r ⍵}∂ 1 2", "{+/⊼⍵-2}⇄ 0 0",
    ] {
        if let Some(error) = session.eval(code).error { wrong.push(format!("[explain_and_memory] {code}: {error:?}")); }
        check(wrong, &session, "(•memory 0).records", "0ₓ");
    }
    // Gradients through reversal, transpose, windows that move by 1 or 2, a sum of windows, a broadcast and a maximum along an axis make no accumulation: rustymath's `Transpose` reads each back, windows as a correlation, with fills between frames that move by more than 1, as a transposed strided convolution reads its cotangent. A selection that repeats positions makes one.
    for (code, scatters) in [
        ("{+/v×⌽⍵}∂ v", 0), ("{+/,(10 10⍴v)×⍉10 10⍴⍵}∂ 100↑v", 0), ("{+/,(998 3⍴v)×3↕⍵}∂ v", 0), ("{+/+/3↕⍵}∂ v", 0),
        ("{+/,(1000 2⍴v)×⍵+⊗0 0}∂ v", 0), ("{+/⌈⌿⍵}∂ 10 100⍴v", 0), ("{+/,(499 3⍴v)×[[3]⋄[2]]↕⍵}∂ v", 0),
        ("{+/⍵.[2|⍳≢⍵]}∂ v", 1),
    ] { check(wrong, &session, &format!(r#"≢(•explain "{code}").accumulations.items"#), &format!("{scatters}ₓ")); }
    // Each path that adds a selection's items has a name: in place, logged for exact integers whose sums can overflow, on a converted copy, and in rounds for repeated positions or in one call for distinct ones, by a primitive or a dfn.
    check(wrong, &session, "i←¿1000⍴1000 ⋄ z←1000⍴0 ⋄ zx←1000⍴0ₓ ⋄ zh←16 •float 1000⍴0 ⋄ zr←1000⍴1ᵣ2 ⋄ $t", "$t");
    for (code, path) in [
        ("z.[i]+←1", "in place"), ("zx.[i]+←1ₓ", "logged"), ("zh.[i]+←1", "on a copy"), ("zr.[i]+←1ᵣ3", "rounds"),
        ("zr.[⍳10]+←1ᵣ3", "one call"), ("z.[i] {⍺+⍵}←1", "rounds"),
    ] { check(wrong, &session, &format!(r#"(•explain "{code}").accumulations.path≡["{path}"]"#), "$t"); }
    // A fused group counts the runs that a long cycle splits its blocks into, and the items it copies out of inputs. A pending input that a name also holds is computed by the group as one of its outputs, with no `stored` row.
    check(wrong, &session, r#"big←¿[256 5000]⍴0 ⋄ f←(•explain "q←2×big(+⍤1)¿5000⍴0 ⋄ q≡q").fused ⋄ ∧/f.runs>f.blocks"#, "$t");
    // The reversed copy reads in place only in a compiled stage, and 4,194,304 items repay compiling `2×⌽sv+1` on its first run (`Blocks::repays`).
    check(wrong, &session, r#"sv←¿65536⍴0 ⋄ sq←[256 256]⍴sv ⋄ (•explain "q←2×1+⍉sq ⋄ q≡q").fused.copied"#, "[65536]ₓ");
    check(wrong, &session, r#"sv←¿4194304⍴0 ⋄ (•explain "q←2×⌽sv+1 ⋄ q≡q").fused.copied"#, "[0]ₓ");
    check(wrong, &session, r#"x←¿1000⍴0 ⋄ e←•explain "s←x-0.25 ⋄ q←s×1-s ⋄ q≡q" ⋄ (0=≢e.stored.reason)∧e.fused.glyphs≡["--×"]"#, "$t");
    set_sizes(Sizes::DEFAULT);
    // A faster route that code considers and doesn't take is a decline, with its reason: a fused fold of a copy that no call moves under, and an idiom whose arguments don't suit it.
    for (code, route, reason) in [
        ("+/⌽sv", "fused fold", "a copy that no call moves under"), ("(10↑)∘⍋ 30 4⍴⍳120", "partial grade", "arguments that don't suit it"),
        ("3↑< 4 1 3 2", "partial sort", "arguments that don't suit it"),
    ] {
        routes.extend(capture(wrong, &session, serde_json::json!({"code":code, "allows":[["declined",format!("{route}: {reason}")]]})));
    }
    // Ranks of a forward-mode dual, then `+/`: `+/` computes at once, and its sum of ordinary integers takes a type that depends on values, which stops the trace. A loop trace of the same body records, since the fold settles on its range (`plan::Fold::lanes`). Why the forward trace builds no fold node is unplaced (plan, stage 3's "State", gs11_'s entry, "Left" (3) (c)).
    routes.extend(capture(wrong, &session, serde_json::json!({"code":"{+/⍋⍋⍵×3 1 2}∂ 2",
    "gap":[["forced","+/"],["stopped","a result whose type depends on values"]]})));
    // `•memory` counts the pending arrays this thread made, including one dropped on another thread. `bytes` counts what a value keeps alive, a shared base once, and takes active values under `∂`.
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
    set_sizes(Sizes::DEFAULT);
    check(wrong, &held, "v←⍳1000 ⋄ r←2 500⍴v ⋄ (•storage r).view∧(•memory r).bytes=(•memory v).bytes", "$t");
    check(wrong, &held, "((•memory [r v]).bytes<2×(•memory v).bytes)∧(•memory ⌽w).bytes=(•memory w).bytes", "$t");
    check(wrong, &held, "b←0 ⋄ g←{b+←(•memory ⍵).bytes ⋄ +/⍵×⍵}∂ 1 2 3 ⋄ b>0", "$t");
    // A branch that `•detach` cuts makes no record nodes, where the same branch without it makes one.
    check(
        wrong, &held,
        "d←0 ⋄ e←0 ⋄ g←{r←(•memory 0).records ⋄ t←(•detach ⍵)×2 ⋄ d+←(•memory 0).records-r ⋄ u←⍵×2 ⋄ e+←(•memory 0).records-r ⋄ +/⍵}∂ 1 2 3 ⋄ (d=0)∧e=1",
        "$t");
    routes
}
