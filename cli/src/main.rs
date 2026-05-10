use pyfastgrep_core::{search, SearchConfig};
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let mut pattern: Option<String> = None;
    let mut root = PathBuf::from(".");
    let mut glob: Option<String> = None;
    let mut max_results: Option<usize> = None;
    let mut ignore_case = false;
    let mut json = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-g" | "--glob" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Missing value for --glob");
                    std::process::exit(1);
                }
                glob = Some(args[i].clone());
            }
            "-n" | "--limit" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Missing value for --limit");
                    std::process::exit(1);
                }
                max_results = args[i].parse::<usize>().ok();
            }
            "-i" | "--ignore-case" => {
                ignore_case = true;
            }
            "-j" | "--json" => {
                json = true;
            }
            "-r" | "--root" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Missing value for --root");
                    std::process::exit(1);
                }
                root = PathBuf::from(&args[i]);
            }
            value if value.starts_with('-') => {
                eprintln!("Unknown flag: {value}");
                print_usage();
                std::process::exit(1);
            }
            value => {
                if pattern.is_none() {
                    pattern = Some(value.to_string());
                } else if root == PathBuf::from(".") {
                    root = PathBuf::from(value);
                } else {
                    eprintln!("Unexpected positional argument: {value}");
                    print_usage();
                    std::process::exit(1);
                }
            }
        }

        i += 1;
    }

    let Some(pattern) = pattern else {
        eprintln!("Missing search pattern");
        print_usage();
        std::process::exit(1);
    };

    let mut config = SearchConfig::new(pattern, root);
    config.glob = glob;
    config.max_results = max_results;
    config.ignore_case = ignore_case;

    match search(&config) {
        Ok(results) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            } else {
                for hit in results {
                    println!("{}:{}: {}", hit.file, hit.line, hit.content.trim_end());
                }
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!(
        "Usage: pyfastgrep <pattern> [root] [--glob <pattern>] [--limit <n>] [--ignore-case] [--json] [--root <path>]"
    );
}
