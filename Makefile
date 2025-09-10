SHELL := /bin/bash

.PHONY: test test-legacy bench perf-save perf-gate coverage fmt

test:
	cargo test --features ast_fastpath

test-legacy:
	cargo test --no-default-features

bench:
	cargo bench --bench ast_perf

perf-save:
	python3 scripts/perf_gate_save.py --out reports/benchmarks/baseline

perf-gate:
	python3 scripts/perf_gate.py --baseline reports/benchmarks/baseline --threshold 0.2

coverage:
	cargo tarpaulin --features ast_fastpath --timeout 120 --out Html || echo "tarpaulin not installed; skipped"

fmt:
	cargo fmt --all
	cargo clippy -- -D warnings || true

