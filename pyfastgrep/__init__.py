from .pyfastgrep import search as _search
from .pyfastgrep import search_iter as _search_iter

def _normalize_search_options(path, glob, max_results, ignore_case, json, kwargs):
    if "root" in kwargs:
        path = kwargs.pop("root")
    if "limit" in kwargs:
        max_results = kwargs.pop("limit")
    if "case_insensitive" in kwargs:
        ignore_case = kwargs.pop("case_insensitive")
    if "as_json" in kwargs:
        json = kwargs.pop("as_json")

    if kwargs:
        unexpected = ", ".join(sorted(kwargs.keys()))
        raise TypeError(f"unexpected keyword argument(s): {unexpected}")

    return path, glob, max_results, ignore_case, json


def search(pattern, path=".", glob=None, max_results=None, ignore_case=False, json=False, **kwargs):
    """
    Search for a pattern in files.
    
    Args:
        pattern: Regex pattern to search for
        path: Root directory to search in (default: ".")
        glob: File pattern to match (default: None)
        max_results: Maximum number of results to return (default: None)
        ignore_case: Case insensitive search (default: False)
        json: Return results as JSON objects (default: False)
    
    Returns:
        List of tuples (file, line, content) or JSON objects if json=True
    """
    path, glob, max_results, ignore_case, json = _normalize_search_options(
        path, glob, max_results, ignore_case, json, kwargs
    )
    return _search(pattern, path, glob, max_results, ignore_case, json)

def search_iter(pattern, path=".", glob=None, ignore_case=False, json=False, **kwargs):
    """
    Streaming iterator search for a pattern in files.
    
    Args:
        pattern: Regex pattern to search for
        path: Root directory to search in (default: ".")
        glob: File pattern to match (default: None)
        ignore_case: Case insensitive search (default: False)
        json: Return results as JSON objects (default: False)
    
    Returns:
        Iterator yielding tuples (file, line, content) or JSON objects if json=True
    """
    path, glob, _, ignore_case, json = _normalize_search_options(
        path, glob, None, ignore_case, json, kwargs
    )
    return _search_iter(pattern, path, glob, ignore_case, json)