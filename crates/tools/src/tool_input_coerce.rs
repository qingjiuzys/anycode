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

/// When argument JSON failed to parse upstream, providers may wrap it as `{"raw": "..."}`.
fn unwrap_raw_object(input: Value) -> Value {
    let Value::Object(map) = input else {
        return input;
    };
    if map.len() != 1 {
        return Value::Object(map);
    }
    let Some(Value::String(raw)) = map.get("raw") else {
        return Value::Object(map);
    };
    let trimmed = raw.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
            return parsed;
        }
    }
    Value::Object(map)
}

fn alias_field(map: &mut serde_json::Map<String, Value>, from: &str, to: &str) {
    if map.contains_key(to) {
        return;
    }
    if let Some(v) = map.get(from).cloned() {
        map.insert(to.to_string(), v);
    }
}

/// Normalize tool input object: unwrap stringified arrays/objects for known fields.
pub fn coerce_tool_input(tool_name: &str, input: Value) -> Value {
    let input = unwrap_raw_object(input);
    let Value::Object(mut map) = input else {
        return input;
    };

    let array_fields: &[&str] = match tool_name {
        "TodoWrite" => &["todos"],
        "PlanWrite" => &["tree", "updates"],
        "Grep" => &["paths", "glob"],
        "Skill" => &["args"],
        _ => &[],
    };

    for field in array_fields {
        if let Some(v) = map.get(*field).cloned() {
            map.insert((*field).to_string(), unwrap_json_string(v));
        }
    }

    if tool_name == "Bash" {
        if !map.contains_key("command") {
            if let Some(cmd) = map.get("cmd").and_then(|v| v.as_str()) {
                map.insert("command".to_string(), Value::String(cmd.to_string()));
            }
        }
    }

    if tool_name == "Glob" {
        alias_field(&mut map, "glob_pattern", "pattern");
        alias_field(&mut map, "glob", "pattern");
    }

    if tool_name == "Skill" {
        alias_field(&mut map, "skill", "name");
        alias_field(&mut map, "skill_id", "name");
        alias_field(&mut map, "id", "name");
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

    #[test]
    fn unwraps_skill_args_string_array() {
        let raw = json!({
            "name": "office-pptx",
            "args": "[\"/tmp/outline.md\", \"/tmp/out.pptx\"]"
        });
        let coerced = coerce_tool_input("Skill", raw);
        assert!(coerced["args"].is_array());
        assert_eq!(coerced["args"][0], "/tmp/outline.md");
    }

    #[test]
    fn maps_glob_pattern_alias() {
        let raw = json!({ "glob_pattern": "**/*.rs" });
        let coerced = coerce_tool_input("Glob", raw);
        assert_eq!(coerced["pattern"], "**/*.rs");
    }

    #[test]
    fn maps_skill_name_aliases() {
        let raw = json!({ "skill": "office-pptx" });
        let coerced = coerce_tool_input("Skill", raw);
        assert_eq!(coerced["name"], "office-pptx");
    }

    #[test]
    fn unwraps_raw_wrapper_object() {
        let raw = json!({
            "raw": "{\"name\":\"office-pptx\",\"args\":[\"a.md\",\"b.pptx\"]}"
        });
        let coerced = coerce_tool_input("Skill", raw);
        assert_eq!(coerced["name"], "office-pptx");
        assert!(coerced["args"].is_array());
    }
}
