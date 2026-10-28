//! Interned names. Parsing turns each name into a `Name`, a number that stands for its text. Binding storage compares and hashes these numbers. A name becomes text again only where text comes in or goes out: execute, `•nc`, `•nl`, `•ex`, `•load`'s record, error messages, inspection and the frontends.

use rustymath::hashing::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::{fmt, sync::{LazyLock, PoisonError, RwLock}};

/// A name, as the number that stands for its text. The numbers are the same in every session, because parsing has no session and a function carries its parsed definition from one session to another.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Name(u32);

/// The text of each name, by number, and the number of each text. Names are never removed, so each text is allocated once and kept for the life of the process.
struct Table { texts: Vec<&'static str>, numbers: HashMap<&'static str, u32> }

/// The six names that a call binds itself, in the order of their slots in a frame (`Name::implicit`), which a call fills in that order (`Session::call_defined`).
const IMPLICIT: [char; 6] = ['⍺', '⍵', '⍶', '⍹', '∇', '⍢'];

/// Whether `c` is one of the six names that a call binds itself.
pub(crate) fn is_implicit(c: char) -> bool { IMPLICIT.contains(&c) }

/// Every name. The six names that a call binds itself have numbers 0 to 5, their slots, and `$e` has 6.
static TABLE: LazyLock<RwLock<Table>> = LazyLock::new(|| {
    let mut table = Table { texts: Vec::new(), numbers: HashMap::new() };
    for c in IMPLICIT { table.add(Box::leak(c.to_string().into_boxed_str())); }
    table.add("$e");
    RwLock::new(table)
});

/// Each distinct list of a definition's locals (`Definition::locals`), kept for the life of the process, as names are.
static LOCALS: LazyLock<RwLock<HashSet<&'static [Name]>>> = LazyLock::new(|| RwLock::new(HashSet::new()));

/// The kept copy of `names`, a definition's locals, which a frame can hold without counting references.
pub(crate) fn kept(names: &[Name]) -> &'static [Name] {
    if names.is_empty() { return &[]; }
    interned(&LOCALS, |locals| locals.get(names).copied(), |locals| {
        let kept: &'static [Name] = Box::leak(names.into());
        locals.insert(kept);
        kept
    })
}

/// What `find` gives from the table that `lock` guards, or else what `add` gives after it adds the entry. Only an add takes the write lock, and `find` looks again under it, because another thread can add the same entry in between.
fn interned<T, R>(lock: &RwLock<T>, find: impl Fn(&T) -> Option<R>, add: impl FnOnce(&mut T) -> R) -> R {
    if let Some(found) = find(&lock.read().unwrap_or_else(PoisonError::into_inner)) { return found; }
    let mut table = lock.write().unwrap_or_else(PoisonError::into_inner);
    find(&table).unwrap_or_else(|| add(&mut table))
}

impl Table {
    fn add(&mut self, text: &'static str) -> u32 {
        let number = self.texts.len() as u32;
        self.texts.push(text);
        self.numbers.insert(text, number);
        number
    }
}

impl Name {
    pub(crate) const ALPHA: Self = Self(0);
    pub(crate) const OMEGA: Self = Self(1);
    pub(crate) const LEFT_OPERAND: Self = Self(2);
    pub(crate) const RIGHT_OPERAND: Self = Self(3);
    /// `$e`, the error that a handler caught.
    pub(crate) const ERROR: Self = Self(6);

    /// The name whose text is `text`.
    pub(crate) fn new(text: &str) -> Self { Self(interned(&TABLE, |table| table.numbers.get(text).copied(), |table| table.add(Box::leak(text.into())))) }

    /// The name's text.
    pub(crate) fn text(self) -> &'static str { TABLE.read().unwrap_or_else(PoisonError::into_inner).texts[self.0 as usize] }

    /// The slot in a frame of one of the six names that a call binds itself, or `None` for any other name.
    #[inline]
    pub(crate) fn implicit(self) -> Option<usize> { (self.0 < 6).then_some(self.0 as usize) }
}

impl fmt::Display for Name { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.text()) } }
impl fmt::Debug for Name { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:?}", self.text()) } }
