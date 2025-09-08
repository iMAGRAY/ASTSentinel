// Тестовый файл для демонстрации автоформатирования
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
    println!("Hello, world!");
    let x = 42;
    if x > 0 {
        println!("positive");
    }
}

struct User {
    name: String,
    age: u32,
}

impl User {
    fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
}
