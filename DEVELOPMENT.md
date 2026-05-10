# Development Setup Guide

## Prerequisites
- Rust (install via rustup from https://rustup.rs/)
- Python 3.8+
- pip

## Initial Setup

1. Clone the repository
2. Create a virtual environment:
   ```bash
   python -m venv .venv
   .venv\Scripts\Activate  # On Windows
   # or
   source .venv/bin/activate  # On macOS/Linux
   ```

3. Install development dependencies:
   ```bash
   pip install maturin pytest
   ```

4. **Install git hooks** (runs tests before push):
   ```bash
   bash setup-hooks.sh  # On macOS/Linux
   # On Windows, run the equivalent:
   cp scripts/pre-push .git/hooks/pre-push
   ```

5. Build the extension:
   ```bash
   maturin develop
   ```

## Repository Layout

- `crates/core/` holds the shared search engine
- `pyfastgrep/` holds the Python binding layer
- `cli/` holds the thin command line interface
- `scripts/pre-push` is the tracked pre-push hook source

## Running Tests

```bash
python tests/mandatory_tests.py
```

## Development Workflow

1. Make your changes to the Rust code or Python API
2. Run `maturin develop` to rebuild the extension
3. Test your changes: `python tests/mandatory_tests.py`
4. Commit and push - the pre-push hook will automatically validate everything

## Git Hooks

The `pre-push` hook will:
- Rebuild the extension with maturin
- Run the mandatory test suite
- Abort the push if either step fails

This ensures code quality and stability on every push.
