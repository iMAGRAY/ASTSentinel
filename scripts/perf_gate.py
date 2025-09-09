#!/usr/bin/env python3
"""
Compare current Criterion benchmark results to a saved baseline and fail
if mean time regresses by more than a threshold.
"""
import argparse
import json
import os


def load_estimate(path):
    try:
        with open(path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        # Prefer mean, fallback to median
        mean = data.get('mean', {}).get('point_estimate')
        if mean is None:
            mean = data.get('median', {}).get('point_estimate')
        return float(mean) if mean is not None else None
    except Exception:
        return None


def collect_current(criterion_dir):
    results = {}
    for root, _, files in os.walk(criterion_dir):
        # Only consider estimates under the immediate "new" directory
        if 'estimates.json' in files and os.path.basename(root) == 'new':
            rel = os.path.relpath(os.path.join(root, 'estimates.json'), criterion_dir)
            results[rel] = os.path.join(root, 'estimates.json')
    return results


def main():
    p = argparse.ArgumentParser(description='Perf gate for Criterion benchmarks')
    p.add_argument('--baseline', required=True, help='Path to baseline directory')
    p.add_argument('--criterion-dir', default=os.path.join('target', 'criterion'))
    p.add_argument('--threshold', type=float, default=0.2, help='Allowed regression ratio (e.g., 0.2 = 20%)')
    args = p.parse_args()

    baseline_dir = os.path.normpath(args.baseline)
    crit_dir = os.path.normpath(args.criterion_dir)

    if not os.path.isdir(baseline_dir):
        print(f"Baseline not found: {baseline_dir}. Skipping perf gate.")
        return 0
    if not os.path.isdir(crit_dir):
        print(f"Criterion directory not found: {crit_dir}. Skipping perf gate.")
        return 0

    current = collect_current(crit_dir)
    if not current:
        print("No current benchmark estimates found. Skipping perf gate.")
        return 0

    failures = []
    for rel, cur_path in current.items():
        base_path = os.path.join(baseline_dir, rel)
        cur_val = load_estimate(cur_path)
        base_val = load_estimate(base_path)
        if cur_val is None or base_val is None:
            # Missing or unreadable; skip
            continue
        if cur_val > base_val * (1.0 + args.threshold):
            failures.append((rel, base_val, cur_val))

    if failures:
        print("Performance regressions detected:")
        for rel, base, cur in failures:
            delta = (cur / base) - 1.0
            print(f" - {rel}: baseline={base:.2f}, current={cur:.2f}, regression={delta*100:.1f}%")
        return 1
    else:
        print("No perf regressions above threshold.")
        return 0


if __name__ == '__main__':
    raise SystemExit(main())
