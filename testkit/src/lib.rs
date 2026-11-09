//! BPL's reference checks and speed measurements, which its tests, `bpl --worker`'s test requests, the Python extension's test helpers and the browser's speed test use. Editing them rebuilds what depends on this crate, never the `basedpl` library.
pub mod reference;
pub mod timing;

pub(crate) const DEFAULT_PREFS: &str = r#"•prefs ["mathish":$f]"#;

/// basedpl's root directory, which holds `tests/`: the one above this crate's.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) fn root() -> &'static std::path::Path { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("testkit sits in basedpl's root") }

/// Takes the machine's timing lock exclusively, as `rustymath_timing::exclusive` describes, then writes `started` to stderr. A worker's caller counts a timing's deadline from that note (`basedpl.worker`).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) fn timing_lock() -> Result<rustymath_timing::Hold, String> {
    let hold = rustymath_timing::exclusive().map_err(|e| e.to_string())?;
    eprintln!("started");
    Ok(hold)
}

/// The browser build takes no timing lock. `basedpl.bpltests` takes it around the module's timings.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) fn timing_lock() -> Result<(), String> { Ok(()) }
