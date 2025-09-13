use jsonschema::JSONSchema;
use serde_json::Value;

/// Validate agent JSON string against internal schema.
/// Returns Ok(()) if valid, Err(message) if invalid or parse error.
pub fn validate_agent_json(agent: &str) -> Result<(), String> {
    // Minimal schema for agent JSON
    let schema_str = r#"
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["schema_version", "quality", "risk_summary"],
  "properties": {
    "schema_version": {"type": "string"},
    "quality": {"type": "object"},
    "risk_summary": {"type": "array"}
  }
}
"#;

    let schema: Value = serde_json::from_str(schema_str).map_err(|e| format!("schema parse error: {}", e))?;
    let instance: Value = serde_json::from_str(agent).map_err(|e| format!("agent json parse error: {}", e))?;
    let compiled = JSONSchema::compile(&schema).map_err(|e| format!("schema compile error: {}", e))?;
    let result = compiled.validate(&instance);
    match result {
        Ok(_) => Ok(()),
        Err(errors) => {
            let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            Err(msgs.join("; "))
        }
    }
}

