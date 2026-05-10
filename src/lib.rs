use pyo3::prelude::*;

mod ast;
use ast::TargetLanguage;
use tree_sitter::{Parser, Query, QueryCursor};
use std::fs;

use grep::regex::RegexMatcher;
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
) -> PyResult<Vec<(String, usize, String)>> {
    let matcher = RegexMatcher::new(&pattern)
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
) -> PyResult<PyResultIterator> {
    let (tx, rx) = bounded(1000);

    thread::spawn(move || {
        let matcher = match RegexMatcher::new(&pattern) {
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

// AST query selector
enum QueryType {
    Function,
    Class,
    Import,
}

// AST search engine engine
fn search_ast_engine(
    target_name: String,
    root: String,
    glob: Option<String>,
    query_type: QueryType,
) -> PyResult<Vec<(String, usize, String)>> {
    let glob_matcher = build_glob(&glob);
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
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        if let Some(lang) = TargetLanguage::from_extension(ext) {
            if let Ok(source_code) = fs::read_to_string(path) {
                let mut parser = Parser::new();
                let ts_lang = lang.get_parser_language();
                let _ = parser.set_language(ts_lang);
                
                if let Some(tree) = parser.parse(&source_code, None) {
                    let query_str = match query_type {
                        QueryType::Function => lang.function_query(),
                        QueryType::Class => lang.class_query(),
                        QueryType::Import => lang.import_query(),
                    };
                    
                    if let Ok(query) = Query::new(ts_lang, query_str) {
                        let mut cursor = QueryCursor::new();
                        let matches = cursor.matches(&query, tree.root_node(), source_code.as_bytes());
                        
                        for m in matches {
                            for capture in m.captures {
                                let node = capture.node;
                                let node_text = &source_code[node.byte_range()];
                                
                                // For imports, often strings or paths match, doing a contains check avoids exact match issues.
                                // For functions/classes exact match works best.
                                let is_match = match query_type {
                                    QueryType::Import => node_text.contains(&target_name),
                                    _ => node_text == target_name,
                                };

                                if is_match {
                                    let start_pos = node.start_position();
                                    let line = source_code.lines().nth(start_pos.row).unwrap_or("").to_string();
                                    
                                    let mut res = results.lock().unwrap();
                                    // simple deduplication (tree-sitter can yield multiple captures per line sometimes)
                                    let item = (path.display().to_string(), start_pos.row + 1, line);
                                    if !res.contains(&item) {
                                        res.push(item);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let final_results = Arc::try_unwrap(results)
        .unwrap()
        .into_inner()
        .unwrap();

    Ok(final_results)
}

// AST API endpoints
#[pyfunction]
fn search_functions(
    target_name: String,
    root: String,
    glob: Option<String>,
) -> PyResult<Vec<(String, usize, String)>> {
    search_ast_engine(target_name, root, glob, QueryType::Function)
}

#[pyfunction]
fn search_classes(
    target_name: String,
    root: String,
    glob: Option<String>,
) -> PyResult<Vec<(String, usize, String)>> {
    search_ast_engine(target_name, root, glob, QueryType::Class)
}

#[pyfunction]
fn search_imports(
    target_name: String,
    root: String,
    glob: Option<String>,
) -> PyResult<Vec<(String, usize, String)>> {
    search_ast_engine(target_name, root, glob, QueryType::Import)
}

// python module
#[pymodule]
fn pyfastgrep(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(search_iter, m)?)?;
    m.add_function(wrap_pyfunction!(search_functions, m)?)?;
    m.add_function(wrap_pyfunction!(search_classes, m)?)?;
    m.add_function(wrap_pyfunction!(search_imports, m)?)?;
    Ok(())
}