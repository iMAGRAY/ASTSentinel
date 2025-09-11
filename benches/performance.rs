use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_validation_hooks::validation::diff_formatter::{
    format_code_diff, format_multi_edit_diff, truncate_for_display,
};

fn benchmark_truncate_for_display(c: &mut Criterion) {
    let mut group = c.benchmark_group("truncate_for_display");

    let long_string = "x".repeat(1000);
    let test_strings = vec![
        ("short", "Hello World"),
        (
            "medium",
            "This is a medium length string that needs to be truncated for display purposes",
        ),
        ("long_ascii", long_string.as_str()),
        (
            "utf8_cyrillic",
            "Это тестовая строка с кириллицей для проверки производительности обрезки",
        ),
        (
            "utf8_emoji",
            "Hello 👋 World 🌍 Test 🚀 Code 💻 Review 📝 Done ✅",
        ),
        ("utf8_mixed", "Test тест 测试 テスト اختبار δοκιμή тест испытание"),
    ];

    for (name, input) in test_strings {
        group.bench_with_input(BenchmarkId::new("50_chars", name), &input, |b, s| {
            b.iter(|| truncate_for_display(black_box(s), 50))
        });
    }

    group.finish();
}

fn benchmark_format_code_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_code_diff");

    let small_old = "line 1\nline 2\nline 3";
    let small_new = "line 1\nmodified line 2\nline 3\nline 4";

    let medium_old = (0..100)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let medium_new = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                format!("Modified line {}", i)
            } else {
                format!("Line {}", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let large_old = (0..1000)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let large_new = (0..1000)
        .map(|i| {
            if i % 50 == 0 {
                format!("Modified line {}", i)
            } else {
                format!("Line {}", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    group.bench_function("small_files", |b| {
        b.iter(|| {
            format_code_diff(
                black_box("test.rs"),
                black_box(Some(small_old)),
                black_box(Some(small_new)),
                black_box(3),
            )
        })
    });

    group.bench_function("medium_files", |b| {
        b.iter(|| {
            format_code_diff(
                black_box("test.rs"),
                black_box(Some(&medium_old)),
                black_box(Some(&medium_new)),
                black_box(3),
            )
        })
    });

    group.bench_function("large_files", |b| {
        b.iter(|| {
            format_code_diff(
                black_box("test.rs"),
                black_box(Some(&large_old)),
                black_box(Some(&large_new)),
                black_box(3),
            )
        })
    });

    group.finish();
}

fn benchmark_multi_edit_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_multi_edit_diff");

    let content = "fn main() {\n    println!(\"Hello, world!\");\n    let x = 42;\n    let y = x * 2;\n    println!(\"Result: {}\", y);\n}";

    let small_edits = vec![("println!".to_string(), "log::info!".to_string())];

    let medium_edits = vec![
        ("println!".to_string(), "log::info!".to_string()),
        ("let x = 42".to_string(), "let x = 100".to_string()),
        ("let y = x * 2".to_string(), "let y = x * 3".to_string()),
    ];

    let large_edits = (0..20)
        .map(|i| (format!("old_{}", i), format!("new_{}", i)))
        .collect::<Vec<_>>();

    group.bench_function("single_edit", |b| {
        b.iter(|| {
            format_multi_edit_diff(
                black_box("test.rs"),
                black_box(Some(content)),
                black_box(&small_edits),
            )
        })
    });

    group.bench_function("multiple_edits", |b| {
        b.iter(|| {
            format_multi_edit_diff(
                black_box("test.rs"),
                black_box(Some(content)),
                black_box(&medium_edits),
            )
        })
    });

    group.bench_function("many_edits", |b| {
        b.iter(|| {
            format_multi_edit_diff(
                black_box("test.rs"),
                black_box(Some(content)),
                black_box(&large_edits),
            )
        })
    });

    group.finish();
}

fn benchmark_large_file_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file_processing");

    // Simulate processing of different file sizes
    let sizes = vec![
        ("1KB", 1_000),
        ("10KB", 10_000),
        ("100KB", 100_000),
        ("1MB", 1_000_000),
    ];

    for (name, size) in sizes {
        let content = "x".repeat(size);

        group.bench_with_input(BenchmarkId::new("truncate", name), &content, |b, s| {
            b.iter(|| truncate_for_display(black_box(s), 100))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_truncate_for_display,
    benchmark_format_code_diff,
    benchmark_multi_edit_diff,
    benchmark_large_file_processing
);
criterion_main!(benches);
