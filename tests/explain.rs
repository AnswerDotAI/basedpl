//! `•explain` and `•memory`, which report what a run did, what the thread that runs a session holds, and what a value keeps alive.
use basedpl::{Session, settings::{Sizes, set_sizes}};

/// Whether `code` gives the value that `expected` gives, both run in `session`. An evaluation error fails.
fn gives(session: &Session, code: &str, expected: &str) -> bool {
    let [actual, wanted] = [code, expected].map(|c| {
        let result = session.eval(c);
        assert!(result.error.is_none(), "{c}: {:?}", result.error);
        result.value
    });
    actual.zip(wanted).is_some_and(|(a, w)| a.same(&w))
}

/// Checks a property that holds on every route, such as a value or a leak: a mismatch fails.
fn check(session: &Session, code: &str, expected: &str) { assert!(gives(session, code, expected), "{code} didn't give {expected}"); }

/// Records in `wrong` a mismatch in what one route did, such as a count of accumulations or of pending arrays: a release-2 obligation that fails no run.
fn report(wrong: &mut Vec<String>, session: &Session, code: &str, expected: &str) {
    if !gives(session, code, expected) { wrong.push(format!("{code} didn't give {expected}")); }
}

/// The counts in `•explain`'s and `•memory`'s reports: accumulations and their paths, fused groups' runs and copies, stored and pending arrays, and the bytes a value keeps alive. A count that differs from what the test expects is a release-2 obligation, which the test prints and which fails no run (`report`). An evaluation error, or a broken property that holds on every route, such as a leaked pending array, fails it (`check`). `reference_cases` checks routes (`reference::route_check`).
#[test]
fn reports_count_what_runs_did() {
    rustymath::parallel::set_threads(1);
    let mut wrong = Vec::new();
    assert!(std::mem::size_of::<basedpl::Number>() <= 32);
    let session = Session::new();
    check(&session, "v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3 ⋄ m←[2 1 0⋄1 3 1⋄0 1 4]ₓ ⋄ $t", "$t");
    // Each documented spelling takes its fast combination. Reference cases in `core.bpl` check that each gives what the same functions give through a dfn, and that each combination's route is taken.
    for (spelling, combination) in [
        ("(10↑)∘⍋ v", "partial grade"), ("¯10↑< v", "partial sort"), ("↑> v", "partial sort"), ("(3⌷)∘⍒ v", "partial grade"),
        ("⍋⍋ v", "ranks"), ("⍋⍒ v", "ranks"), ("(≢⊢)⌸ k", "group counts"), ("k((+/⊢)⌸)v", "group folds"),
        ("k((⌈⌿⊢)⌸)v", "group folds"), ("k((⌊/⊢)⌸)b", "group folds"), ("k((+/⊢)⌸)k", "group folds"), ("-.× m", "determinant"),
        ("+.× m", "permanent"),
    ] { report(&mut wrong, &session, &format!(r#"(⊂"{combination}")∊(•explain "{spelling}").routes.route"#), "$t"); }
    // Gradients through reversal, transpose, windows, a sum of windows, a broadcast, a maximum along an axis and windows that move by 2 make no accumulation. A selection that repeats positions makes one.
    for (code, scatters) in [
        ("{+/v×⌽⍵}∂ v", 0), ("{+/,(10 10⍴v)×⍉10 10⍴⍵}∂ 100↑v", 0), ("{+/,(998 3⍴v)×3↕⍵}∂ v", 0), ("{+/+/3↕⍵}∂ v", 0),
        ("{+/,(1000 2⍴v)×⍵+⊗0 0}∂ v", 0), ("{+/⌈⌿⍵}∂ 10 100⍴v", 0), ("{+/,(499 3⍴v)×[[3]⋄[2]]↕⍵}∂ v", 0),
        ("{+/⍵.[2|⍳≢⍵]}∂ v", 1),
    ] {
        report(&mut wrong, &session, &format!(r#"≢(•explain "{code}").accumulations.items"#), &format!("{scatters}ₓ"));
    }
    // Each path that adds a selection's items has a name: in place, on a converted copy, in rounds for repeated positions, in one call for distinct ones, and item by item for a dfn that isn't one primitive's spelling. Exact integers whose sums can overflow widen on a copy once, then add in place.
    check(&session, "i←¿1000⍴1000 ⋄ z←1000⍴0 ⋄ zx←1000⍴0ₓ ⋄ zh←16 •float 1000⍴0 ⋄ zr←1000⍴1ᵣ2 ⋄ $t", "$t");
    for (code, path) in [
        ("z.[i]+←1", "in place"), ("zx.[i]+←1ₓ", "on a copy"), ("zx.[i]+←1ₓ", "in place"), ("zh.[i]+←1", "on a copy"),
        ("zr.[i]+←1ᵣ3", "rounds"), ("zr.[⍳10]+←1ᵣ3", "one call"), ("z.[i] {⍺+2×⍵}←1", "item by item"),
    ] { report(&mut wrong, &session, &format!(r#"(•explain "{code}").accumulations.path≡["{path}"]"#), "$t"); }
    // A fused group counts the runs that a long cycle splits its blocks into, and the items it copies out of inputs. A pending input that a name also holds is computed by the group as one of its outputs, with no `stored` row.
    report(&mut wrong, &session, r#"big←¿[256 5000]⍴0 ⋄ f←(•explain "q←2×big(+⍤1)¿5000⍴0 ⋄ q≡q").fused ⋄ ∧/f.runs>f.blocks"#, "$t");
    // With every call waiting, small arrays take the routes that these checks count. The reversed copy needs a run that compiles on its first run (`COMPILES_AT`).
    set_sizes(Sizes { min_items: 1, ..Sizes::DEFAULT });
    report(&mut wrong, &session, r#"sv←¿65536⍴0 ⋄ sq←[256 256]⍴sv ⋄ (•explain "q←2×1+⍉sq ⋄ q≡q").fused.copied"#, "[65536]ₓ");
    report(&mut wrong, &session, r#"sv←¿4194304⍴0 ⋄ (•explain "q←2×⌽sv+1 ⋄ q≡q").fused.copied"#, "[0]ₓ");
    report(&mut wrong, &session, r#"x←¿1000⍴0 ⋄ e←•explain "s←x-0.25 ⋄ q←s×1-s ⋄ q≡q" ⋄ (0=≢e.stored.reason)∧e.fused.glyphs≡["--×"]"#, "$t");
    set_sizes(Sizes::DEFAULT);
    // `•memory` counts the pending arrays this thread made, including one dropped on another thread. `bytes` counts what a value keeps alive, a shared base once, and takes active values under `∂`.
    let held = Session::new();
    set_sizes(Sizes { min_items: 1, ..Sizes::DEFAULT });
    report(&mut wrong, &held, "w←⍳1000 ⋄ n←(•memory 0).pending ⋄ p←⌽w ⋄ (•memory 0).pending-n", "1ₓ");
    check(&held, "p←0 ⋄ (•memory 0).pending-n", "0ₓ");
    let copy = held.eval("⌽w").value.expect("a pending copy");
    report(&mut wrong, &held, "(•memory 0).pending-n", "1ₓ");
    std::thread::spawn(move || drop(copy)).join().expect("another thread drops the copy");
    check(&held, "(•memory 0).pending-n", "0ₓ");
    set_sizes(Sizes::DEFAULT);
    report(&mut wrong, &held, "v←⍳1000 ⋄ r←2 500⍴v ⋄ (•storage r).view∧(•memory r).bytes=(•memory v).bytes", "$t");
    report(&mut wrong, &held, "((•memory [r v]).bytes<2×(•memory v).bytes)∧(•memory ⌽w).bytes=(•memory w).bytes", "$t");
    check(&held, "b←0 ⋄ g←{b+←(•memory ⍵).bytes ⋄ +/⍵×⍵}∂ 1 2 3 ⋄ b>0", "$t");
    if !wrong.is_empty() { println!("report counts, release-2 obligations that fail no run: {wrong:#?}"); }
}
