//! The route contracts too slow for plain `cargo test`: every speed case in `speed/*.bpl`, at its full size. `tests/explain.rs`'s `cases_take_their_routes` describes the contracts and checks the other cases. Plain `cargo test` skips this test binary (`test = false` in `Cargo.toml`) until its cases are fast again; `cargo test --test routes` runs it, as do CI's `Test` workflow and `bpl-tests prerelease`.
use basedpl::{InterruptHandle, timing};

/// Every speed case takes the routes its `⍝⍝⍝ route` lines require and records no slow event beyond those its directives name (`timing::contracts`). Each runs at its full size, after its section's setup, on one thread: the test takes minutes. A case that fails to evaluate or panics counts as one failure without stopping the others. It holds the timing lock shared, as a test run does.
#[test]
fn speed_cases_take_their_routes() {
    let _lock = rustymath::timing::shared().unwrap();
    rustymath::parallel::set_threads(1);
    let mut files = basedpl::reference::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/speed")).unwrap();
    let mut selection = basedpl::reference::Selection::from_env();
    selection.files(&mut files);
    selection.assert_selected();
    let (routes, failures) = timing::routes(&files, None, &InterruptHandle::default()).unwrap();
    timing::contracts(&routes, failures);
}
