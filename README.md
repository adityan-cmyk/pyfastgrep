# pyfastgrep

Fast file search for Python powered by ripgrep's engine.

`pyfastgrep` is now organized as a small workspace:
- `crates/core/` contains the shared Rust search engine
- `pyfastgrep/` contains the Python bindings
- `cli/` contains the thin CLI binary

## Install

pip install pyfastgrep

## Usage

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
```

CLI example:

```bash
pyfastgrep fn src --glob "*.rs" --ignore-case
```

## Features
1. Uses ripgrep internals (fast)
2. Parallel search
3. Respects .gitignore
4. Python-friendly API
5. Thin CLI over the same Rust core