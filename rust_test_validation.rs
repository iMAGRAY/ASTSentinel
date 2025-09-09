// Rust test file to validate ALL AST analysis rules
use std::collections::HashMap;

// Function with too many parameters (should trigger TooManyParameters - 8 > 5)
fn function_with_excessive_params(param1: i32, param2: String, param3: bool, param4: f64, param5: Vec<i32>, param6: HashMap<String, i32>, param7: Option<String>, param8: Result<i32, String>) -> i32 {
    // This function has 8 parameters, exceeding the limit of 5
    param1 + param2.len() as i32 + if param3 { 1 } else { 0 } + param4 as i32
}

// Function with deep nesting (should trigger DeepNesting - 8 levels > 6) 
fn deeply_nested_logic(value: i32) -> String {
    if value > 0 {  // Level 1
        if value > 10 {  // Level 2
            if value > 20 {  // Level 3
                if value > 30 {  // Level 4
                    if value > 40 {  // Level 5
                        if value > 50 {  // Level 6
                            if value > 60 {  // Level 7
                                if value > 70 {  // Level 8 - should trigger warning
                                    return "too deep".to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    "not reached".to_string()
}

// This line is intentionally very long to test the long line detection rule in the AST analyzer - it should definitely exceed 120 characters and trigger LongLine rule
fn test_long_lines() -> String {
    let short_line = "ok";
    short_line.to_string()
}

// Security issues - hardcoded credentials (should trigger HardcodedCredentials)
fn security_problems() -> (String, String, String) {
    let api_key = "sk-1234567890abcdef1234567890abcdef1234567890".to_string();  // OpenAI-style key
    let secret_token = "this_is_a_very_long_secret_that_should_be_detected_as_credential".to_string();
    let password = "admin123password".to_string();
    (api_key, secret_token, password)
}

// Good code - should not trigger any warnings
fn good_clean_function(data: Option<HashMap<String, i32>>) -> Option<String> {
    match data {
        Some(map) => {
            if !map.is_empty() {
                Some("valid data".to_string())
            } else {
                None
            }
        }
        None => None,
    }
}

// Mixed complexity - moderate nesting (should be OK - 5 levels <= 6)
fn moderate_complexity(items: Vec<HashMap<String, Vec<String>>>) -> Vec<String> {
    let mut result = Vec::new();
    for item in items {  // Level 1
        if item.contains_key("active") {  // Level 2
            for (key, values) in item {  // Level 3
                if key == "categories" {  // Level 4
                    for value in values {  // Level 5
                        result.push(value);  // Still within limit
                    }
                }
            }
        }
    }
    result
}

// Using .unwrap() without error handling (should trigger UnhandledError)
fn bad_error_handling(maybe_value: Option<i32>) -> i32 {
    maybe_value.unwrap()  // This should be detected as unsafe
}

// Using panic! (should trigger UnhandledError)
fn panic_usage() {
    panic!("This should be detected");  // This should trigger critical warning
}

fn main() {
    println!("AST validation test file");
}