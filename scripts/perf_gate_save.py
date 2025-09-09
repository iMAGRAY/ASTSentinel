#!/usr/bin/env python3
"""
Save Criterion benchmark results as baseline for perf-gate comparisons.

Copies all estimates.json from target/criterion/**/new/ into a baseline directory,
preserving relative paths to allow later comparisons.
"""
import argparse
import os
import shutil


def find_estimates(criterion_dir):
    # Walk criterion tree and pick only estimates under a "new" directory
    for root, _, files in os.walk(criterion_dir):
        if 'estimates.json' in files and os.path.basename(root) == 'new':
            yield os.path.join(root, 'estimates.json')


def main():
    parser = argparse.ArgumentParser(description='Save Criterion benchmarks baseline')
    parser.add_argument('--criterion-dir', default=os.path.join('target', 'criterion'))
    parser.add_argument('--out', default=os.path.join('reports', 'benchmarks', 'baseline'))
    args = parser.parse_args()

    crit = os.path.normpath(args.criterion_dir)
    out = os.path.normpath(args.out)

    if not os.path.isdir(crit):
        print(f"Criterion directory not found: {crit}")
        return 0

    count = 0
    for est in find_estimates(crit):
        rel = os.path.relpath(est, crit)
        # Normalize path under baseline
        dst = os.path.join(out, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(est, dst)
        count += 1
    print(f"Saved {count} estimate file(s) into {out}")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
