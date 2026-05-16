import sys
import os
import subprocess
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

sys.path.insert(0, str(REPO_ROOT))

import pyfastgrep


def run_test(name, func):
    try:
        func()
        print(f"PASS: {name}")
        return True
    except AssertionError as exc:
        print(f"FAIL: {name} - {exc}")
        return False


def main():
    print("Running pyfastgrep test suite...")
    source_root = str(REPO_ROOT / "src")

    def test_case_sensitive_search():
        res_sensitive = pyfastgrep.search("FN", source_root, "*.rs", None, False, False)
        assert len(res_sensitive) == 0, f"Expected 0 results for case-sensitive 'FN', got {len(res_sensitive)}"

    def test_ignore_case_search():
        res_ignore = pyfastgrep.search("FN", source_root, "*.rs", None, True, False)
        assert len(res_ignore) > 0, "Expected >0 results for 'FN' with ignore_case=True"

    def test_iterator_matches_batch():
        res_ignore = pyfastgrep.search("FN", source_root, "*.rs", None, True, False)
        iter_ignore = list(pyfastgrep.search_iter("FN", source_root, "*.rs", True, False))
        assert len(iter_ignore) == len(res_ignore), "Batch and iterator search result counts should match"

    def test_json_output():
        json_results = pyfastgrep.search("fn", source_root, "*.rs", None, False, True)
        json_iter = list(pyfastgrep.search_iter("fn", source_root, "*.rs", False, True))

        assert len(json_results) > 0, "Expected >0 results for JSON batch search"
        assert len(json_iter) > 0, "Expected >0 results for JSON iterator search"
        assert isinstance(json_results[0], dict), "JSON batch results should contain dicts"
        assert isinstance(json_iter[0], dict), "JSON iterator results should contain dicts"
        assert {'file', 'line', 'content'} <= set(json_results[0].keys()), "JSON results should have file, line, and content keys"

    def test_csv_output():
        csv_path = Path(tempfile.gettempdir()) / "pyfastgrep_api_output.csv"
        csv_iter_path = Path(tempfile.gettempdir()) / "pyfastgrep_api_iter_output.csv"

        for candidate in (csv_path, csv_iter_path):
            if candidate.exists():
                candidate.unlink()

        csv_results = pyfastgrep.search("fn", source_root, "*.rs", None, False, False, as_csv=True, output_path=str(csv_path))
        csv_iter = list(pyfastgrep.search_iter("fn", source_root, "*.rs", False, False, csv=True, output_path=str(csv_iter_path)))

        assert isinstance(csv_results, str), "CSV batch results should be a string"
        assert csv_results.startswith("file,line,content"), "CSV batch results should start with a header"
        assert len(csv_iter) > 1, "CSV iterator should include a header and at least one row"
        assert csv_iter[0] == "file,line,content\n", "CSV iterator should yield the header first"
        assert csv_iter[1].endswith("\n"), "CSV iterator rows should end with a newline"
        assert csv_path.exists(), "CSV batch should write a file"
        assert csv_iter_path.exists(), "CSV iterator should write a file"
        assert csv_path.read_text(encoding="utf-8").startswith("file,line,content"), "CSV batch file should start with a header"
        assert csv_iter_path.read_text(encoding="utf-8").startswith("file,line,content"), "CSV iterator file should start with a header"

    def test_cli_csv():
        csv_path = Path(tempfile.gettempdir()) / "pyfastgrep_cli_output.csv"

        if csv_path.exists():
            csv_path.unlink()

        cli_result = subprocess.run(
            [
                "cargo",
                "run",
                "-p",
                "pyfastgrep-cli",
                "--",
                "fn",
                "src",
                "--glob",
                "*.rs",
                "--ignore-case",
                "--csv",
                "--output",
                str(csv_path),
            ],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
        )

        assert cli_result.returncode == 0, f"CLI CSV exited with {cli_result.returncode}: {cli_result.stderr}"
        assert cli_result.stdout.startswith("file,line,content"), "CLI CSV output should start with a header"
        assert csv_path.exists(), "CLI CSV should write a file"
        assert csv_path.read_text(encoding="utf-8").startswith("file,line,content"), "CLI CSV file should start with a header"

    def test_legacy_output_and_consistency():
        json_results = pyfastgrep.search("fn", source_root, "*.rs", None, False, True)
        legacy_results = pyfastgrep.search("fn", source_root, "*.rs", None, False, False)

        assert len(legacy_results) > 0, "Expected >0 results for legacy search"
        assert isinstance(legacy_results[0], tuple), "Legacy results should contain tuples"
        assert len(legacy_results[0]) == 3, "Legacy tuples should have 3 elements"
        assert json_results[0]['file'] == legacy_results[0][0], "File paths should match between JSON and legacy"
        assert json_results[0]['line'] == legacy_results[0][1], "Line numbers should match between JSON and legacy"
        assert json_results[0]['content'].strip() == legacy_results[0][2].strip(), "Content should match between JSON and legacy"

    def test_cli_smoke():
        cli_result = subprocess.run(
            [
                "cargo",
                "run",
                "-p",
                "pyfastgrep-cli",
                "--",
                "fn",
                "src",
                "--glob",
                "*.rs",
                "--ignore-case",
            ],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
        )

        assert cli_result.returncode == 0, f"CLI exited with {cli_result.returncode}: {cli_result.stderr}"
        assert os.path.join("src", "lib.rs") in cli_result.stdout, "CLI output should include the Rust source file"

    def test_ergonomic_aliases():
        alias_results = pyfastgrep.search("FN", root=source_root, glob="*.rs", case_insensitive=True, limit=2)
        alias_iter = list(pyfastgrep.search_iter("FN", root=source_root, glob="*.rs", case_insensitive=True))

        assert len(alias_results) > 0, "Alias-based search should find results"
        assert len(alias_iter) > 0, "Alias-based iterator search should find results"
        assert len(alias_results) <= 2, "limit alias should cap the batch results"

    tests = [
        ("Case-sensitive search returns no matches", test_case_sensitive_search),
        ("Ignore-case batch search finds matches", test_ignore_case_search),
        ("Iterator search matches batch count", test_iterator_matches_batch),
        ("JSON output works for batch and iterator", test_json_output),
        ("CSV output works for batch and iterator", test_csv_output),
        ("Legacy tuple output stays compatible", test_legacy_output_and_consistency),
        ("CLI smoke test passes", test_cli_smoke),
        ("CLI CSV output passes", test_cli_csv),
        ("Ergonomic aliases work", test_ergonomic_aliases),
    ]

    passed = 0
    failed = 0

    for name, func in tests:
        if run_test(name, func):
            passed += 1
        else:
            failed += 1

    print("\nTest Summary")
    print(f"Total: {len(tests)}")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")

    if failed:
        sys.exit(1)

    print("All tests passed successfully!")


if __name__ == "__main__":
    main()
