use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct DataProcessor {
    data: HashMap<String, Vec<i32>>,
    cache: HashSet<String>,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            cache: HashSet::new(),
        }
    }

    pub async fn process_data(&mut self, key: &str, values: Vec<i32>) -> Result<Vec<i32>, String> {
        if self.cache.contains(key) {
            return Ok(self.data.get(key).unwrap_or(&vec![]).clone());
        }

        let mut processed = Vec::new();
        for value in values {
            if value > 0 {
                processed.push(value * 2);
            } else {
                processed.push(value);
            }
        }

        self.data.insert(key.to_string(), processed.clone());
        self.cache.insert(key.to_string());
        Ok(processed)
    }

    pub fn get_stats(&self) -> (usize, usize) {
        (self.data.len(), self.cache.len())
    }
}
