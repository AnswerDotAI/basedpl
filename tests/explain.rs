//! `•explain` and `•memory`, which report what a run did, what the thread that runs a session holds, and what a value keeps alive.
use basedpl::{timing, InterruptHandle, Session};

/// Checks that `code` gives the value that `expected` gives, both run in `session`.
fn check(session: &Session, code: &str, expected: &str) {
    let [actual, wanted] = [code, expected].map(|c| {
        let result = session.eval(c);
        assert!(result.error.is_none(), "{c}: {:?}", result.error);
        result.value
    });
    assert!(actual.zip(wanted).is_some_and(|(a, w)| a.same(&w)), "{code} didn't give {expected}");
}

/// Speed and idiom cases require their fast routes and name any accepted slow path. Captures run serially because they record all threads.
#[test]
fn speed_cases_take_their_routes() {
    let _lock = rustymath::timing::shared().unwrap();
    rustymath::parallel::set_threads(1);
    let mut routes = explain_and_memory();
    let files = basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap();
    routes.extend(timing::routes(&files, None, &InterruptHandle::default()).unwrap());
    for file in basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference")).unwrap() {
        for case in file.cases.iter().filter(|case| basedpl::reference::routed(case)) {
            let mut route = basedpl::reference::route(case, &file.setup, &InterruptHandle::default()).unwrap();
            route.file = file.name.clone();
            route.label = format!("{}:{}", file.name, route.label);
            routes.push(route);
        }
    }
    let (mut wrong, mut gaps) = (Vec::new(), Vec::new());
    for r in &routes {
        let case = format!("[{}] {}", r.section, r.label);
        wrong.extend(r.violations().into_iter().map(|message| format!("{case}: {message}")));
        gaps.extend(r.named.gaps.iter().map(|(record, event)| format!("{case}: {record} \"{event}\"")));
    }
    use rustymath::events::{EventKind, Route};
    for &route in Route::ALL {
        for (kind, required) in [(EventKind::Taken, route.reports_taken()), (EventKind::Declined, route.reports_decline()), (EventKind::Itemwise, route.reports_itemwise())] {
            if !required { continue; }
            let seen = |events: &rustymath::events::Report| events.records.iter().any(|e| e.kind == kind && e.route == route);
            if let Some(r) = routes.iter().find(|r| seen(&r.events)).or_else(|| routes.iter().find(|r| seen(&r.warmup))) { println!("observed {kind:?} {} [{}] {} ({})", route.name(), r.section, r.label, if seen(&r.events) { "hot" } else { "warmup-only" }); }
            else { wrong.push(format!("no case exercises {kind:?} {}", route.name())); }
        }
    }
    println!("gaps that route cases tolerate: {gaps:#?}");
    assert!(wrong.is_empty(), "route contracts: {wrong:#?}");
}

