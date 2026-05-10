from .pyfastgrep import search as _search
from .pyfastgrep import search_iter as _search_iter
from .pyfastgrep import search_functions as _search_functions
from .pyfastgrep import search_classes as _search_classes
from .pyfastgrep import search_imports as _search_imports

def search(pattern, path=".", glob=None, max_results=None):
    return _search(pattern, path, glob, max_results)

def search_iter(pattern, path=".", glob=None):
    return _search_iter(pattern, path, glob)

def search_functions(target_name, path=".", glob=None):
    return _search_functions(target_name, path, glob)

def search_classes(target_name, path=".", glob=None):
    return _search_classes(target_name, path, glob)

def search_imports(target_name, path=".", glob=None):
    return _search_imports(target_name, path, glob)