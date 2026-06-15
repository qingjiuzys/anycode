//! Coerce common LLM tool-argument mistakes before serde deserialize.

use serde_json::Value;

/// If `value` is a JSON string that parses as JSON, return the parsed value.
fn unwrap_json_string(value: Value) -> Value {
    let Value::String(s) = value else {
        return value;
    };
    let trimmed = s.trim();
    if !(trimmed.starts_with('[') || trimmed.starts_with('{')) {
        return Value::String(s);
    }
    serde_json::from_str(trimmed).unwrap_or(Value::String(s))
}

/// Normalize tool input object: unwrap stringified arrays/objects for known fields.
pub fn coerce_tool_input(tool_name: &str, input: Value) -> Value {
    let Value::Object(mut map) = input else {
        return input;
    };

    let array_fields: &[&str] = match tool_name {
        "TodoWrite" => &["todos"],
        "PlanWrite" => &["tree", "updates"],
        "QueryWeChatHistory" => &["attachment_types"],
        "Grep" => &["paths", "glob"],
        _ => &[],
    };

    for field in array_fields {
        if let Some(v) = map.get(*field).cloned() {
            map.insert((*field).to_string(), unwrap_json_string(v));
        }
    }

    // Bash: some models emit `cmd` instead of `command`
    if tool_name == "Bash" {
        if !map.contains_key("command") {
            if let Some(cmd) = map.get("cmd").and_then(|v| v.as_str()) {
                map.insert("command".to_string(), Value::String(cmd.to_string()));
            }
        }
    }

    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unwraps_stringified_todos_array() {
        let raw = json!({
            "todos": "[{\"id\":\"1\",\"content\":\"x\",\"status\":\"pending\"}]"
        });
        let coerced = coerce_tool_input("TodoWrite", raw);
        assert!(coerced["todos"].is_array());
    }

    #[test]
    fn unwraps_stringified_plan_tree_array() {
        let raw = json!({
            "tree": "[{\"id\":\"1\",\"title\":\"x\"}]"
        });
        let coerced = coerce_tool_input("PlanWrite", raw);
        assert!(coerced["tree"].is_array());
    }

    #[test]
    fn maps_bash_cmd_alias() {
        let raw = json!({ "cmd": "echo hi" });
        let coerced = coerce_tool_input("Bash", raw);
        assert_eq!(coerced["command"], "echo hi");
    }
}
