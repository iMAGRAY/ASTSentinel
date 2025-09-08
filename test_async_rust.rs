use std::collections::HashMap;
use std::path::Path;

pub async fn load_config(path: &Path) -> Result<HashMap<String, String>, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut config = HashMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            config.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(config)
}

pub struct ConfigManager {
    configs: HashMap<String, String>,
}

impl ConfigManager {
    pub async fn new(config_path: &Path) -> Result<Self, std::io::Error> {
        let configs = load_config(config_path).await?;
        Ok(Self { configs })
    }

    pub async fn get_value(&self, key: &str) -> Option<&String> {
        self.configs.get(key)
    }

    pub async fn update_config(
        &mut self,
        key: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.configs.insert(key, value);
        Ok(())
    }
}
