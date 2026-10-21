//! Every glyph's kind, names and ranks, shared by completion, help and Python.
use crate::primitive::{OperatorKind, Primitive, Rank, WHOLE};
use rustymath::hashing::HashMap;
use std::sync::OnceLock;

// Glyph, simplified aliases, traditional aliases; an empty third field keeps the simplified spellings.
const HANZI: &[(&str, &str, &str)] = &[
    ("+", "加", ""), ("-", "减", "減"), ("×", "乘号", "乘號"), ("÷", "除", ""), ("⌊", "下", ""), ("⌈", "上", ""), ("|", "绝余", "絕餘"),
    ("*", "幂", "冪"), ("⍟", "对", "對"), ("○", "圆", "圓"), ("π", "周", ""), ("√", "根", ""), ("!", "阶拣", "階揀"), ("⍭", "素", ""),
    ("⨸", "因", ""), ("⌻", "零值", ""), ("¿", "掷抽", "擲抽"), ("∨", "析或", ""), ("∧", "极且", "極且"), ("⊼", "方否", ""),
    ("⊽", "双莫", "雙莫"), ("~", "非无", "非無"), ("=", "等", ""), ("≠", "别初", "別初"), ("<", "小", ""), ("≤", "限退", ""),
    (">", "大", ""), ("≥", "达进", "達進"), ("≡", "同深", ""), ("≢", "数异", "數異"), ("⍴", "形", ""), (",", "连", "連"),
    ("⍪", "接矩", ""), ("⌽", "转", "轉"), ("⊖", "纵", "縱"), ("⍉", "翻", ""), ("↑", "取", ""), ("↓", "去拆", ""), ("↕", "窗", ""),
    ("⊂", "包", ""), ("⊆", "巢段", ""), ("⊃", "选合", "選合"), ("⌷", "索", ""), ("⍳", "位", ""), ("↦", "至", ""),
    (":", "键卸", "鍵卸"), ("⍸", "处间", "處間"), ("∊", "属展", "屬展"), ("∪", "并独", "併獨"), ("∩", "交重", ""),
    ("⍷", "寻群", "尋群"), ("⍋", "升", ""), ("⍒", "降", ""), ("#", "复", "複"), ("⊤", "编", "編"), ("⊥", "译", "譯"),
    ("⌹", "解逆", ""), ("⊣", "左", ""), ("⊢", "右", ""), ("⍎", "行", ""), ("⍕", "文", ""), ("/", "总", "總"), ("⌿", "汇", "匯"),
    ("\\", "累", ""), ("⍀", "续", "續"), ("¨", "遍", ""), ("ᵇ", "按", ""), ("⍨", "己换", "己換"), ("↣", "先", ""), ("↢", "后", "後"), ("⍤", "秩", ""),
    ("∘", "承", ""), ("⍠", "轴", "軸"), ("⍥", "经", "經"), (".", "内", "內"), ("⊗", "表", ""), ("⌸", "组", "組"), ("⍣", "次止", ""),
    ("⇄", "优", "優"), ("⊘", "兼", ""), ("∂", "导梯", "導梯"), ("⍚", "择", "擇"), ("@", "于", "於"), ("⌺", "邻", "鄰"),
    ("←", "是", ""), ("⎕", "出入", ""), ("•", "系", ""), ("⍺", "甲", ""), ("⍵", "乙", ""), ("⍶", "丙", ""), ("⍹", "丁", ""),
    ("∇", "递", "遞"), ("⍢", "循", ""), ("⍝", "注", ""), ("⋄", "句", ""), ("?", "若", ""), ("¯", "负", "負"), ("∞", "永", ""),
    ("⍬", "空", ""),
];

fn hanzi(glyph: &str) -> [&'static str; 2] {
    HANZI.iter().find(|row| row.0 == glyph).map_or(["", ""], |&(_, simple, traditional)| [simple, if traditional.is_empty() { simple } else { traditional }])
}

/// The APL glyph that a Hanzi spelling names. Other characters stay unchanged.
pub fn canonical(c: char) -> char {
    if c < '\u{3400}' { return c; }
    static ALIASES: OnceLock<HashMap<char, char>> = OnceLock::new();
    *ALIASES.get_or_init(|| {
        HANZI.iter().flat_map(|&(glyph, simple, traditional)| simple.chars().chain(traditional.chars()).map(move |c| (c, glyph.chars().next().unwrap())))
            .collect()
    }).get(&c).unwrap_or(&c)
}

/// The spelling of generated function and operator notation, independent of input preferences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Notation { #[default] Apl, Simplified, Traditional }
impl Notation {
    pub fn glyph(self, glyph: &str) -> &str {
        let aliases = match self { Self::Apl => return glyph, Self::Simplified => hanzi(glyph)[0], Self::Traditional => hanzi(glyph)[1] };
        aliases.chars().next().map_or(glyph, |c| &aliases[..c.len_utf8()])
    }
}

