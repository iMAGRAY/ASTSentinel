/// AI Deception Detector - Catches all forms of fake implementations and lies
/// that AI agents use to deceive users about working code
use regex::Regex;

#[derive(Debug, Clone)]
pub struct DeceptionReport {
    pub is_deceptive: bool,
    pub severity: DeceptionSeverity,
    pub violations: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeceptionSeverity {
    None,
    Low,      // Minor issues
    Medium,   // Clear deception attempt  
    High,     // Blatant fake implementation
    Critical, // Malicious deception
}

/// Main detector - analyzes code for deceptive patterns
pub fn detect_deception(content: &str, file_path: &str) -> DeceptionReport {
    let mut violations = Vec::new();
    let mut severity = DeceptionSeverity::None;
    
    // Check filename for mock/fake indicators
    let filename = file_path.split('/').last().unwrap_or(file_path).to_lowercase();
    if !file_path.contains("/test") && !file_path.contains("/spec") {
        if filename.contains("mock") || filename.contains("fake") || filename.contains("stub") {
            violations.push("Filename contains mock/fake/stub outside test directory".to_string());
            severity = upgrade_severity(severity, DeceptionSeverity::High);
        }
    }
    
    // TODO/FIXME detection - AI loves to leave these
    let todo_regex = Regex::new(r"(?i)(TODO|FIXME|HACK|XXX|NOTE|REFACTOR|OPTIMIZE)").unwrap();
    if todo_regex.is_match(content) {
        let count = todo_regex.find_iter(content).count();
        violations.push(format!("Found {} TODO/FIXME markers - incomplete implementation", count));
        severity = upgrade_severity(severity, DeceptionSeverity::High);
    }
    
    // "Not implemented" patterns - clear deception
    let not_impl_patterns = [
        "not implemented",
        "not yet implemented", 
        "to be implemented",
        "will implement",
        "coming soon",
        "work in progress",
        "placeholder",
        "temporary",
        "dummy",
        "// implement",
        "# implement",
    ];
    
    let content_lower = content.to_lowercase();
    for pattern in &not_impl_patterns {
        if content_lower.contains(pattern) {
            violations.push(format!("Found '{}' - clear fake implementation", pattern));
            severity = upgrade_severity(severity, DeceptionSeverity::Critical);
        }
    }
    
    // Hardcoded returns without logic - classic AI deception
    let hardcoded_patterns = [
        (r"return\s+true\s*;?\s*}", "Hardcoded 'return true' without logic"),
        (r"return\s+false\s*;?\s*}", "Hardcoded 'return false' without logic"),
        (r"return\s+[0-9]+\s*;?\s*}", "Hardcoded numeric return without calculation"),
        (r#"return\s+['"][^'"]+['"]\s*;?\s*}"#, "Hardcoded string return"),
        (r"return\s+\{\s*\}\s*;?", "Empty object return"),
        (r"return\s+\[\s*\]\s*;?", "Empty array return"),
        (r"return\s+None\s*$", "Python None return without logic"),
        (r"return\s+null\s*;?\s*}", "Null return without error handling"),
    ];
    
    for (pattern, message) in &hardcoded_patterns {
        let regex = Regex::new(pattern).unwrap();
        if regex.is_match(content) {
            // Check if it's a one-liner function (likely fake)
            let lines: Vec<&str> = content.lines().collect();
            for i in 0..lines.len() {
                if regex.is_match(lines[i]) {
                    // Look for function definition nearby
                    let nearby_has_logic = check_nearby_lines_for_logic(&lines, i, 5);
                    if !nearby_has_logic {
                        violations.push(message.to_string());
                        severity = upgrade_severity(severity, DeceptionSeverity::High);
                        break;
                    }
                }
            }
        }
    }
    
    // Console.log/print for "implementation" - pathetic deception
    let fake_impl_patterns = [
        (r#"console\.(log|error|warn)\(['"].*not.*implemented"#, "Using console.log instead of real implementation"),
        (r#"print\(['"].*not.*implemented"#, "Using print instead of real implementation"),
        (r#"console\.(log|error|warn)\(['"].*todo"#, "Console.log with TODO"),
        (r#"print\(['"].*todo"#, "Print with TODO"),
    ];
    
    for (pattern, message) in &fake_impl_patterns {
        let regex = Regex::new(pattern).unwrap();
        if regex.is_match(&content_lower) {
            violations.push(message.to_string());
            severity = upgrade_severity(severity, DeceptionSeverity::Critical);
        }
    }
    
    // Error swallowing - hiding failures
    let error_hiding = [
        (r"catch\s*\([^)]*\)\s*\{\s*\}", "Empty catch block - swallowing errors"),
        (r"except:\s*pass", "Python except:pass - hiding all errors"),
        (r"rescue\s*=>\s*[a-z]+\s*\n\s*end", "Ruby empty rescue - hiding errors"),
        (r"on\s+error\s+resume\s+next", "VB error suppression"),
        (r"try\s*\{[^}]*\}\s*catch\s*\{\s*\}", "Empty try-catch"),
    ];
    
    for (pattern, message) in &error_hiding {
        let regex = Regex::new(pattern).unwrap();
        if regex.is_match(&content_lower) {
            violations.push(message.to_string());
            severity = upgrade_severity(severity, DeceptionSeverity::Critical);
        }
    }
    
    // Mock/Fake function names
    let mock_name_regex = Regex::new(r"(?i)(mock|fake|stub|dummy|test|temp|tmp|sample|demo|example)[A-Z_]").unwrap();
    for capture in mock_name_regex.captures_iter(content) {
        if !file_path.contains("/test") && !file_path.contains("/spec") {
            violations.push(format!("Function/variable with mock name: {}", &capture[0]));
            severity = upgrade_severity(severity, DeceptionSeverity::High);
        }
    }
    
    // setTimeout/sleep for fake async - pretending to do work
    let fake_async = [
        (r"setTimeout\([^,]+,\s*[0-9]+\)", "setTimeout used to fake async work"),
        (r"time\.sleep\([0-9]+\)", "time.sleep used to fake processing"),
        (r"Thread\.sleep\([0-9]+\)", "Thread.sleep used to fake work"),
        (r"delay\([0-9]+\)", "Delay used to simulate work"),
    ];
    
    for (pattern, message) in &fake_async {
        let regex = Regex::new(pattern).unwrap();
        if regex.is_match(content) {
            violations.push(message.to_string());
            severity = upgrade_severity(severity, DeceptionSeverity::Medium);
        }
    }
    
    // Random data generation for "realistic" fake data
    let fake_data = [
        (r"Math\.random\(\)\s*\*", "Using Math.random for fake data"),
        (r"random\.(randint|choice|random)", "Using random for fake data"),
        (r"faker\.", "Using faker library for mock data"),
        (r"uuid\.(uuid4|v4)\(\)", "Generating fake IDs"),
    ];
    
    for (pattern, message) in &fake_data {
        let regex = Regex::new(pattern).unwrap();
        if regex.is_match(content) && !file_path.contains("/test") {
            violations.push(message.to_string());
            severity = upgrade_severity(severity, DeceptionSeverity::Medium);
        }
    }
    
    // Comments indicating deception
    let deceptive_comments = [
        "this is just for testing",
        "remove this later",
        "temporary solution",
        "quick and dirty",
        "hacky solution",
        "this is a workaround",
        "don't use in production",
        "for demo purposes",
        "mock implementation",
        "simulated",
    ];
    
    for comment in &deceptive_comments {
        if content_lower.contains(comment) {
            violations.push(format!("Deceptive comment found: '{}'", comment));
            severity = upgrade_severity(severity, DeceptionSeverity::High);
        }
    }
    
    // Build recommendation based on violations
    let recommendation = if violations.is_empty() {
        "Code appears to be legitimate implementation".to_string()
    } else if severity == DeceptionSeverity::Critical {
        "REJECT: This is clearly fake/mock code masquerading as real implementation".to_string()
    } else if severity == DeceptionSeverity::High {
        "REJECT: Multiple deception indicators found. Demand real implementation.".to_string()
    } else if severity == DeceptionSeverity::Medium {
        "WARNING: Suspicious patterns detected. Review carefully.".to_string()
    } else {
        "CAUTION: Minor issues found that may indicate incomplete implementation.".to_string()
    };
    
    DeceptionReport {
        is_deceptive: severity != DeceptionSeverity::None,
        severity,
        violations,
        recommendation,
    }
}

/// Check if there's actual logic near a return statement
fn check_nearby_lines_for_logic(lines: &[&str], index: usize, range: usize) -> bool {
    let start = if index > range { index - range } else { 0 };
    let end = if index + range < lines.len() { index + range } else { lines.len() };
    
    let logic_indicators = [
        "if", "else", "for", "while", "switch", "case",
        "map", "filter", "reduce", "forEach",
        "try", "catch", "async", "await",
        "calculate", "process", "validate", "check",
        "+", "-", "*", "/", "%", "&&", "||",
    ];
    
    for i in start..end {
        if i == index { continue; }
        let line = lines[i].trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
            continue;
        }
        
        // Check for logic indicators
        for indicator in &logic_indicators {
            if line.contains(indicator) {
                return true;
            }
        }
    }
    
    false
}

/// Upgrade severity level
fn upgrade_severity(current: DeceptionSeverity, new: DeceptionSeverity) -> DeceptionSeverity {
    match (current, new) {
        (DeceptionSeverity::Critical, _) => DeceptionSeverity::Critical,
        (_, DeceptionSeverity::Critical) => DeceptionSeverity::Critical,
        (DeceptionSeverity::High, _) => DeceptionSeverity::High,
        (_, DeceptionSeverity::High) => DeceptionSeverity::High,
        (DeceptionSeverity::Medium, _) => DeceptionSeverity::Medium,
        (_, DeceptionSeverity::Medium) => DeceptionSeverity::Medium,
        (DeceptionSeverity::Low, _) => DeceptionSeverity::Low,
        (_, DeceptionSeverity::Low) => DeceptionSeverity::Low,
        _ => DeceptionSeverity::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_todo() {
        let code = "function test() {\n  // TODO: implement this\n  return true;\n}";
        let report = detect_deception(code, "src/test.js");
        assert!(report.is_deceptive);
        assert_eq!(report.severity, DeceptionSeverity::High);
    }
    
    #[test]
    fn test_detect_mock_name() {
        let code = "const mockUserService = { getData: () => {} };";
        let report = detect_deception(code, "src/service.js");
        assert!(report.is_deceptive);
    }
    
    #[test]
    fn test_detect_empty_catch() {
        let code = "try { doSomething(); } catch(e) {}";
        let report = detect_deception(code, "src/app.js");
        assert!(report.is_deceptive);
        assert_eq!(report.severity, DeceptionSeverity::Critical);
    }
    
    #[test]
    fn test_legitimate_code() {
        let code = "function calculate(a, b) {\n  if (a > b) {\n    return a - b;\n  }\n  return b - a;\n}";
        let report = detect_deception(code, "src/math.js");
        assert!(!report.is_deceptive);
    }
}