fn explain_and_memory() -> Vec<timing::Route> {
    let mut routes = Vec::new();
    assert!(std::mem::size_of::<basedpl::Number>() <= 32);
    let session = Session::new();
    check(&session, "v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3 ⋄ m←[2 1 0⋄1 3 1⋄0 1 4]ₓ ⋄ $t", "$t");
    // Each documented spelling takes its fast combination once. Reference cases in `core.bpl` check that each gives what the same functions give through a dfn.
    for (spelling, combination) in [
        ("(10↑)∘⍋ v", "partial grade"), ("¯10↑< v", "partial sort"), ("↑> v", "partial sort"), ("(3⌷)∘⍒ v", "partial grade"),
        ("⍋⍋ v", "ranks"), ("⍋⍒ v", "ranks"), ("(≢⊢)⌸ k", "group counts"), ("k((+/⊢)⌸)v", "group folds"), ("k((⌈⌿⊢)⌸)v", "group folds"),
        ("k((⌊/⊢)⌸)b", "group folds"), ("k((+/⊢)⌸)k", "group folds"), ("-.× m", "determinant"), ("+.× m", "permanent"),
    ] {
        routes.push(timing::route(&session, &serde_json::json!({"code":spelling, "route":[combination]}), &InterruptHandle::default()).unwrap());
    }
    // `∂` frees its record when it returns: from its result, an outer name its function wrote, a nested `∂`, an attached rule and `⇄`'s cached linearisation.
    for code in [
        "{+/⍵×⍵}∂ 1 2 3", "{+/,⍵+.×⍵}∂ [1 2⋄3 4]", "x←0.5×⍳300000 ⋄ ≢{+/⊼1○⍵}∂ x", "s←0 ⋄ g←{s+←⍵×2 ⋄ +/⍵×⍵}∂ 1 2 ⋄ s",
        "{+/,{+/⍵×⍵×⍵}∂ ⍵}∂ 1 2", "r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ {+/r ⍵}∂ 1 2", "{+/⊼⍵-2}⇄ 0 0",
    ] {
        assert!(session.eval(code).error.is_none(), "{code}");
        check(&session, "(•memory 0).records", "0ₓ");
    }
    // Gradients through reversal, transpose, windows, a sum of windows, a broadcast, a maximum along an axis and windows that move by 2 make no accumulation. A selection that repeats positions makes one.
    for (code, scatters) in [
        ("{+/v×⌽⍵}∂ v", 0), ("{+/,(10 10⍴v)×⍉10 10⍴⍵}∂ 100↑v", 0), ("{+/,(998 3⍴v)×3↕⍵}∂ v", 0), ("{+/+/3↕⍵}∂ v", 0),
        ("{+/,(1000 2⍴v)×⍵+⊗0 0}∂ v", 0), ("{+/⌈⌿⍵}∂ 10 100⍴v", 0), ("{+/,(499 3⍴v)×[[3]⋄[2]]↕⍵}∂ v", 0), ("{+/⍵.[2|⍳≢⍵]}∂ v", 1),
    ] {
        check(&session, &format!(r#"≢(•explain "{code}").accumulations.items"#), &format!("{scatters}ₓ"));
    }
    // Each path that adds a selection's items has a name: in place, logged for exact integers whose sums can overflow, on a converted copy, in rounds for repeated positions, in one call for distinct ones, and item by item for a dfn.
    check(&session, "i←¿1000⍴1000 ⋄ z←1000⍴0 ⋄ zx←1000⍴0ₓ ⋄ zh←16 •float 1000⍴0 ⋄ zr←1000⍴1ᵣ2 ⋄ $t", "$t");
    for (code, path) in [
        ("z.[i]+←1", "in place"), ("zx.[i]+←1ₓ", "logged"), ("zh.[i]+←1", "on a copy"),
        ("zr.[i]+←1ᵣ3", "rounds"), ("zr.[⍳10]+←1ᵣ3", "one call"), ("z.[i] {⍺+⍵}←1", "item by item"),
    ] {
        check(&session, &format!(r#"(•explain "{code}").accumulations.path≡["{path}"]"#), "$t");
    }
    // A fused group counts the runs that a long cycle splits its blocks into, and the items it copies out of inputs. A pending input that a name also holds is computed by the group as one of its outputs, with no `stored` row.
    check(&session, r#"big←¿[256 5000]⍴0 ⋄ f←(•explain "q←2×big(+⍤1)¿5000⍴0 ⋄ q≡q").fused ⋄ ∧/f.runs>f.blocks"#, "$t");
    check(&session, r#"sv←¿1048576⍴0 ⋄ sq←[1024 1024]⍴sv ⋄ (•explain "q←2×1+⍉sq ⋄ q≡q").fused.copied"#, "[1048576]ₓ");
    check(&session, r#"sv←¿4194304⍴0 ⋄ (•explain "q←2×⌽sv+1 ⋄ q≡q").fused.copied"#, "[0]ₓ");
    check(&session, r#"x←¿1000000⍴0 ⋄ e←•explain "s←x-0.25 ⋄ q←s×1-s ⋄ q≡q" ⋄ (0=≢e.stored.reason)∧e.fused.glyphs≡["--×"]"#, "$t");
    // A faster route that code considers and doesn't take is a decline, with its reason: a fused fold of a copy that no call moves under, and an idiom whose arguments don't suit it.
    for (code, route, reason) in [
        ("+/⌽sv", "fused fold", "a copy that no call moves under"),
        ("(10↑)∘⍋ 30 4⍴⍳120", "partial grade", "arguments that don't suit it"),
        ("↑< [3 4⋄1 2]", "partial sort", "arguments that don't suit it"),
        (r#"⍋⍋ ["a":30 "b":10 "c":20]"#, "ranks", "arguments that don't suit it"),
    ] {
        routes.push(timing::route(&session, &serde_json::json!({"code":code, "allows":[["declined",format!("{route}: {reason}")]]}), &InterruptHandle::default()).unwrap());
    }
    // `•memory` counts the pending arrays this thread made, including one dropped on another thread. `bytes` counts what a value keeps alive, a shared base once, and takes active values under `∂`.
    let held = Session::new();
    check(&held, "w←⍳262144 ⋄ n←(•memory 0).pending ⋄ p←⌽w ⋄ (•memory 0).pending-n", "1ₓ");
    check(&held, "p←0 ⋄ (•memory 0).pending-n", "0ₓ");
    let copy = held.eval("⌽w").value.expect("a pending copy");
    check(&held, "(•memory 0).pending-n", "1ₓ");
    std::thread::spawn(move || drop(copy)).join().expect("another thread drops the copy");
    check(&held, "(•memory 0).pending-n", "0ₓ");
    check(&held, "v←⍳1000 ⋄ r←2 500⍴v ⋄ (•storage r).view∧(•memory r).bytes=(•memory v).bytes", "$t");
    check(&held, "((•memory [r v]).bytes<2×(•memory v).bytes)∧(•memory ⌽w).bytes=(•memory w).bytes", "$t");
    check(&held, "b←0 ⋄ g←{b+←(•memory ⍵).bytes ⋄ +/⍵×⍵}∂ 1 2 3 ⋄ b>0", "$t");
    // A branch that `•detach` cuts makes no record nodes, where the same branch without it makes one.
    check(&held, "d←0 ⋄ e←0 ⋄ g←{r←(•memory 0).records ⋄ t←(•detach ⍵)×2 ⋄ d+←(•memory 0).records-r ⋄ u←⍵×2 ⋄ e+←(•memory 0).records-r ⋄ +/⍵}∂ 1 2 3 ⋄ (d=0)∧e=1", "$t");
    routes
}