/// A glyph's kind and names. `name` names the character, and its page in `nbs/glyphs`. `monad` and `dyad` name what the glyph does: a function's monadic and dyadic forms, or an operator's derived function with one argument and with two. An operator's `monad` covers both valences when its `dyad` is empty. A syntax glyph's one name for what it does is its `monad`. An empty name means the glyph has no such name.
pub struct Symbol {
    pub glyph: &'static str,
    pub name: &'static str,
    /// `"function"`, `"monadic-operator"`, `"dyadic-operator"`, or a syntax kind: `"argument"`, `"literal"`, `"comment"`, `"system"` or `"syntax"`.
    pub kind: &'static str,
    pub monad: &'static str,
    pub dyad: &'static str,
    /// The monadic form's natural rank, and the dyadic form's left and right ranks, as BPL text such as `1` or `1 ∞`. `∞` is the whole argument. Each is empty when the glyph has no such form.
    pub monad_rank: String,
    pub dyad_ranks: String,
    /// Alternative single-character Hanzi spellings: simplified, then traditional. The first in each is the preferred output.
    pub hanzi: [&'static str; 2],
}

// Glyphs that are syntax, not functions or operators: glyph, name, kind, and the name for what it does.
const SYNTAX: &[(&str, &str, &str, &str)] = &[
    ("←", "left-arrow", "syntax", "assign"), ("⎕", "quad", "syntax", "io"), ("•", "bullet", "system", "system"),
    ("⍺", "alpha", "argument", "left-argument"), ("⍵", "omega", "argument", "right-argument"), ("⍶", "alpha-underbar", "argument", "left-operand"),
    ("⍹", "omega-underbar", "argument", "right-operand"), ("∇", "del", "argument", "recursion"), ("⍢", "del-diaeresis", "argument", "operator-recursion"),
    ("⍝", "lamp", "comment", "comment"), ("⋄", "diamond", "syntax", "separator"), ("?", "question", "syntax", "predicate"),
    ("¯", "overbar", "literal", "negative"), ("∞", "infinity", "literal", ""), ("⍬", "zilde", "literal", "empty"),
];

/// Every glyph: syntax, then primitives and operators from their rows in `primitive.rs`.
pub fn symbols() -> &'static [Symbol] {
    static SYMBOLS: OnceLock<Vec<Symbol>> = OnceLock::new();
    SYMBOLS.get_or_init(|| {
        let syntax = SYNTAX.iter().map(|&(glyph, name, kind, monad)| Symbol { glyph, name, kind, monad, dyad: "", monad_rank: String::new(),
            dyad_ranks: String::new(), hanzi: hanzi(glyph) });
        let primitives = Primitive::all().map(|p| {
            let info = p.info();
            let (monad, dyad) = (info.monad.facts.name, info.dyad.facts.name);
            let rank = |r: Rank| if r == WHOLE { "∞".to_string() } else { r.to_string() };
            let (monad_rank, dyad_ranks) = (rank(info.monad.rank), info.dyad.ranks.map(rank).join(" "));
            Symbol { glyph: info.glyph, name: info.name, kind: "function", monad, dyad, monad_rank, dyad_ranks, hanzi: hanzi(info.glyph) }
        });
        let operators = OperatorKind::all().map(|op| {
            let info = op.info();
            let [name, monad, dyad] = info.names;
            let kind = if info.dyadic() { "dyadic-operator" } else { "monadic-operator" };
            Symbol { glyph: info.glyph, name, kind, monad, dyad, monad_rank: String::new(), dyad_ranks: String::new(), hanzi: hanzi(info.glyph) }
        });
        syntax.chain(primitives).chain(operators).collect()
    })
}

/// What a key does: type text, or move to a dead-key state, which types nothing until the next key.
#[derive(PartialEq)]
pub enum Action { Text(String), State(String) }

/// A dead-key state: what each listed key does next, and the terminator an unlisted key types first.
pub struct DeadState { pub terminator: String, pub keys: Vec<(char, Action)> }

/// The shared key mapping in `python/basedpl/layout.json`, which the macOS layout and the browser also read. Keys are US characters after Shift. `option` holds unshifted keys typed with Option.
pub struct KeyLayout {
    pub option: Vec<(char, Action)>, pub alt_aliases: Vec<(char, Action)>, pub states: HashMap<String, DeadState>, pub unshifted: HashMap<char, char>
}

impl KeyLayout {
    pub fn state(&self, name: &str) -> &DeadState { &self.states[name] }
    pub fn alt(&self, c: char) -> Option<&Action> { find(&self.alt_aliases, c).or_else(|| find(&self.option, c)) }
    pub fn display_keys<'a>(&'a self, keys: &'a [(char, Action)]) -> impl Iterator<Item = &'a (char, Action)> {
        keys.iter().filter(move |(key, action)| self.unshifted.get(key).and_then(|key| find(keys, *key)).is_none_or(|a| a != action))
    }
}

