// The extension leaves Python's symbols undefined, for the interpreter that loads it to supply.
fn main() { pyo3_build_config::add_extension_module_link_args(); }
