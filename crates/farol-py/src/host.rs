//! Python plugin host.
//!
//! Implements [`farol_core::PluginHost`] by calling into a Python-side
//! `PluginManager` object. The Python side (pluggy + entry points) decides
//! which registered plugins provide each hook; from Rust's perspective we
//! just dispatch to the manager.

use std::path::Path;

use farol_core::{Config, FarolError, FileTree, Page, PluginHost, Result};
use pyo3::{
    prelude::*,
    types::{PyAny, PyDict, PyList},
};
use pythonize::{depythonize, pythonize};

/// A [`PluginHost`] that dispatches to a Python `farol.PluginManager`.
pub struct PythonPluginHost {
    manager: Py<PyAny>,
}

impl PythonPluginHost {
    /// Build from an existing Python object. The object must implement
    /// one method per hook (`on_config`, `on_page_markdown`, etc.) that
    /// returns `None` for "no change" or the transformed value otherwise.
    pub fn new(manager: Py<PyAny>) -> Self {
        Self { manager }
    }
}

fn to_farol(err: PyErr, hook: &str) -> FarolError {
    Python::attach(|py| err.print(py));
    FarolError::ConfigInvalid { message: format!("plugin hook `{hook}` failed: {err}") }
}

impl PluginHost for PythonPluginHost {
    fn name(&self) -> &str {
        "python"
    }

    fn plugins(&self) -> Vec<String> {
        Python::attach(|py| {
            self.manager
                .bind(py)
                .call_method0("plugins")
                .and_then(|v| v.extract::<Vec<String>>())
                .unwrap_or_default()
        })
    }

    fn on_config(&self, config: Config) -> Result<Config> {
        Python::attach(|py| -> Result<Config> {
            let cfg_py = pythonize(py, &config).map_err(|e| FarolError::ConfigInvalid {
                message: format!("serialize config: {e}"),
            })?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("config", cfg_py).map_err(|e| to_farol(e, "on_config"))?;
            let ret = self
                .manager
                .bind(py)
                .call_method("on_config", (), Some(&kwargs))
                .map_err(|e| to_farol(e, "on_config"))?;
            if ret.is_none() {
                return Ok(config);
            }
            depythonize::<Config>(&ret).map_err(|e| FarolError::ConfigInvalid {
                message: format!("on_config returned invalid config: {e}"),
            })
        })
    }

    fn on_files(&self, files: FileTree, _config: &Config) -> Result<FileTree> {
        Python::attach(|py| -> Result<FileTree> {
            let paths: Vec<String> =
                files.files.iter().map(|f| f.relative.to_string_lossy().into_owned()).collect();
            let list = PyList::new(py, paths).map_err(|e| to_farol(e, "on_files"))?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("files", list).map_err(|e| to_farol(e, "on_files"))?;
            self.manager
                .bind(py)
                .call_method("on_files", (), Some(&kwargs))
                .map_err(|e| to_farol(e, "on_files"))?;
            Ok(files)
        })
    }

    fn on_page_markdown(&self, markdown: String, page: &Page, config: &Config) -> Result<String> {
        Python::attach(|py| -> Result<String> {
            let kwargs = PyDict::new(py);
            kwargs.set_item("markdown", &markdown).map_err(|e| to_farol(e, "on_page_markdown"))?;
            let page_py = pythonize(py, page).map_err(|e| FarolError::ConfigInvalid {
                message: format!("serialize page: {e}"),
            })?;
            kwargs.set_item("page", page_py).map_err(|e| to_farol(e, "on_page_markdown"))?;
            let cfg_py = pythonize(py, config).map_err(|e| FarolError::ConfigInvalid {
                message: format!("serialize config: {e}"),
            })?;
            kwargs.set_item("config", cfg_py).map_err(|e| to_farol(e, "on_page_markdown"))?;

            let ret = self
                .manager
                .bind(py)
                .call_method("on_page_markdown", (), Some(&kwargs))
                .map_err(|e| to_farol(e, "on_page_markdown"))?;
            if ret.is_none() {
                return Ok(markdown);
            }
            ret.extract::<String>().map_err(|e| to_farol(e, "on_page_markdown"))
        })
    }

    fn on_page_html(&self, html: String, page: &Page, config: &Config) -> Result<String> {
        Python::attach(|py| -> Result<String> {
            let kwargs = PyDict::new(py);
            kwargs.set_item("html", &html).map_err(|e| to_farol(e, "on_page_html"))?;
            let page_py = pythonize(py, page).map_err(|e| FarolError::ConfigInvalid {
                message: format!("serialize page: {e}"),
            })?;
            kwargs.set_item("page", page_py).map_err(|e| to_farol(e, "on_page_html"))?;
            let cfg_py = pythonize(py, config).map_err(|e| FarolError::ConfigInvalid {
                message: format!("serialize config: {e}"),
            })?;
            kwargs.set_item("config", cfg_py).map_err(|e| to_farol(e, "on_page_html"))?;

            let ret = self
                .manager
                .bind(py)
                .call_method("on_page_html", (), Some(&kwargs))
                .map_err(|e| to_farol(e, "on_page_html"))?;
            if ret.is_none() {
                return Ok(html);
            }
            ret.extract::<String>().map_err(|e| to_farol(e, "on_page_html"))
        })
    }

    fn on_post_build(&self, site_dir: &Path, config: &Config) -> Result<()> {
        Python::attach(|py| -> Result<()> {
            let kwargs = PyDict::new(py);
            kwargs
                .set_item("site_dir", site_dir.to_string_lossy().into_owned())
                .map_err(|e| to_farol(e, "on_post_build"))?;
            let cfg_py = pythonize(py, config).map_err(|e| FarolError::ConfigInvalid {
                message: format!("serialize config: {e}"),
            })?;
            kwargs.set_item("config", cfg_py).map_err(|e| to_farol(e, "on_post_build"))?;
            self.manager
                .bind(py)
                .call_method("on_post_build", (), Some(&kwargs))
                .map_err(|e| to_farol(e, "on_post_build"))?;
            Ok(())
        })
    }
}