pub fn find(keys: &[(char, Action)], key: char) -> Option<&Action> { keys.iter().find(|(k, _)| *k == key).map(|(_, a)| a) }

pub fn layout() -> &'static KeyLayout {
    static LAYOUT: OnceLock<KeyLayout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        let json: serde_json::Value = serde_json::from_str(include_str!("../python/basedpl/layout.json")).expect("valid key layout");
        let action = |a: &serde_json::Value| match a.as_str() {
            Some(text) => Action::Text(text.into()), None => Action::State(a["state"].as_str().expect("a state name").into())
        };
        let keys =
            |m: &serde_json::Value| m.as_object().expect("a key map").iter().map(|(k, a)| (k.chars().next().expect("a key"), action(a))).collect::<Vec<_>>();
        let states = json["states"].as_object().expect("states").iter()
            .map(|(name, s)| (name.clone(), DeadState { terminator: s["terminator"].as_str().expect("a terminator").into(), keys: keys(&s["keys"]) }));
        let unshifted = json["unshifted"].as_object().expect("unshifted keys").iter()
            .map(|(shifted, plain)| (shifted.chars().next().unwrap(), plain.as_str().unwrap().chars().next().unwrap())).collect();
        KeyLayout { option: keys(&json["option"]), alt_aliases: keys(&json["alt_aliases"]), states: states.collect(), unshifted }
    })
}

/// The keys that type `glyph`, shown beside each listed name: ` a` for Option-A, or ` c t` for Option-C then T. The shortest sequence wins. A dead key's terminator, such as `○` for Option-O, shows the key alone. An ASCII glyph, such as `+` or `|`, gives nothing, because it has a key of its own.
pub fn chord(glyph: &str) -> String {
    if glyph.is_ascii() { return String::new(); }
    let layout = layout();
    let mut queue: std::collections::VecDeque<(String, &[(char, Action)])> = [(String::new(), layout.option.as_slice())].into();
    while let Some((typed, keys)) = queue.pop_front() {
        for (key, action) in layout.display_keys(keys) {
            let sequence = format!("{typed} {key}");
            match action {
                Action::Text(text) if text == glyph => return sequence, Action::State(name) if layout.state(name).terminator == glyph => return sequence,
                Action::State(name) => queue.push_back((sequence, &layout.state(name).keys)), Action::Text(_) => {}
            }
        }
    }
    String::new()
}

/// Every glyph's row, as `basedpl.symbols` and the browser's language bar read it: the fields of `Symbol`, and `shortcut`, the glyph's `chord`.
pub fn rows() -> serde_json::Value {
    let row = |s: &Symbol| {
        serde_json::json!({"glyph": s.glyph, "name": s.name, "kind": s.kind, "monad": s.monad, "dyad": s.dyad, "monad_rank": s.monad_rank,
            "dyad_ranks": s.dyad_ranks, "hanzi": s.hanzi, "shortcut": chord(s.glyph)})
    };
    symbols().iter().map(row).collect()
}

// Every name ranks alike at each level: exact, prefix, then prefixes of hyphen-separated parts. Each match carries the name it matched by, the glyph name where several of a glyph's names match equally.
pub fn matches(query: &str) -> Vec<(&'static str, &'static str)> {
    let query = query.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut best = usize::MAX;
    for &Symbol { glyph, name, monad, dyad, .. } in symbols() {
        let ranked = [name, monad, dyad].into_iter().filter(|word| !word.is_empty()).filter_map(|word| {
            let letters = word.replace('-', "");
            let rank = if letters == query { 0 }
            else if letters.starts_with(&query) { 1 } else {
                let mut rest = query.as_str();
                for part in word.split('-') {
                    let n = part.bytes().zip(rest.bytes()).take_while(|(a, b)| a == b).count();
                    if n == 0 { break; }
                    rest = &rest[n..];
                    if rest.is_empty() { break; }
                }
                if !rest.is_empty() { return None; }
                2
            };
            Some((rank, word))
        }).min_by_key(|&(rank, _)| rank);
        if let Some((rank, word)) = ranked {
            if rank < best {
                found.clear();
                best = rank;
            }
            if rank == best { found.push((glyph, word)); }
        }
    }
    // A name that is a prefix of every other match wins: `om gives omega, `omu gives omega-underbar.
    let letters = |name: &str| name.replace('-', "");
    let shortest = found.iter().copied().find(|a| found.iter().all(|b| letters(b.1).starts_with(&letters(a.1))));
    match shortest { Some(shortest) if found.len() > 1 => vec![shortest], _ => found }
}

pub fn entry(line: &str, pos: usize, aliases: bool) -> Option<(usize, &str)> {
    let start = line[..pos].rfind('`')?;
    let prefix = &line[start + 1..pos];
    (prefix.bytes().all(|c| c.is_ascii_alphabetic()) && crate::syntax::in_code(&line[..start], aliases)).then_some((start, prefix))
}
