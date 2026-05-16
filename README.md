# pyfastgrep

Fast file search for Python powered by ripgrep's engine.

`pyfastgrep` is now organized as a small workspace:
- `crates/core/` contains the shared Rust search engine
- `pyfastgrep/` contains the Python bindings
- `cli/` contains the thin CLI binary

## Install

pip install pyfastgrep

## Usage

### Python API

```python
import pyfastgrep

results = pyfastgrep.search(r'"/[^"]*-[^"]*"', "src")

for r in results:
    print(r)
```

Ergonomic keyword aliases are also supported:

```python
results = pyfastgrep.search("FN", root="src", glob="*.rs", case_insensitive=True, limit=10)
json_results = pyfastgrep.search("FN", root="src", glob="*.rs", case_insensitive=True, as_json=True)
csv_output = pyfastgrep.search("FN", root="src", glob="*.rs", case_insensitive=True, as_csv=True)
pyfastgrep.search("FN", root="src", glob="*.rs", case_insensitive=True, as_csv=True, output_path="results.csv")
```

Usage by mode:

```python
# Plain tuples
pyfastgrep.search("fn", "src", "*.rs")

# JSON objects
pyfastgrep.search("fn", "src", "*.rs", as_json=True)

# CSV text
pyfastgrep.search("fn", "src", "*.rs", as_csv=True)

# CSV written to a file
pyfastgrep.search("fn", "src", "*.rs", as_csv=True, output_path="results.csv")

# Streaming iterator
for match in pyfastgrep.search_iter("fn", "src", "*.rs"):
    print(match)
```

### CLI

The CLI is a thin interface over the same Rust core:

```bash
pyfastgrep fn src --glob "*.rs" --ignore-case
pyfastgrep fn src --glob "*.rs" --ignore-case --json
pyfastgrep fn src --glob "*.rs" --ignore-case --csv
pyfastgrep fn src --glob "*.rs" --ignore-case --csv --output results.csv
```

You can also run it directly from the workspace while developing:

```bash
cargo run -p pyfastgrep-cli -- fn src --glob "*.rs" --ignore-case
```

CLI output modes:

```bash
cargo run -p pyfastgrep-cli -- fn src --glob "*.rs" --ignore-case --json
cargo run -p pyfastgrep-cli -- fn src --glob "*.rs" --ignore-case --csv
cargo run -p pyfastgrep-cli -- fn src --glob "*.rs" --ignore-case --csv --output results.csv
```

CLI flags at a glance:

```bash
pyfastgrep <pattern> [root] [--glob <pattern>] [--limit <n>] [--ignore-case] [--json] [--csv] [--output <file>] [--root <path>]
```

### AST-powered semantic search

Search by structure, not just text:

```python
# Find functions by name
pyfastgrep.search_functions("main", "src", "*.py")

# Find classes/structs by name
pyfastgrep.search_classes("MyClass", "src", "*.py")

# Find imports/use statements
pyfastgrep.search_imports("requests", "src", "*.py")

# Streaming AST search
for match in pyfastgrep.search_functions_iter("main", "src", "*.py"):
    print(match)
```

Supported languages: Rust, Python, C, C++, Go, JavaScript, TypeScript.

### CLI AST search

```bash
pyfastgrep build_config src --glob "*.rs" --functions
pyfastgrep PyResultIterator src --glob "*.rs" --classes
pyfastgrep pyo3 src --glob "*.rs" --imports
```

## Features
1. Uses ripgrep internals (fast regex search)
2. Parallel search
3. Respects .gitignore
4. Python-friendly API with ergonomic aliases
5. Thin CLI over the same Rust core
6. AST-powered semantic search (functions, classes, imports)
7. Streaming iterators for both regex and AST search
8. JSON, CSV, and tuple output modes