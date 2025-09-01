use rust_validation_hooks::smart_deception_detector::detect_deception;

fn main() {
    // Простой тест template literal
    let test_code = r#"const template = `
    TODO: Add implementation
    Status: fake data for demo
`;"#;

    println!("=== Диагностический тест Template Literal ===");
    println!("Код для тестирования:");
    println!("{}", test_code);
    println!("\nАнализ построчно:");
    
    let lines: Vec<&str> = test_code.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        println!("Строка {}: '{}'", i, line);
        if line.contains("TODO") {
            println!("  ^ Содержит TODO");
        }
        if line.contains("fake") {
            println!("  ^ Содержит fake");
        }
    }
    
    println!("\nРезультат анализа детектора:");
    let report = detect_deception(test_code, "template_test.js");
    println!("Обманчивый код: {}", report.is_deceptive);
    println!("Найдено нарушений: {}", report.violations.len());
    
    for violation in &report.violations {
        println!("- {} (строка: {:?})", violation.message, violation.line_number);
    }
    
    // Тест 2: Простой строковый литерал
    let simple_string = r#"const msg = "Feature not implemented yet";"#;
    
    println!("\n=== Тест простого строкового литерала ===");
    println!("Код: {}", simple_string);
    
    let report2 = detect_deception(simple_string, "string_test.js");
    println!("Обманчивый код: {}", report2.is_deceptive);
    println!("Найдено нарушений: {}", report2.violations.len());
    
    for violation in &report2.violations {
        println!("- {}", violation.message);
    }
}