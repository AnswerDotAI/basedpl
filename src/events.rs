//! BPL's diagnostic vocabulary. Rustymath owns capture and mathematical route declarations.
pub(crate) use rustymath::events::{Column, Event, EventKind, Field, Record, Report, Route, Run, Sink, Table, declined, running, taken};

#[allow(non_upper_case_globals)]
pub(crate) mod routes {
    use super::Route;
    macro_rules! routes {
        ($($id:ident: $name:literal, $taken:literal, $declined:literal, $itemwise:literal;)+) => {
            pub const ALL: &[Route] = &[$($id),+];
            $(pub const $id: Route = Route::new($name, $taken, $declined, $itemwise);)+
        };
    }
    routes! {
        Trace: "trace", true, false, false;
        Fusion: "fusion", true, true, false;
        Batching: "batching", true, true, false;
        PartialGrade: "partial grade", true, true, false;
        PartialSort: "partial sort", true, true, false;
        Ranks: "ranks", true, true, false;
        GroupCounts: "group counts", true, false, false;
        GroupFolds: "group folds", true, true, false;
        BulkCatenate: "bulk catenate", true, false, false;
        Determinant: "determinant", true, true, false;
        Permanent: "permanent", true, true, false;
        MaskedSelect: "masked select", true, false, false;
        Program: "program", false, true, false;
        FusedFold: "fused fold", true, true, false;
        Compiled: "compiled", true, true, false;
        ItemPairs: "item pairs", false, false, true;
        ItemLanes: "item lanes", false, false, true;
        NumberParts: "number parts", false, false, true;
        Accumulation: "accumulation", false, false, true;
        NestedLabels: "nested labels", false, false, true;
        LabelIds: "label ids", false, false, true;
        InverseScan: "inverse scan", false, false, true;
        KeyGroups: "key groups", false, false, true;
        BatchCells: "batch cells", false, false, true;
        BatchedItems: "batched items", false, false, true;
        DerivativeItems: "∂ items", false, false, true;
        TracedItems: "traced items", false, false, true;
    }
}

pub fn event_routes() -> impl Iterator<Item = Route> { Route::ALL.iter().chain(routes::ALL).copied() }

macro_rules! detail_value {
    ($field:ident) => { $field };
    ($field:ident, $value:expr) => { $value };
}
macro_rules! detail_label {
    () => { false };
    (label) => { true };
}
macro_rules! details {
    ($($variant:ident, $table:ident, $name:literal { $($field:ident: $ty:ty => $kind:ident $(($value:expr))? $(@$label:ident)?),* };)+) => {
        #[cfg_attr(web, expect(dead_code, reason = "shared report schema retains native-JIT compile rows"))]
        pub(crate) enum Detail { $($variant { $($field: $ty),* }),+ }
        $(const $table: Table = ($name, &[$((stringify!($field), Column::$kind)),*]);)+
        impl Detail {
            pub(crate) fn event(self) -> Event {
                match self { $(Self::$variant { $($field),* } => Event::Row {
                    table: &$table,
                    fields: vec![$(Field::$kind(detail_value!($field $(, $value)?).into())),*],
                    label: [$(detail_label!($($label)?)),*].into_iter().position(|label| label),
                }),+ }
            }
        }
    };
}
details! {
    Fused, FUSED, "fused" { glyphs: String => Text, items: usize => Count, blocks: usize => Count, runs: usize => Count, copied: usize => Count };
    Stored, STORED, "stored" { glyphs: String => Text, items: usize => Count, reason: &'static str => Text @label, partway: bool => Truth };
    Forced, FORCED, "forced" { function: String => Text @label, line: usize => Count, column: usize => Count, items: usize => Count, reason: &'static str => Text, site: Option<String> => Text(site.unwrap_or_default()) };
    Program, PROGRAMS, "programs" { event: &'static str => Text, steps: usize => Count, listing: String => Text };
    Fast, FAST, "fast" { rule: &'static str => Text, function: String => Text };
    Fallback, FALLBACKS, "fallbacks" { reason: String => Text @label, items: usize => Count };
    Accumulation, ACCUMULATIONS, "accumulations" { items: usize => Count, size: usize => Count, path: &'static str => Text };
    Mixed, MIXED, "mixed" { code: String => Text @label, line: usize => Count, column: usize => Count, target: &'static str => Text, written: &'static str => Text, items: usize => Count };
    Compile, COMPILES, "compiles" { glyphs: String => Text, microseconds: usize => Count };
    Itemwise, ITEMWISE, "itemwise" { function: String => Text, line: usize => Count, column: usize => Count, route: Route => Text(route.name()) @label, items: usize => Count };
}

pub(crate) const TABLES: &[&Table] = &[
    &FUSED, &STORED, &FORCED, &PROGRAMS, &rustymath::events::STOPPED, &FAST, &FALLBACKS, &rustymath::events::DECLINED, &ACCUMULATIONS, &MIXED, &COMPILES,
    &rustymath::events::ROUTES, &ITEMWISE,
];

#[inline]
pub(crate) fn emit(make: impl FnOnce(bool) -> Detail) { rustymath::events::emit(|sites| make(sites).event()); }

#[inline]
pub(crate) fn stopped(reason: &'static str, items: usize, site: impl FnOnce() -> Option<String>) {
    rustymath::events::emit(|sites| {
        Event::Route(Record { kind: EventKind::Stopped, route: routes::Trace, reason, calls: 1, items, site: if sites { site() } else { None } })
    });
}
