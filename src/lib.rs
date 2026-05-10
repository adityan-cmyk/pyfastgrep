use pyo3::prelude::*;

use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{SearcherBuilder, sinks::UTF8};
use ignore::WalkBuilder;

use rayon::prelude::*;
use std::sync::{Arc, Mutex};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crossbeam_channel::{bounded, Receiver};
use std::thread;

// glob helper
fn build_glob(glob: &Option<String>) -> Option<GlobSet> {
    if let Some(g) = glob {
        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new(g).unwrap());
        Some(builder.build().unwrap())
    } else {
        None
    }
}

// batch search
#[pyfunction]
fn search(
    pattern: String,
    root: String,
    glob: Option<String>,
    max_results: Option<usize>,
    ignore_case: Option<bool>,
) -> PyResult<Vec<(String, usize, String)>> {
    let is_case_insensitive = ignore_case.unwrap_or(false);
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(is_case_insensitive)
        .build(&pattern)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let glob_matcher: Option<GlobSet> = build_glob(&glob);

    let results = Arc::new(Mutex::new(Vec::new()));

    let entries: Vec<_> = WalkBuilder::new(&root)
        .standard_filters(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|entry| {
            if let Some(ref gs) = glob_matcher {
                gs.is_match(entry.path())
            } else {
                true
            }
        })
        .collect();

    entries.par_iter().for_each(|entry| {
        let path = entry.path();

        // Optional: skip empty files (cheap win)
        if path.metadata().map(|m| m.len() == 0).unwrap_or(false) {
            return;
        }

        let mut searcher = SearcherBuilder::new().build();
        let results = Arc::clone(&results);

        let _ = searcher.search_path(
            &matcher,
            path,
            UTF8(|lnum, line| {
                let mut res = results.lock().unwrap();

                if let Some(max) = max_results {
                    if res.len() >= max {
                        return Ok(false); // early exit
                    }
                }

                res.push((
                    path.display().to_string(),
                    lnum as usize,
                    line.to_string(),
                ));

                Ok(true)
            }),
        );
    });

    let final_results = Arc::try_unwrap(results)
        .unwrap()
        .into_inner()
        .unwrap();

    Ok(final_results)
}

// streaming iterator 
#[pyclass]
struct PyResultIterator {
    receiver: Receiver<(String, usize, String)>,
}

#[pymethods]
impl PyResultIterator {
    fn __iter__(slf: PyRef<Self>) -> Py<PyResultIterator> {
        slf.into()
    }

    fn __next__(slf: PyRefMut<Self>)-> Option<(String, usize, String)> {
        slf.receiver.recv().ok()
    }
}

#[pyfunction]
fn search_iter(
    pattern: String,
    root: String,
    glob: Option<String>,
    ignore_case: Option<bool>,
) -> PyResult<PyResultIterator> {
    let (tx, rx) = bounded(1000);

    let is_case_insensitive = ignore_case.unwrap_or(false);

    thread::spawn(move || {
        let matcher = match RegexMatcherBuilder::new()
            .case_insensitive(is_case_insensitive)
            .build(&pattern)
        {
            Ok(m) => m,
            Err(_) => return,
        };

        let glob_matcher: Option<GlobSet> = build_glob(&glob);

        let walker = WalkBuilder::new(&root)
            .standard_filters(true)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }

            if let Some(ref gs) = glob_matcher {
                if !gs.is_match(entry.path()) {
                    continue;
                }
            }

            let path = entry.path().to_path_buf();

            // skip empty files
            if path.metadata().map(|m| m.len() == 0).unwrap_or(false) {
                continue;
            }

            let mut searcher = SearcherBuilder::new().build();

            let _ = searcher.search_path(
                &matcher,
                &path,
                UTF8(|lnum, line| {
                    if tx.send((
                        path.display().to_string(),
                        lnum as usize,
                        line.to_string(),
                    )).is_err() {
                        return Ok(false); // stop if receiver gone
                    }
                    Ok(true)
                }),
            );
        }
    });

    Ok(PyResultIterator { receiver: rx })
}

// python module
#[pymodule]
fn pyfastgrep(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(search_iter, m)?)?;
    Ok(())
}