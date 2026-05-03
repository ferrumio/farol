//! Python bindings for farol.
//!
//! Exposed as the `farol._native` extension module. The user-facing Python
//! API (decorators, PluginManager, PluginTester, CLI entry point) lives in
//! pure Python code under `python/farol/`.

mod host;

use std::sync::Arc;

use farol_core::PluginHost;
use host::PythonPluginHost;
use pyo3::{exceptions::PyRuntimeError, prelude::*};

#[pyfunction]
fn version() -> &'static str {
    farol_core::VERSION
}

/// Run the CLI with a Python-side PluginManager.
///
/// `argv` is the full argument list including `sys.argv[0]`.
/// `manager` must implement the hook dispatch API (see `farol.PluginManager`).
#[pyfunction]
#[pyo3(signature = (argv, manager))]
fn run_cli(argv: Vec<String>, manager: PyObject) -> PyResult<()> {
    let host: Arc<dyn PluginHost> = Arc::new(PythonPluginHost::new(manager));
    let result = Python::with_gil(|py| py.allow_threads(|| farol_cli::run_with_argv(argv, host)));
    result.map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))
}

/// Build a site programmatically.
///
/// `config_path` is the path to `farol.toml`. `manager` is the PluginManager.
/// Returns a dict with `pages`, `assets`, `broken_links`.
#[pyfunction]
#[pyo3(signature = (config_path, manager=None))]
fn build(py: Python<'_>, config_path: String, manager: Option<PyObject>) -> PyResult<PyObject> {
    use farol_core::{build as core_build, Config};
    use std::path::PathBuf;

    let config_path = PathBuf::from(config_path);
    let project_root =
        config_path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let config =
        Config::load(&config_path).map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))?;

    let report = py
        .allow_threads(|| -> farol_core::Result<farol_core::BuildReport> {
            match manager {
                Some(m) => {
                    let host = PythonPluginHost::new(m);
                    farol_core::build_with(
                        &config,
                        &project_root,
                        &farol_core::BuildOptions::default(),
                        &host,
                    )
                }
                None => core_build(&config, &project_root),
            }
        })
        .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))?;

    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("pages", report.pages)?;
    dict.set_item("assets", report.assets)?;
    dict.set_item("broken_links", report.broken_links.len())?;
    Ok(dict.into())
}

/// Expose the NoOpHost (useful when plugins are not desired).
#[pyfunction]
fn null_host() -> PyObject {
    // A Python-side PluginManager-compatible object that does nothing. Built
    // in pure Python, referenced through farol.PluginManager.null().
    Python::with_gil(|py| {
        let module = py.import("farol").unwrap();
        module.getattr("PluginManager").unwrap().call_method0("null").unwrap().unbind()
    })
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(build, m)?)?;
    m.add_function(wrap_pyfunction!(null_host, m)?)?;
    Ok(())
}
