use rust_validation_hooks::smart_deception_detector::detect_deception;

fn main() {
    println!("=== Testing Smart Deception Detector ===\n");
    
    // Test case 1: Spaced TODO
    let code1 = r#"
function test() {
  // T O D O: implement this
  // N O T  I M P L E M E N T E D
  return true;
}"#;
    
    let report1 = detect_deception(code1, "test.js");
    println!("Test 1 - Spaced TODO:");
    println!("  Is deceptive: {}", report1.is_deceptive);
    println!("  Severity: {:?}", report1.severity);
    println!("  Violations: {} found", report1.violations.len());
    for v in &report1.violations {
        println!("    - {} (confidence: {})", v.message, v.confidence);
    }
    println!("  Recommendation: {}", report1.recommendation);
    println!();
    
    // Test case 2: Obfuscated TODO
    let code2 = r#"
function test() {
  // T0D0: implement this
  // T.O.D.O: add logic
  return true;
}"#;
    
    let report2 = detect_deception(code2, "test.js");
    println!("Test 2 - Obfuscated TODO:");
    println!("  Is deceptive: {}", report2.is_deceptive);
    println!("  Severity: {:?}", report2.severity);
    println!("  Violations: {} found", report2.violations.len());
    for v in &report2.violations {
        println!("    - {} (confidence: {})", v.message, v.confidence);
    }
    println!("  Recommendation: {}", report2.recommendation);
    println!();
    
    // Test case 3: Simulation near TODO
    let code3 = r#"
function process() {
  // TODO(#123): integrate with API
  await delay(1000);
  return { id: Math.random() };
}"#;
    
    let report3 = detect_deception(code3, "test.js");
    println!("Test 3 - Simulation near TODO:");
    println!("  Is deceptive: {}", report3.is_deceptive);
    println!("  Severity: {:?}", report3.severity);
    println!("  Violations: {} found", report3.violations.len());
    for v in &report3.violations {
        println!("    - {} (confidence: {})", v.message, v.confidence);
    }
    println!("  Recommendation: {}", report3.recommendation);
    println!();
    
    // Test case 4: Legitimate validator
    let code4 = r#"
function isValidEmail(email) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

function hasPermission(user, action) {
  if (!user || !user.permissions) {
    return false;
  }
  return user.permissions.includes(action);
}"#;
    
    let report4 = detect_deception(code4, "validators.js");
    println!("Test 4 - Legitimate validators:");
    println!("  Is deceptive: {}", report4.is_deceptive);
    println!("  Severity: {:?}", report4.severity);
    println!("  Violations: {} found", report4.violations.len());
    for v in &report4.violations {
        println!("    - {} (confidence: {})", v.message, v.confidence);
    }
    println!("  Recommendation: {}", report4.recommendation);
    println!();
    
    // Test case 5: Test data in production
    let code5 = r#"
const testUser = {
  email: "user@test.com",
  password: "password123"
};

const testMode = true;
"#;
    
    let report5 = detect_deception(code5, "config.js");
    println!("Test 5 - Test data in production:");
    println!("  Is deceptive: {}", report5.is_deceptive);
    println!("  Severity: {:?}", report5.severity);
    println!("  Violations: {} found", report5.violations.len());
    for v in &report5.violations {
        println!("    - {} (confidence: {})", v.message, v.confidence);
    }
    println!("  Recommendation: {}", report5.recommendation);
    
    // Test case 6: String literals with suspicious patterns (should NOT be flagged)
    let code6 = r#"
describe('Error handling', () => {
  const errorMessage = "Feature not implemented yet";
  const mockData = 'Using mock data for testing';
  const template = `
    TODO: Add implementation
    Status: fake data for demo
  `;
  
  expect(errorMessage).toContain("not implemented");
});

class ErrorHandler {
  constructor() {
    this.messages = new Map([
      ['NOT_IMPL', "Feature not implemented"],
      ['MOCK_MODE', 'Running in mock mode']
    ]);
  }
}
"#;
    
    let report6 = detect_deception(code6, "test_strings.js");
    println!("Test 6 - String literals with suspicious patterns:");
    println!("  Is deceptive: {}", report6.is_deceptive);
    println!("  Severity: {:?}", report6.severity);
    println!("  Violations: {} found", report6.violations.len());
    for v in &report6.violations {
        println!("    - {} (confidence: {})", v.message, v.confidence);
    }
    println!("  Recommendation: {}", report6.recommendation);
    println!();

    // ДИАГНОСТИЧЕСКИЙ ТЕСТ - простой template literal
    println!("\n=== ДИАГНОСТИЧЕСКИЙ ТЕСТ ===");
    let diagnostic_code = r#"const template = `
    TODO: Add implementation
    Status: fake data for demo
`;"#;
    
    println!("Диагностический код:");
    for (i, line) in diagnostic_code.lines().enumerate() {
        println!("  Строка {}: '{}'", i, line);
        if line.contains("TODO") || line.contains("fake") {
            println!("    ^ Подозрительные паттерны найдены");
        }
    }
    
    let diagnostic_report = detect_deception(diagnostic_code, "diagnostic.js");
    println!("\nРезультат диагностики:");
    println!("  Обманчивый: {}", diagnostic_report.is_deceptive);
    println!("  Нарушений: {}", diagnostic_report.violations.len());
    for v in &diagnostic_report.violations {
        println!("    - {} (строка {:?})", v.message, v.line_number);
    }

    println!("\n=== Test Summary ===");
    println!("✓ Spaced TODO detected: {}", report1.is_deceptive);
    println!("✓ Obfuscated TODO detected: {}", report2.is_deceptive);
    println!("✓ Simulation near TODO detected: {}", report3.is_deceptive);
    println!("✓ Legitimate validators NOT blocked: {}", !report4.is_deceptive);
    println!("✓ Test data in production detected: {}", report5.is_deceptive);
    println!("✓ String literals with suspicious patterns NOT blocked: {}", !report6.is_deceptive);
    println!("✓ Diagnostic test (simple template literal): {}", !diagnostic_report.is_deceptive);
}