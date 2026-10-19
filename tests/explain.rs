//! `•explain` and `•memory`, which report what a run did, what the thread that runs a session holds, and what a value keeps alive.
use basedpl::Session;

/// Checks that `code` gives the value that `expected` gives, both run in `session`.
fn check(session: &mut Session, code: &str, expected: &str) {
    let [actual, wanted] = [code, expected].map(|c| {
        let result = session.eval(c);
        assert!(result.error.is_none(), "{c}: {:?}", result.error);
        result.value
    });
    assert!(actual.zip(wanted).is_some_and(|(a, w)| a.same(&w)), "{code} didn't give {expected}");
}

#[test]
fn explain_and_memory() {
    let mut session = Session::new();
    check(&mut session, "v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3 ⋄ m←[2 1 0⋄1 3 1⋄0 1 4]ₓ ⋄ $t", "$t");
    // Each documented spelling takes its fast combination once. Reference cases in `core.bpl` check that each gives what the same functions give through a dfn.
    for (spelling, combination) in [
        ("(10↑)∘⍋ v", "partial grade"), ("¯10↑< v", "partial sort"), ("↑> v", "partial sort"), ("(3⌷)∘⍒ v", "partial grade"),
        ("⍋⍋ v", "ranks"), ("⍋⍒ v", "ranks"), ("(≢⊢)⌸ k", "group counts"), ("k (+/⊢)⌸ v", "group folds"), ("k (⌈⌿⊢)⌸ v", "group folds"),
        ("k (⌊/⊢)⌸ b", "group folds"), ("k (+/⊢)⌸ k", "group folds"), ("-.× m", "determinant"), ("+.× m", "permanent"),
    ] {
        check(&mut session, &format!(r#"(•explain "{spelling}").fast.combination≡["{combination}"]"#), "$t");
    }
    // `∂` frees its record when it returns: from its result, an outer name its function wrote, a nested `∂`, an attached rule and `⇄`'s cached linearisation.
    for code in [
        "{+/⍵×⍵}∂ 1 2 3", "{+/,⍵+.×⍵}∂ [1 2⋄3 4]", "x←0.5×⍳300000 ⋄ ≢{+/⊼1○⍵}∂ x", "s←0 ⋄ g←{s+←⍵×2 ⋄ +/⍵×⍵}∂ 1 2 ⋄ s",
        "{+/,{+/⍵×⍵×⍵}∂ ⍵}∂ 1 2", "r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ {+/r ⍵}∂ 1 2", "{+/⊼⍵-2}⇄ 0 0",
    ] {
        assert!(session.eval(code).error.is_none(), "{code}");
        check(&mut session, "(•memory 0).records", "0ₓ");
    }
    // Gradients through reversal, transpose, windows, a sum of windows, a broadcast, a maximum along an axis and windows that move by 2 make no accumulation. A selection that repeats positions makes one.
    for (code, scatters) in [
        ("{+/v×⌽⍵}∂ v", 0), ("{+/,(10 10⍴v)×⍉10 10⍴⍵}∂ 100↑v", 0), ("{+/,(998 3⍴v)×3↕⍵}∂ v", 0), ("{+/+/3↕⍵}∂ v", 0),
        ("{+/,(1000 2⍴v)×⍵+⊗0 0}∂ v", 0), ("{+/⌈⌿⍵}∂ 10 100⍴v", 0), ("{+/,(499 3⍴v)×[[3]⋄[2]]↕⍵}∂ v", 0), ("{+/⍵.[2|⍳≢⍵]}∂ v", 1),
    ] {
        check(&mut session, &format!(r#"≢(•explain "{code}").accumulations.items"#), &format!("{scatters}ₓ"));
    }
    // Each path that adds a selection's items has a name: in place, logged for exact integers whose sums can overflow, on a converted copy, in rounds for repeated positions, in one call for distinct ones, and item by item for a dfn.
    check(&mut session, "i←¿1000⍴1000 ⋄ z←1000⍴0 ⋄ zx←1000⍴0ₓ ⋄ zc←1000⍴0ₓ ⋄ zr←1000⍴1ᵣ2 ⋄ $t", "$t");
    for (code, path) in [
        ("z.[i]+←1", "in place"), ("zx.[i]+←1ₓ", "logged"), ("zc.[i]+←0.5", "on a copy"),
        ("zr.[i]+←1ᵣ3", "rounds"), ("zr.[⍳10]+←1ᵣ3", "one call"), ("z.[i] {⍺+⍵}←1", "item by item"),
    ] {
        check(&mut session, &format!(r#"(•explain "{code}").accumulations.path≡["{path}"]"#), "$t");
    }
    // A fused group counts the runs that a long cycle splits its blocks into, and the items it copies out of inputs. A pending input that a name holds is stored on its own first.
    check(&mut session, r#"big←¿[256 5000]⍴0 ⋄ f←(•explain "q←2×big+⍤1⊢¿5000⍴0 ⋄ q≡q").fused ⋄ ∧/f.runs>f.blocks"#, "$t");
    check(&mut session, r#"sv←¿1048576⍴0 ⋄ (•explain "q←2×⌽sv+1 ⋄ q≡q").fused.copied"#, "[1048576]ₓ");
    check(&mut session, r#"x←¿1000000⍴0 ⋄ (•explain "s←x-0.25 ⋄ q←s×1-s ⋄ q≡q").stored.reason≡["held outside the graph"]"#, "$t");
    // `•memory` counts the pending arrays this thread made, including one dropped on another thread. `bytes` counts what a value keeps alive, a shared base once, and takes active values under `∂`.
    let mut held = Session::new();
    check(&mut held, "w←⍳262144 ⋄ n←(•memory 0).pending ⋄ p←⌽w ⋄ (•memory 0).pending-n", "1ₓ");
    check(&mut held, "p←0 ⋄ (•memory 0).pending-n", "0ₓ");
    let copy = held.eval("⌽w").value.expect("a pending copy");
    check(&mut held, "(•memory 0).pending-n", "1ₓ");
    std::thread::spawn(move || drop(copy)).join().expect("another thread drops the copy");
    check(&mut held, "(•memory 0).pending-n", "0ₓ");
    check(&mut held, "v←⍳1000 ⋄ r←2 500⍴v ⋄ (•storage r).view∧(•memory r).bytes=(•memory v).bytes", "$t");
    check(&mut held, "((•memory [r v]).bytes<2×(•memory v).bytes)∧(•memory ⌽w).bytes=(•memory w).bytes", "$t");
    check(&mut held, "b←0 ⋄ g←{b+←(•memory ⍵).bytes ⋄ +/⍵×⍵}∂ 1 2 3 ⋄ b>0", "$t");
    // A branch that `•detach` cuts makes no record nodes, where the same branch without it makes one.
    check(&mut held, "d←0 ⋄ e←0 ⋄ g←{r←(•memory 0).records ⋄ t←(•detach ⍵)×2 ⋄ d+←(•memory 0).records-r ⋄ u←⍵×2 ⋄ e+←(•memory 0).records-r ⋄ +/⍵}∂ 1 2 3 ⋄ (d=0)∧e=1", "$t");
}
