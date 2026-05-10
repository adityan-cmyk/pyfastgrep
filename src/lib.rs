use pyo3::prelude::*;
use pyfastgrep_core::{search as core_search, search_stream as core_search_stream, SearchConfig, SearchHit, SearchReceiver};
use serde_json::{json, Value};
use std::path::PathBuf;

fn build_config(
    pattern: String,
    root: String,
    glob: Option<String>,
    max_results: Option<usize>,
    ignore_case: Option<bool>,
) -> SearchConfig {
    SearchConfig {
        pattern,
        root: PathBuf::from(root),
        glob,
        max_results,
        ignore_case: ignore_case.unwrap_or(false),
    }
}

fn hits_to_json(py: Python<'_>, hits: Vec<SearchHit>) -> PyResult<PyObject> {
    let json_results: Vec<Value> = hits
        .into_iter()
        .map(|hit| {
            json!({
                "file": hit.file,
                "line": hit.line,
                "content": hit.content.trim_end()
            })
        })
        .collect();

    let json_string = serde_json::to_string(&json_results).unwrap();
    let json_module = py.import("json")?;
    let parsed = json_module.call_method("loads", (json_string,), None)?;
    Ok(parsed.into())
}

fn hits_to_tuples(py: Python<'_>, hits: Vec<SearchHit>) -> PyResult<PyObject> {
    let tuples: Vec<(String, usize, String)> = hits
        .into_iter()
        .map(|hit| (hit.file, hit.line, hit.content))
        .collect();

    Ok(tuples.into_py(py))
}

#[pyfunction]
fn search(
    pattern: String,
    root: String,
    glob: Option<String>,
    max_results: Option<usize>,
    ignore_case: Option<bool>,
    json: Option<bool>,
) -> PyResult<PyObject> {
    let config = build_config(pattern, root, glob, max_results, ignore_case);
    let return_json = json.unwrap_or(false);
    let hits = core_search(&config).map_err(pyo3::exceptions::PyValueError::new_err)?;

    Python::with_gil(|py| {
        if return_json {
            hits_to_json(py, hits)
        } else {
            hits_to_tuples(py, hits)
        }
    })
}

#[pyclass]
struct PyResultIterator {
    receiver: SearchReceiver,
    json_mode: bool,
}

#[pymethods]
impl PyResultIterator {
    fn __iter__(slf: PyRef<Self>) -> Py<PyResultIterator> {
        slf.into()
    }

    fn __next__(slf: PyRefMut<Self>) -> Option<PyObject> {
        let hit = slf.receiver.recv().ok()?;

        Python::with_gil(|py| {
            if slf.json_mode {
                let json_obj = json!({
                    "file": hit.file,
                    "line": hit.line,
                    "content": hit.content.trim_end()
                });
                let json_string = serde_json::to_string(&json_obj).unwrap();
                let json_module = py.import("json").ok()?;
                let parsed = json_module.call_method("loads", (json_string,), None).ok()?;
                Some(parsed.into())
            } else {
                Some((hit.file, hit.line, hit.content).into_py(py))
            }
        })
    }
}

#[pyfunction]
fn search_iter(
    pattern: String,
    root: String,
    glob: Option<String>,
    ignore_case: Option<bool>,
    json: Option<bool>,
) -> PyResult<PyResultIterator> {
    let config = build_config(pattern, root, glob, None, ignore_case);
    let receiver = core_search_stream(config).map_err(pyo3::exceptions::PyValueError::new_err)?;

    Ok(PyResultIterator {
        receiver,
        json_mode: json.unwrap_or(false),
    })
}

#[pymodule]
fn pyfastgrep(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(search_iter, m)?)?;
    Ok(())
}
