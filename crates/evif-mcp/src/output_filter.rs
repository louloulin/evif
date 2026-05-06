// Output Filtering Pipeline for EVIF MCP Server
//
// Applies configurable transformations to MCP tool output to reduce token usage.
// Pipeline: strip_ansi -> truncate_lines -> compact_json -> max_string_length

use regex_lite::Regex;
use serde_json::Value;

/// Output filter configuration.
#[derive(Debug, Clone)]
pub struct OutputFilterConfig {
    pub strip_ansi: bool,
    pub max_lines: usize,
    pub compact_json: bool,
    pub max_string_length: usize,
}

impl Default for OutputFilterConfig {
    fn default() -> Self {
        Self {
            strip_ansi: true,
            max_lines: 200,
            compact_json: true,
            max_string_length: 10000,
        }
    }
}

/// Apply the full output filter pipeline to a string.
pub fn apply_output_filters(content: &str, config: &OutputFilterConfig) -> String {
    let mut result = content.to_string();

    if config.strip_ansi {
        result = strip_ansi_codes(&result);
    }

    if config.max_lines > 0 {
        result = truncate_lines(&result, config.max_lines);
    }

    if config.max_string_length > 0 {
        result = truncate_long_strings(&result, config.max_string_length);
    }

    if config.compact_json {
        result = compact_json_output(&result);
    }

    result
}

/// Strip ANSI escape codes from a string.
pub fn strip_ansi_codes(input: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(input, "").to_string()
}

/// Truncate output to a maximum number of lines.
/// Adds a truncation notice if content is shortened.
pub fn truncate_lines(input: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() <= max_lines {
        return input.to_string();
    }

    let truncated: Vec<&str> = lines[..max_lines].to_vec();
    let omitted = lines.len() - max_lines;
    let mut result = truncated.join("\n");
    result.push_str(&format!("\n[... {} more lines truncated]", omitted));
    result
}

/// Truncate individual long strings (lines) within the output.
pub fn truncate_long_strings(input: &str, max_length: usize) -> String {
    input
        .lines()
        .map(|line| {
            if line.len() > max_length {
                let mut truncated = line[..max_length].to_string();
                truncated.push_str("...[truncated]");
                truncated
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compact JSON output by removing null fields and empty objects.
/// Only processes content that appears to be JSON.
pub fn compact_json_output(input: &str) -> String {
    let trimmed = input.trim();

    // Only try to compact if the content looks like JSON
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return input.to_string();
    }

    // Try to parse as JSON, compact if successful
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let compacted = remove_null_fields(&value);
        // Pretty print compacted JSON
        serde_json::to_string_pretty(&compacted).unwrap_or_else(|_| input.to_string())
    } else {
        // Not valid JSON, return as-is
        input.to_string()
    }
}

/// Recursively remove null fields from a JSON value.
/// Removes null values from both objects and arrays.
fn remove_null_fields(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let cleaned: serde_json::Map<String, Value> = map
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), remove_null_fields(v)))
                .collect();
            Value::Object(cleaned)
        }
        Value::Array(arr) => {
            let cleaned: Vec<Value> = arr
                .iter()
                .filter(|v| !v.is_null())
                .map(remove_null_fields)
                .collect();
            Value::Array(cleaned)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_strip_ansi_codes() {
        let input = "\x1b[32mSuccess\x1b[0m: \x1b[1mbold text\x1b[0m";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "Success: bold text");
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn test_strip_ansi_no_codes() {
        let input = "Plain text without ANSI";
        let result = strip_ansi_codes(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_truncate_lines_within_limit() {
        let input = "line1\nline2\nline3";
        let result = truncate_lines(input, 5);
        assert_eq!(result, input);
    }

    #[test]
    fn test_truncate_lines_exceeds_limit() {
        let input = (1..=10).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n");
        let result = truncate_lines(&input, 3);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
        assert!(!result.contains("line4"));
        assert!(result.contains("[... 7 more lines truncated]"));
    }

    #[test]
    fn test_truncate_long_strings_short() {
        let input = "short line";
        let result = truncate_long_strings(input, 100);
        assert_eq!(result, input);
    }

    #[test]
    fn test_truncate_long_strings_exceeds() {
        let input = "a".repeat(200);
        let result = truncate_long_strings(&input, 50);
        assert!(result.len() < 200);
        assert!(result.ends_with("...[truncated]"));
        assert_eq!(&result[..50], &"a".repeat(50));
    }

    #[test]
    fn test_compact_json_removes_nulls() {
        let input = r#"{"name": "test", "value": null, "items": [1, null, 3]}"#;
        let result = compact_json_output(input);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["name"].is_string());
        assert!(parsed.get("value").is_none());
        assert_eq!(parsed["items"][0], 1);
        assert_eq!(parsed["items"][1], 3);
    }

    #[test]
    fn test_compact_json_non_json_passthrough() {
        let input = "This is not JSON at all";
        let result = compact_json_output(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_remove_null_fields_nested() {
        let input = json!({
            "a": "hello",
            "b": null,
            "c": {
                "d": null,
                "e": "world"
            }
        });
        let result = remove_null_fields(&input);
        assert!(result.get("b").is_none());
        assert!(result["c"].get("d").is_none());
        assert_eq!(result["c"]["e"], "world");
    }

    #[test]
    fn test_apply_output_filters_full_pipeline() {
        let config = OutputFilterConfig {
            strip_ansi: true,
            max_lines: 3,
            compact_json: false,
            max_string_length: 50,
        };
        let input = "\x1b[31mline1\x1b[0m\nline2\nline3\nline4\nline5";
        let result = apply_output_filters(input, &config);
        assert!(!result.contains('\x1b'));
        assert!(result.contains("line1"));
        assert!(result.contains("truncated"));
    }

    #[test]
    fn test_apply_output_filters_disabled() {
        let config = OutputFilterConfig {
            strip_ansi: false,
            max_lines: 0,
            compact_json: false,
            max_string_length: 0,
        };
        let input = "\x1b[31mhello\x1b[0m";
        let result = apply_output_filters(input, &config);
        assert_eq!(result, input); // No filtering applied
    }
}
