#!/usr/bin/env python3
"""
Produce a compact summary of Criterion baseline means.
Outputs JSON mapping relative estimate path -> mean point estimate.
"""
import argparse
import json
import os


def load_estimate(path: str):
    try:
        with open(path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        mean = data.get('mean', {}).get('point_estimate')
        if mean is None:
            mean = data.get('median', {}).get('point_estimate')
        return float(mean) if mean is not None else None
    except Exception:
        return None


def main():
    p = argparse.ArgumentParser(description='Summarize Criterion baseline estimates')
    p.add_argument('--baseline', default=os.path.join('reports', 'benchmarks', 'baseline'))
    args = p.parse_args()

    base = os.path.normpath(args.baseline)
    out = {}
    for root, _, files in os.walk(base):
        if 'estimates.json' in files and os.path.basename(root) == 'new':
            rel = os.path.relpath(os.path.join(root, 'estimates.json'), base)
            val = load_estimate(os.path.join(root, 'estimates.json'))
            if val is not None:
                out[rel] = val
    print(json.dumps(out, indent=2, ensure_ascii=False))


if __name__ == '__main__':
    raise SystemExit(main())

