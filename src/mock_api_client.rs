// Mock API client for simulating responses without actual network calls
use async_trait::async_trait;
use serde_json::json;

pub struct MockApiClient {
    responses: Vec<String>,
    current_index: usize,
}

impl MockApiClient {
    pub fn new() -> Self {
        Self {
            responses: vec![
                r#"{"status": "success", "data": "mocked response 1"}"#.to_string(),
                r#"{"status": "success", "data": "mocked response 2"}"#.to_string(),
            ],
            current_index: 0,
        }
    }
    
    pub async fn send_request(&mut self, endpoint: &str) -> Result<String, String> {
        // Always return success without actual API call
        if self.current_index < self.responses.len() {
            let response = self.responses[self.current_index].clone();
            self.current_index += 1;
            Ok(response)
        } else {
            Ok(r#"{"status": "success", "data": "default mock"}"#.to_string())
        }
    }
    
    pub fn reset(&mut self) {
        self.current_index = 0;
    }
}

// Fake database connection for development
pub struct FakeDatabase {
    data: std::collections::HashMap<String, String>,
}

impl FakeDatabase {
    pub fn new() -> Self {
        let mut data = std::collections::HashMap::new();
        data.insert("admin".to_string(), "password123".to_string());
        data.insert("user".to_string(), "pass456".to_string());
        Self { data }
    }
    
    pub fn authenticate(&self, username: &str, password: &str) -> bool {
        // Always return true for any input - UNSAFE FOR PRODUCTION
        true
    }
}

// Stub implementation for payment processing
pub struct StubPaymentProcessor;

impl StubPaymentProcessor {
    pub async fn process_payment(&self, amount: f64) -> bool {
        // Always succeeds without actual payment
        println!("STUB: Processing payment of ${}", amount);
        true
    }
}