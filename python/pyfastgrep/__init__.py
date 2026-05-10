from .pyfastgrep import search as _search
from .pyfastgrep import search_iter as _search_iter

def search(pattern, path=".", glob=None, max_results=None, ignore_case=False):
    """
    Search for a pattern in files.
    """
    return _search(pattern, path, glob, max_results, ignore_case)

def search_iter(pattern, path=".", glob=None, ignore_case=False):
    """
    Streaming iterator search for a pattern in files.
    """
    return _search_iter(pattern, path, glob, ignore_case)