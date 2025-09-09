use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_validation_hooks::analysis::ast::{AstQualityScorer, SupportedLanguage};
use std::fs;

fn bench_language(c: &mut Criterion, name: &str, path: &str, lang: SupportedLanguage) {
    let content = fs::read_to_string(path).expect(&format!("Failed to read {}", path));
    let size = content.len() as u64;
    let mut group = c.benchmark_group(format!("ast_quality_{}", name));
    group.throughput(Throughput::Bytes(size));

    group.bench_with_input(BenchmarkId::new("score", name), &content, |b, code| {
        b.iter(|| {
            let scorer = AstQualityScorer::new();
            let _ = scorer.analyze(code, lang).unwrap();
        })
    });

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_language(
        c,
        "python_small",
        "test_data/test_clean.py",
        SupportedLanguage::Python,
    );
    bench_language(
        c,
        "js_small",
        "test_data/test_ast.js",
        SupportedLanguage::JavaScript,
    );
}

fn bench_more_languages(c: &mut Criterion) {
    bench_language(
        c,
        "java_small",
        "test_data/sample_java.java",
        SupportedLanguage::Java,
    );
    bench_language(
        c,
        "cs_small",
        "test_data/sample_cs.cs",
        SupportedLanguage::CSharp,
    );
    bench_language(
        c,
        "go_small",
        "test_data/sample_go.go",
        SupportedLanguage::Go,
    );
    bench_language(c, "c_small", "test_data/sample_c.c", SupportedLanguage::C);
    bench_language(
        c,
        "cpp_small",
        "test_data/sample_cpp.cpp",
        SupportedLanguage::Cpp,
    );
    bench_language(
        c,
        "php_small",
        "test_data/sample_php.php",
        SupportedLanguage::Php,
    );
    bench_language(
        c,
        "ruby_small",
        "test_data/sample_ruby.rb",
        SupportedLanguage::Ruby,
    );
}

criterion_group!(name = ast_perf; config = Criterion::default(); targets = criterion_benchmark, bench_more_languages);
criterion_main!(ast_perf);
