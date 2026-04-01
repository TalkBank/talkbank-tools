#!/usr/bin/env python3
"""Diagnose exact divergence between TreeSitter and Re2c parsers.

Reads JSON output from both parsers (dumped by the quick_divergence_check test)
and finds the exact field that differs.

Usage: Run the test first, then this script reads from /tmp.
"""
import json, sys, os

def strip_spans(obj):
    """Remove span fields for comparison."""
    if isinstance(obj, dict):
        return {k: strip_spans(v) for k, v in obj.items() if k != 'span'}
    elif isinstance(obj, list):
        return [strip_spans(x) for x in obj]
    return obj

def find_diff(a, b, path=""):
    """Find the first difference between two JSON structures."""
    if type(a) != type(b):
        return f"{path}: type {type(a).__name__} vs {type(b).__name__}"
    if isinstance(a, dict):
        all_keys = set(list(a.keys()) + list(b.keys()))
        for k in sorted(all_keys):
            if k == 'span':
                continue
            if k not in a:
                return f"{path}.{k}: MISSING in first"
            if k not in b:
                return f"{path}.{k}: MISSING in second"
            d = find_diff(a[k], b[k], f"{path}.{k}")
            if d:
                return d
    elif isinstance(a, list):
        if len(a) != len(b):
            return f"{path}: length {len(a)} vs {len(b)}"
        for i, (x, y) in enumerate(zip(a, b)):
            d = find_diff(x, y, f"{path}[{i}]")
            if d:
                return d
    else:
        if a != b:
            av = repr(a)[:80]
            bv = repr(b)[:80]
            return f"{path}: {av} vs {bv}"
    return None

if __name__ == "__main__":
    ts_path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ts_output.json"
    re2c_path = sys.argv[2] if len(sys.argv) > 2 else "/tmp/re2c_output.json"

    if not os.path.exists(ts_path):
        print(f"No file: {ts_path}")
        sys.exit(1)

    with open(ts_path) as f:
        ts = strip_spans(json.load(f))
    with open(re2c_path) as f:
        re2c = strip_spans(json.load(f))

    diff = find_diff(ts, re2c)
    if diff:
        print(f"DIFF: {diff}")
    else:
        print("IDENTICAL after span stripping")
