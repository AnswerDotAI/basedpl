//! `•explain` and `•memory`, which report what a run did and what the thread that runs a session holds for differentiation.
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
    check(&mut session, "y←¿1000000⍴0 ⋄ v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3 ⋄ m←[2 1 0⋄1 3 1⋄0 1 4]ₓ ⋄ $t", "$t");
    // A fold reads through the fused group it reduces, which nothing forces. A call that reads every item forces the group.
    check(&mut session, r#"e←•explain "+/(2×y+1.5)-1" ⋄ (e.fused.glyphs≡["+×-+/"])∧0=≢e.forced.function"#, "$t");
    check(&mut session, r#"e←•explain "≢⍕(2×y+1.5)-1" ⋄ (e.fused.glyphs≡["+×-"])∧e.forced.function≡["⍕"]"#, "$t");
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
}
