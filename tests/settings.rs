//! Checks that change settings. The settings are one store for the whole process, so these checks run in turn, in one test, each from the defaults.
#[macro_use]
mod common;
use basedpl::{EvalOptions, Session, Value as AplValue};
use common::*;

#[test]
fn checks_that_change_settings() {
    let checks: [fn(); 8] = [
        language_examples, mathish_arithmetic, boxed_display_and_function_trees, expression_dissection, threads_give_the_same_results,
        hanzi_spellings_and_language, leading_prefs_set_how_their_code_reads, highlight_after_a_prefs_header,
    ];
    for check in checks {
        basedpl::settings::reset();
        check();
    }
}

fn mathish_arithmetic() {
    let s = Session::new();
    equiv_in!(&s; "0ₓ÷0ₓ" => "0ₓ", "0÷ₓ0" => "0ₓ", "÷/1 0 0" => "∞");
    equiv_in(&s, r#"•prefs ["mathish":$f] ⋄ [≠⍨0÷0;≠⍨1⍟1;0.3=0.1+0.2]"#, "[$t;$t;$f]");
    equiv_in(&s, r#"•prefs ["mathish":$t] ⋄ [0÷0;1⍟1;0.3=0.1+0.2]"#, "[0;0;$t]");
    equiv_in(&s, "[0ₚ÷0ₚ;≠⍨0ₓ₄÷0ₓ₄;≠⍨0ₓ₈÷0ₓ₈]", "[0;$t;$t]");
    equiv_in(&s, "n←[0ₛ÷0ₛ;0ₓ₂÷0ₓ₂] ⋄ [≠⍨¨n;{(•storage ⍵).kind}¨n]", r#"[$t $t;"float32" "float32"]"#);
    equiv_in(&s, "[÷¯0÷0;÷¯0 ¯0÷0 0;÷¯0÷¨0 0]", "[¯∞;¯∞ ¯∞;¯∞ ¯∞]");
    equiv_in(&s, r#"•prefs ["kinds":"static"] ⋄ n←[0÷0;0ₓ÷0ₓ;0j0ₚ÷0j0ₚ;1j0ₚ⍟1j0ₚ] ⋄ [≠⍨¨n;0.7=0.1+0.6]"#, "[$t $t $t $t;$f]");
    equiv_in(&s, r#"•prefs ["float":32] ⋄ [≠⍨0ₓ÷0ₓ;≠⍨[0 0]ₓ÷[0 0]ₓ;≠⍨0ₓ÷¨[0 0]ₓ]"#, "[$t;$t $t;$t $t]");
    equiv_in(
        &s,
        r#"•prefs ["kinds":"dynamic"] ⋄ n←1ₓ⍟1ₓ ⋄ p←1ₚ⍟1ₚ ⋄ [≠⍨n;(•storage n).kind;p;(•storage p).kind;0.3ₚ=0.1ₚ+0.2ₚ]"#,
        r#"[$t;"float32";0ₚ;"float64";$t]"#);
}

fn language_examples() {
    let mut failures = Vec::new();
    let page = std::env::var("BASEDPL_PAGE").ok();
    let prefs = std::fs::read_to_string("nbs/gallery/prefs.bpl").unwrap();
    for dir in ["nbs", "nbs/glyphs", "nbs/system", "nbs/gallery"] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let Some(ext) = path.extension().filter(|ext| *ext == "qmd" || *ext == "bpl") else { continue; };
            if page.as_deref().is_some_and(|p| !path.to_string_lossy().contains(p)) { continue; }
            let text = std::fs::read_to_string(&path).unwrap();
            // A gallery demo is a whole program, run after the gallery's settings, as the gallery runs it. A demo that defines `frame` also draws its first frame.
            if ext == "bpl" {
                let session = Session::new();
                let error = [prefs.as_str(), &text, r#"{3=•nc "frame"?frame 0;0}0"#].into_iter().find_map(|code| session.eval(code).error);
                if let Some(error) = error { failures.push(format!("{}: {error}", path.display())); }
                continue;
            }
            let mut fenced = false;
            for line in text.lines() {
                if line.starts_with("```") { fenced = !fenced; continue; }
                if fenced { continue; }
                for link in line.split("](").skip(1) {
                    let target = link.split(')').next().unwrap().split('#').next().unwrap();
                    if !target.is_empty() && !target.contains("://") {
                        assert!(path.parent().unwrap().join(target).exists(), "{}: missing {target}", path.display());
                    }
                }
            }
            for f in basedpl::reference::page(&text, None, &Default::default()) {
                failures.push(format!("{}:{}: {}\n{}", path.display(), f["line"], f["code"].as_str().unwrap(), f["message"].as_str().unwrap()));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn boxed_display_and_function_trees() {
    let s = Session::new();
    assert!(s.eval(r#"•prefs ["box":$t "trees":$t "fns":$t]"#).error.is_none());
    assert_eq!(s.eval("⊂4ₓ ⋄ ⊂⊂4ₓ ⋄ ⊂¨[0 1]ₓ").output_text(), ["⊂4ₓ", "⊂⊂4ₓ", "┌→────────┐\n│ ⊂0ₓ ⊂1ₓ │\n└∊────────┘"]);
    let r = s.eval(r#"A←2 3 4⍴"DUCKSWANBIRDWORMCAKESEED" ⋄ ⊂⍠2 A"#);
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.output_text(), ["┌→─────────────────────┐\n↓ \"DUCK\" \"SWAN\" \"BIRD\" │\n│ \"WORM\" \"CAKE\" \"SEED\" │\n└∊─────────────────────┘"]);
    assert_eq!(s.eval(r#"⍬ ⋄ "" ⋄ "a""bc" ⋄ ["i":2]⍴1 2"#).output_text(), ["⍬", r#""""#, r#""a""bc""#, r#"["i":2]⍴[1 2]"#]);
    assert_eq!(s.eval("0 3⍴0").output_text(), ["┌→────┐\n⌽0 0 0│\n└~────┘"]);
    assert_eq!(s.eval("1 2⍴1ₓ 0.5").output_text(), ["┌→─────┐\n↓1ₓ 0.5│\n└+─────┘"]);
    assert_eq!(s.eval("2 2⍴⍳4ₓ").output_text(), ["┌→──┐\n↓0 1│\n│2 3│\n└ₓ──┘"]);
    let f = s.eval("avg←+/÷≢ ⋄ avg");
    assert!(f.error.is_none() && f.value.is_none());
    assert_eq!(f.output_text(), ["fork\n├─ /\n│  └─ +\n├─ ÷\n└─ ≢"]);
    equiv_in(&s, "avg [1 2 4]ₓ", "7r3");
    let r = s.eval("{⎕←⍵ ⋄ ⍵}1 2");
    assert_eq!(r.output_text().len(), 2);
    assert_eq!(r.output_text()[0], r.output_text()[1]);
    assert!(s.eval(r#"•prefs ["fns":$f]"#).error.is_none());
    assert_eq!(s.eval("{⎕←⍵ ⋄ ⍵}1 2").output_text()[0], "1 2");
    equiv_in(&s, "⍕1 2", r#""1 2""#);
    assert!(s.eval(r#"•prefs ["box":$f]"#).error.is_none());
    assert_eq!(s.eval("1 2").output_text(), ["1 2"]);
    assert_eq!(s.eval("]Display ⎕←1 2").output_text(), ["1 2", "┌→──┐\n│1 2│\n└~──┘"]);
    assert_eq!(s.eval("1 2").output_text(), ["1 2"]);
    assert!(s.eval(r#"•prefs ["boks":$t]"#).error.is_some());
    assert!(s.eval(r#"p←•prefs ["limit":4 "edges":1]"#).error.is_none());
    assert_eq!(s.eval(r#"3 3⍴⍳9 ⋄ ⍳9 ⋄ ⎕←⍳9 ⋄ "abcdefghi" ⋄ ≢⍕⍳9"#).output_text(), ["0 … 2\n⋮ ⋱ ⋮\n6 … 8", "0 … 8", "0 1 2 3 4 5 6 7 8", "\"abc…i\"", "17ₓ"]);
    assert_eq!(s.eval("\"a\nb\nc\nd\ne\"").output_text(), ["\"a\nb\nc\n⋮\ne\""]);
    s.eval(r#"•prefs ["limit":∞ "width":10]"#);
    let narrow = s.eval(r#""abcd界efghijk""#).output_text().join("\n");
    assert!(narrow.starts_with("\"abcd") && narrow.ends_with("k\"") && narrow.contains('…'));
    assert!(unicode_width::UnicodeWidthStr::width(narrow.as_str()) <= 10);
    s.eval(r#"•prefs ["width":∞]"#);
    let controls = s.eval(",•ucs97 9 98").value.unwrap();
    assert_same(s.eval(&controls.literal()).value, &controls);
    equiv_in(&s, "⍕,•ucs97 9 98", ",•ucs97 9 98");
    assert_eq!(s.eval("1 2").output_text(), ["1 2"]);
    let wide = s.eval("]Display '界' 'a'").output_text().join("\n");
    assert!(wide.contains("界"));
    s.eval(r#"•prefs ["hanzi_aliases":$t]"#);
    assert!(s.eval(r#"•prefs ["notation":"simplified" "trees":$f]"#).error.is_none());
    assert_eq!(s.eval("+/÷≢").output_text(), ["加总除数"]);
    assert_eq!(s.eval("/").output_text(), ["总"]);
    let product = s.eval("+.×");
    assert_eq!(product.output_text(), ["加内乘"]);
    let product = product.function.unwrap();
    assert_eq!(product.bpl(), "+.×");
    assert_eq!(product.parts().unwrap().0, ".");
    assert_eq!(s.eval("f←{加总乙} ⋄ f").output_text(), ["{加总乙}"]);
    let array = s.eval(r#"[+/ "总"]"#);
    assert_eq!(array.output_text(), [r#"[加总 "总"]"#]);
    assert_eq!(array.value.unwrap().literal(), r#"[+/ "总"]"#);
    let key = AplValue::Function(s.late_bound("1键").unwrap());
    assert_eq!(AplValue::from_parts([1], vec![key.clone()], key).unwrap().literal(), "[(1键)]");
    let power = AplValue::Function(s.late_bound("f⍣永").unwrap());
    assert_eq!(AplValue::from_parts([], vec![power.clone()], power).unwrap().literal(), "(f⍣永)ᵘ");
    assert!(s.eval(r#"•prefs ["notation":"traditional"]"#).error.is_none());
    assert_eq!(s.eval("+/÷≢").output_text(), ["加總除數"]);
    s.eval(r#"•prefs ["trees":$t]"#);
    let tree = s.eval("avg").output_text().join("\n");
    for part in ["總", "加", "除", "數"] { assert!(tree.contains(part)); }
    assert!(s.eval("]dissect 1+2").output_text()[0].contains('加'));
    assert!(s.eval(r#"•prefs ["notation":"unknown"]"#).error.is_some());
    s.eval("f←{注 help\n乙}");
    assert_eq!(s.inspect("f").unwrap().help, "help");
}

fn expression_dissection() {
    let s = Session::new();
    s.eval(r#"•prefs ["trees":$t]"#);
    let r = s.eval("]dissect -@ 1 0 1# 1 2 3");
    assert!(r.error.is_none(), "{:?}", r.error);
    let tree = r.output_text()[0];
    for part in ["call", "@", "↣", "[1 0 1]", "[1 2 3]"] { assert!(tree.contains(part), "{tree}"); }
    assert_same(r.value, &vector(&[-1., 2., -3.]));
    assert_eq!(s.eval("1+2").output_text(), ["3"]);
    assert!(s.eval("+/÷≢").output_text()[0].contains("fork\n"));
    assert!(s.eval(r#"•prefs ["trees":$f "dissect":$t] ⋄ f←{⎕←⍵ ⋄ ⍵+1}"#).output_text().is_empty());
    let r = s.eval("f 2");
    assert_eq!(r.output_text().len(), 3);
    assert_eq!(r.output_text()[0], "2");
    assert_eq!(r.output_text()[2], "3");
    assert_same(r.value, &float(3.));
    assert_eq!(s.eval("f").output_text().len(), 1);
    let r = s.eval("↑[+]");
    assert!(r.function.is_some());
    assert_eq!(r.output_text().len(), 2);
    for code in ["1+⍳3", "(1+⍳3)ᵀ", "(+/⍳3) (≢⍳4)"] {
        let r = s.eval(code);
        assert_eq!(r.output_text().len(), 2, "{code}: {:?}", r.error);
        assert!(r.output_text()[0].contains('⍳'));
        assert_same(run(r.output_text()[0]).unwrap(), &r.value.unwrap());
    }
    assert!(s.eval("a←1+⍳3").output_text().is_empty());
    assert!(s.eval("{}0").output_text().is_empty());
    let quiet = || EvalOptions { echo: false, ..EvalOptions::default() };
    assert!(s.eval_with("1+2", quiet()).output_text().is_empty());
    assert_eq!(s.eval_with("]dissect 1+2", quiet()).output_text().len(), 1);
    assert!(s.eval("1÷'a'").error.is_some());
    assert_eq!(s.eval("1+2").output_text().len(), 2);
}

fn threads_give_the_same_results() {
    let data: Vec<_> = (0..(1 << 18) + 3).map(|i| i % 257 - 128).collect();
    let expected = vector(&data.iter().map(|&x| f64::from(x)).collect::<Vec<_>>());
    let s = Session::new();
    s.set("v", AplValue::from_buffer([data.len()], basedpl::buffer::Buffer::Integers(basedpl::buffer::IntBuf::I32(data))).unwrap()).unwrap();
    for threads in [1, 4] {
        assert!(s.eval(&format!(r#"•prefs ["threads":{threads}]"#)).error.is_none());
        check_in(&s, "64•float 32•float v", expected.clone());
    }
}

fn hanzi_spellings_and_language() {
    use basedpl::symbols::{Notation, canonical, symbols};
    let mut seen = std::collections::HashMap::new();
    let s = Session::new();
    equiv_in(&s, r#"•prefs ["hanzi_aliases":$t] ⋄ mean是加总除数句mean 2 4 9"#, "5");
    for symbol in symbols() {
        for (aliases, notation) in symbol.hanzi.into_iter().zip([Notation::Simplified, Notation::Traditional]) {
            assert!(!aliases.is_empty(), "{} needs Hanzi spellings", symbol.glyph);
            assert_eq!(notation.glyph(symbol.glyph), &aliases[..aliases.chars().next().unwrap().len_utf8()]);
            for alias in aliases.chars() {
                assert!(seen.insert(alias, symbol.glyph).is_none_or(|glyph| glyph == symbol.glyph), "duplicate alias {alias}");
                assert_eq!(canonical(alias).to_string(), symbol.glyph);
                if matches!(symbol.kind, "function" | "monadic-operator" | "dyadic-operator") {
                    assert_eq!(s.inspect(&alias.to_string()).unwrap().source, symbol.glyph);
                }
            }
        }
    }
    equiv_in!(&s;
        "mean是加总除数句mean 2 4 9" => "5", "加累 2 3 4" => "2 5 9", "乘表己至3" => "[1 2 3⋄2 4 6⋄3 6 9]", "加總 1 2 3" => "6",
        "負3减负1" => "¯2", "1e负3加1r负4" => "1e¯3+1r¯4", "[负3 负1]ₓ" => "[¯3 ¯1]ₓ", "[空]ₛ" => "[⍬]ₛ", "负永" => "¯∞",
        "{甲是2句甲加乙}3" => "5", "{永键键7句除'a'}0" => "7", "op是{丙乙}句加op 3" => "3", "r是[\"a1\":7]句r内a1" => "7",
        "f是{乙加1}句f次3隔5" => "8", "\"加总 注 负永\"" => "\"加总 注 负永\"", "'加'" => "'加'", "加总 1 2 3 注 汉字注释" => "6",
        "系prefs [\"float\":32]句(系storage 1).kind" => "\"float32\"", "語←1 ⋄ 𠀀←2 ⋄ 語+𠀀" => "3",
        "_加←1 ⋄ _+←2 ⋄ _加_总←3 ⋄ x_語←4 ⋄ [_加 _+ _加_总 x_語]" => "1 2 3 4",
    );
    assert_eq!(s.name_class("系time"), 3);
    assert_eq!(s.inspect("系time").unwrap().source, "•time");
    assert!(s.set("乙", number(1)).is_err() && !s.erase("乙"));
    equiv_in(&s, "{系nc \"乙\"}0", "2ₓ");
    equiv("かな←2 ⋄ 한글←3 ⋄ かな+한글", "5");
    let code = "加总 1 2句辞";
    let error = s.eval(code).error.unwrap();
    assert_eq!(error.span.as_ref().unwrap().range, 13..16);
    assert_eq!(error.span.as_ref().unwrap().source.text, code);
    fails_in(&s, "DOMAIN", &[r#"0 ⋄ •prefs ["hanzi_aliases":$f]"#, r#"p←["hanzi_aliases":$f] ⋄ •prefs p"#]);
    equiv_in(&s, r#"•prefs ["hanzi_aliases":$f] ⋄ 中文←7 ⋄ 中文"#, "7");
    equiv_in(&s, "中文+1", "8");
}

fn leading_prefs_set_how_their_code_reads() {
    let s = Session::new();
    s.eval(r#"•prefs ["kinds":"static" "int":32]"#);
    equiv_in(&s, r#"•prefs ["kinds":"dynamic" "int":64] ⋄ 2147483648ₓ"#, "2147483648ₓ");
    equiv_in(&s, r#"a←3000000000ₓ ⋄ •prefs ["kinds":"static" "int":32] ⋄ aₓ≡¯1294967296ₓ"#, "$t");
    equiv_in(&s, r#"•prefs ["kinds":"static" "int":32] ⋄ 0ₓ₈+/[2000000000 2000000000]ₓ₄"#, "4000000000ₓ₈");
    equiv_in(&s, r#"⍎"•prefs [""float"":32] ⋄ (•storage 1.5).kind""#, r#""float32""#);
}

fn highlight_after_a_prefs_header() {
    let code = "•prefs [\"hanzi_aliases\":$t]\n加1 注 comment";
    assert_eq!(basedpl::highlight(code, false).last().map(|(range, class)| (&code[range.clone()], *class)), Some(("注 comment", "comment")));
}
