use pyo3::prelude::*;

#[pyfunction]
fn version() -> &'static str {
    farol_core::VERSION
}

#[pymodule]
fn farol(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
