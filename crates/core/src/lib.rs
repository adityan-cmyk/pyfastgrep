use crossbeam_channel::{bounded, Receiver};
use globset::{Glob, GlobSet, GlobSetBuilder};
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{sinks::UTF8, SearcherBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use tree_sitter::{Parser, Query, QueryCursor};

pub mod ast;
use ast::TargetLanguage;

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub pattern: String,
    pub root: PathBuf,
    pub glob: Option<String>,
    pub max_results: Option<usize>,
    pub ignore_case: bool,
}

impl SearchConfig {
    pub fn new(pattern: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            pattern: pattern.into(),
            root: root.into(),
            glob: None,
            max_results: None,
            ignore_case: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchHit {
    pub file: String,
    pub line: usize,
    pub content: String,
}

pub type SearchReceiver = Receiver<SearchHit>;

pub fn search(config: &SearchConfig) -> Result<Vec<SearchHit>, String> {
    let matcher = build_matcher(&config.pattern, config.ignore_case)?;
    let glob_matcher = build_glob(&config.glob)?;
    let paths = collect_paths(&config.root, &glob_matcher);

    let mut results: Vec<SearchHit> = paths
        .par_iter()
        .map(|path| search_file(path, &matcher))
        .flatten()
        .collect();

    if let Some(max_results) = config.max_results {
        results.truncate(max_results);
    }

    Ok(results)
}

pub fn search_stream(config: SearchConfig) -> Result<SearchReceiver, String> {
    let matcher = build_matcher(&config.pattern, config.ignore_case)?;
    let glob_matcher = build_glob(&config.glob)?;
    let (tx, rx) = bounded(1000);

    thread::spawn(move || {
        let walker = WalkBuilder::new(&config.root)
            .standard_filters(true)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
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

            if path.metadata().map(|m| m.len() == 0).unwrap_or(false) {
                continue;
            }

            let mut searcher = SearcherBuilder::new().build();

            let _ = searcher.search_path(
                &matcher,
                &path,
                UTF8(|lnum, line| {
                    if tx
                        .send(SearchHit {
                            file: path.display().to_string(),
                            line: lnum as usize,
                            content: line.to_string(),
                        })
                        .is_err()
                    {
                        return Ok(false);
                    }

                    Ok(true)
                }),
            );
        }
    });

    Ok(rx)
}

// ---------------------------------------------------------------------------
// AST search
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum AstQueryType {
    Function,
    Class,
    Import,
}

pub type AstResultReceiver = Receiver<(String, usize, String)>;

pub fn search_ast(
    target_name: &str,
    root: &Path,
    glob: &Option<String>,
    query_type: AstQueryType,
) -> Result<Vec<(String, usize, String)>, String> {
    let glob_matcher = build_glob(glob)?;
    let results = Arc::new(Mutex::new(Vec::new()));

    let paths = collect_paths(root, &glob_matcher);

    paths.par_iter().for_each(|path| {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if let Some(lang) = TargetLanguage::from_extension(ext) {
            if let Ok(source_code) = fs::read_to_string(path) {
                let mut parser = Parser::new();
                let ts_lang = lang.get_parser_language();
                let _ = parser.set_language(ts_lang);

                if let Some(tree) = parser.parse(&source_code, None) {
                    let query_str = match query_type {
                        AstQueryType::Function => lang.function_query(),
                        AstQueryType::Class => lang.class_query(),
                        AstQueryType::Import => lang.import_query(),
                    };

                    if let Ok(query) = Query::new(ts_lang, query_str) {
                        let mut cursor = QueryCursor::new();
                        let matches = cursor.matches(&query, tree.root_node(), source_code.as_bytes());

                        for m in matches {
                            for capture in m.captures {
                                let node = capture.node;
                                let node_text = &source_code[node.byte_range()];

                                let is_match = match query_type {
                                    AstQueryType::Import => node_text.contains(target_name),
                                    _ => node_text == target_name,
                                };

                                if is_match {
                                    let start_pos = node.start_position();
                                    let line = source_code.lines().nth(start_pos.row).unwrap_or("").to_string();
                                    let mut res = results.lock().unwrap();
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

pub fn search_ast_stream(
    target_name: String,
    root: String,
    glob: Option<String>,
    query_type: AstQueryType,
) -> Result<AstResultReceiver, String> {
    let glob_matcher = build_glob(&glob)?;
    let (tx, rx) = bounded(1000);

    thread::spawn(move || {
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

            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            if let Some(lang) = TargetLanguage::from_extension(ext) {
                if let Ok(source_code) = fs::read_to_string(path) {
                    let mut parser = Parser::new();
                    let ts_lang = lang.get_parser_language();
                    let _ = parser.set_language(ts_lang);

                    if let Some(tree) = parser.parse(&source_code, None) {
                        let query_str = match query_type {
                            AstQueryType::Function => lang.function_query(),
                            AstQueryType::Class => lang.class_query(),
                            AstQueryType::Import => lang.import_query(),
                        };

                        if let Ok(query) = Query::new(ts_lang, query_str) {
                            let mut cursor = QueryCursor::new();
                            let matches = cursor.matches(&query, tree.root_node(), source_code.as_bytes());

                            for m in matches {
                                for capture in m.captures {
                                    let node = capture.node;
                                    let node_text = &source_code[node.byte_range()];

                                    let is_match = match query_type {
                                        AstQueryType::Import => node_text.contains(&target_name),
                                        _ => node_text == target_name,
                                    };

                                    if is_match {
                                        let start_pos = node.start_position();
                                        let line = source_code.lines().nth(start_pos.row).unwrap_or("").to_string();
                                        if tx.send((path.display().to_string(), start_pos.row + 1, line)).is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(rx)
}

// ---------------------------------------------------------------------------
fn build_glob(glob: &Option<String>) -> Result<Option<GlobSet>, String> {
    if let Some(g) = glob {
        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new(g).map_err(|e| e.to_string())?);
        Ok(Some(builder.build().map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

fn build_matcher(pattern: &str, ignore_case: bool) -> Result<RegexMatcher, String> {
    RegexMatcherBuilder::new()
        .case_insensitive(ignore_case)
        .build(pattern)
        .map_err(|e| e.to_string())
}

fn search_file(path: &Path, matcher: &RegexMatcher) -> Vec<SearchHit> {
    let Some(metadata) = path.metadata().ok() else {
        return Vec::new();
    };

    if metadata.len() == 0 {
        return Vec::new();
    }

    let mut hits = Vec::new();
    let mut searcher = SearcherBuilder::new().build();

    let _ = searcher.search_path(
        matcher,
        path,
        UTF8(|lnum, line| {
            hits.push(SearchHit {
                file: path.display().to_string(),
                line: lnum as usize,
                content: line.to_string(),
            });

            Ok(true)
        }),
    );

    hits
}

fn collect_paths(root: &Path, glob_matcher: &Option<GlobSet>) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|entry| match glob_matcher {
            Some(gs) => gs.is_match(entry.path()),
            None => true,
        })
        .map(|entry| entry.into_path())
        .collect()
}
