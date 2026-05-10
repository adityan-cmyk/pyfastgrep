use crossbeam_channel::{bounded, Receiver};
use globset::{Glob, GlobSet, GlobSetBuilder};
use grep::regex::RegexMatcherBuilder;
use grep::searcher::{sinks::UTF8, SearcherBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

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
    let results = Arc::new(Mutex::new(Vec::new()));
    let paths = collect_paths(&config.root, &glob_matcher);

    paths.par_iter().for_each(|path| {
        let Some(metadata) = path.metadata().ok() else {
            return;
        };

        if metadata.len() == 0 {
            return;
        }

        let mut searcher = SearcherBuilder::new().build();
        let results = Arc::clone(&results);
        let max_results = config.max_results;

        let _ = searcher.search_path(
            &matcher,
            path,
            UTF8(|lnum, line| {
                let mut res = results.lock().unwrap();

                if let Some(max) = max_results {
                    if res.len() >= max {
                        return Ok(false);
                    }
                }

                res.push(SearchHit {
                    file: path.display().to_string(),
                    line: lnum as usize,
                    content: line.to_string(),
                });

                Ok(true)
            }),
        );
    });

    Ok(Arc::try_unwrap(results).unwrap().into_inner().unwrap())
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

fn build_glob(glob: &Option<String>) -> Result<Option<GlobSet>, String> {
    if let Some(g) = glob {
        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new(g).map_err(|e| e.to_string())?);
        Ok(Some(builder.build().map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

fn build_matcher(pattern: &str, ignore_case: bool) -> Result<grep::regex::RegexMatcher, String> {
    RegexMatcherBuilder::new()
        .case_insensitive(ignore_case)
        .build(pattern)
        .map_err(|e| e.to_string())
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
