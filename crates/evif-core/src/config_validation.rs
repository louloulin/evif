// Plugin Configuration Validation
//
// 对标 AGFS config/validation.go (231行)
// 提供类型安全的配置验证和参数获取

use crate::error::{EvifError, EvifResult};
use serde_json::Value;
use std::collections::HashMap;

/// 配置参数类型
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigParamType {
    String,
    Bool,
    Int,
    Float64,
    Size,   // 支持 "512KB", "1MB", "2GB"
    StringList,
}

/// 配置参数元数据
#[derive(Debug, Clone)]
pub struct ConfigParameter {
    pub name: String,
    pub param_type: ConfigParamType,
    pub required: bool,
    pub default: Option<String>,
    pub description: String,
}

impl ConfigParameter {
    /// 创建新的配置参数
    pub fn new(
        name: impl Into<String>,
        param_type: ConfigParamType,
        required: bool,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            param_type,
            required,
            default: None,
            description: description.into(),
        }
    }

    /// 设置默认值
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// 配置验证工具
///
/// 对标 AGFS validation.go
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证配置只包含已知键
    ///
    /// # AGFS 对标
    /// ```go
    /// func ValidateOnlyKnownKeys(config map[string]interface{}, knownKeys []string) error
    /// ```
    pub fn validate_only_known_keys(
        config: &HashMap<String, Value>,
        known_keys: &[&str],
    ) -> EvifResult<()> {
        let known_set: std::collections::HashSet<&str> = known_keys.iter().copied().collect();

        for key in config.keys() {
            if !known_set.contains(key.as_str()) {
                return Err(EvifError::Configuration(format!(
                    "Unknown configuration key: {}",
                    key
                )));
            }
        }

        Ok(())
    }

    /// 验证必需字段存在
    ///
    /// # AGFS 对标
    /// ```go
    /// func RequireString(config map[string]interface{}, key string) (string, error)
    /// ```
    pub fn require_string(config: &HashMap<String, Value>, key: &str) -> EvifResult<String> {
        match config.get(key) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(_) => Err(EvifError::Configuration(format!(
                "{} must be a string",
                key
            ))),
            None => Err(EvifError::Configuration(format!(
                "Missing required field: {}",
                key
            ))),
        }
    }

    /// 验证必需整数字段
    ///
    /// # AGFS 对标
    /// ```go
    /// func RequireInt(config map[string]interface{}, key string) (int, error)
    /// ```
    pub fn require_int(config: &HashMap<String, Value>, key: &str) -> EvifResult<i64> {
        match config.get(key) {
            Some(Value::Number(n)) => {
                n.as_i64().ok_or_else(|| {
                    EvifError::Configuration(format!("{} must be an integer", key))
                })
            }
            Some(_) => Err(EvifError::Configuration(format!(
                "{} must be an integer",
                key
            ))),
            None => Err(EvifError::Configuration(format!(
                "Missing required field: {}",
                key
            ))),
        }
    }

    /// 验证必需布尔字段
    ///
    /// # AGFS 对标
    /// ```go
    /// func RequireBool(config map[string]interface{}, key string) (bool, error)
    /// ```
    pub fn require_bool(config: &HashMap<String, Value>, key: &str) -> EvifResult<bool> {
        match config.get(key) {
            Some(Value::Bool(b)) => Ok(*b),
            Some(_) => Err(EvifError::Configuration(format!(
                "{} must be a boolean",
                key
            ))),
            None => Err(EvifError::Configuration(format!(
                "Missing required field: {}",
                key
            ))),
        }
    }

    /// 获取字符串字段（可选）
    ///
    /// # AGFS 对标
    /// ```go
    /// func GetString(config map[string]interface{}, key string) (string, bool)
    /// ```
    pub fn get_string(config: &HashMap<String, Value>, key: &str) -> Option<String> {
        match config.get(key) {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// 获取整数字段（可选）
    pub fn get_int(config: &HashMap<String, Value>, key: &str) -> Option<i64> {
        match config.get(key) {
            Some(Value::Number(n)) => n.as_i64(),
            _ => None,
        }
    }

    /// 获取布尔字段（可选）
    pub fn get_bool(config: &HashMap<String, Value>, key: &str) -> Option<bool> {
        match config.get(key) {
            Some(Value::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// 验证字符串类型
    ///
    /// # AGFS 对标
    /// ```go
    /// func ValidateStringType(config map[string]interface{}, key string) error
    /// ```
    pub fn validate_string_type(config: &HashMap<String, Value>, key: &str) -> EvifResult<()> {
        match config.get(key) {
            Some(Value::String(_)) => Ok(()),
            Some(_) => Err(EvifError::Configuration(format!(
                "{} must be a string",
                key
            ))),
            None => Ok(()), // 可选字段，不存在也OK
        }
    }

    /// 验证整数类型
    pub fn validate_int_type(config: &HashMap<String, Value>, key: &str) -> EvifResult<()> {
        match config.get(key) {
            Some(Value::Number(n)) if n.is_i64() => Ok(()),
            Some(_) => Err(EvifError::Configuration(format!(
                "{} must be an integer",
                key
            ))),
            None => Ok(()),
        }
    }

    /// 验证布尔类型
    ///
    /// # AGFS 对标
    /// ```go
    /// func ValidateBoolType(config map[string]interface{}, key string) error
    /// ```
    pub fn validate_bool_type(config: &HashMap<String, Value>, key: &str) -> EvifResult<()> {
        match config.get(key) {
            Some(Value::Bool(_)) => Ok(()),
            Some(_) => Err(EvifError::Configuration(format!(
                "{} must be a boolean",
                key
            ))),
            None => Ok(()),
        }
    }

    /// 解析大小字符串（带单位）
    ///
    /// 支持格式: "512KB", "1MB", "2GB", "3TB"
    ///
    /// # AGFS 对标
    /// ```go
    /// func ParseSize(sizeStr string) (int64, error)
    /// ```
    pub fn parse_size(size_str: &str) -> EvifResult<i64> {
        let size_str = size_str.trim().to_uppercase();

        // 纯数字视为字节
        if size_str.chars().all(|c| c.is_ascii_digit()) {
            return size_str
                .parse::<i64>()
                .map_err(|_| EvifError::Configuration(format!("Invalid size number: {}", size_str)));
        }

        // 找到数字和单位的分界点
        let split_idx = size_str
            .find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| {
                EvifError::Configuration(format!("Invalid size format: {}", size_str))
            })?;

        let number_str = &size_str[..split_idx];
        let unit = &size_str[split_idx..];

        let number: i64 = number_str
            .parse()
            .map_err(|_| {
                EvifError::Configuration(format!("Invalid size number: {}", number_str))
            })?;

        let multiplier = match unit {
            "B" | "BYTE" | "BYTES" => 1,
            "KB" | "K" => 1024,
            "MB" | "M" => 1024 * 1024,
            "GB" | "G" => 1024 * 1024 * 1024,
            "TB" | "T" => 1024 * 1024 * 1024 * 1024,
            "PB" | "P" => 1024 * 1024 * 1024 * 1024 * 1024,
            _ => {
                return Err(EvifError::Configuration(format!(
                    "Unknown size unit: {}",
                    unit
                )))
            }
        };

        Ok(number * multiplier)
    }

    /// 验证并解析大小字段
    pub fn parse_size_field(
        config: &HashMap<String, Value>,
        key: &str,
    ) -> EvifResult<i64> {
        let size_str = Self::require_string(config, key)?;
        Self::parse_size(&size_str)
    }

    /// 获取大小字段（可选）
    pub fn get_size_field(
        config: &HashMap<String, Value>,
        key: &str,
    ) -> EvifResult<Option<i64>> {
        match Self::get_string(config, key) {
            Some(s) => Ok(Some(Self::parse_size(&s)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_only_known_keys() {
        let mut config = HashMap::new();
        config.insert("host".to_string(), json!("localhost"));
        config.insert("port".to_string(), json!(8080));

        // 应该通过
        assert!(ConfigValidator::validate_only_known_keys(
            &config,
            &["host", "port"]
        ).is_ok());

        // 应该失败（未知键）
        assert!(ConfigValidator::validate_only_known_keys(
            &config,
            &["host"]
        ).is_err());
    }

    #[test]
    fn test_require_string() {
        let mut config = HashMap::new();
        config.insert("name".to_string(), json!("test"));

        assert_eq!(
            ConfigValidator::require_string(&config, "name").unwrap(),
            "test"
        );

        // 缺失字段
        assert!(ConfigValidator::require_string(&config, "missing").is_err());

        // 错误类型
        config.insert("wrong".to_string(), json!(123));
        assert!(ConfigValidator::require_string(&config, "wrong").is_err());
    }

    #[test]
    fn test_require_int() {
        let mut config = HashMap::new();
        config.insert("count".to_string(), json!(42));

        assert_eq!(
            ConfigValidator::require_int(&config, "count").unwrap(),
            42
        );

        // 缺失字段
        assert!(ConfigValidator::require_int(&config, "missing").is_err());
    }

    #[test]
    fn test_require_bool() {
        let mut config = HashMap::new();
        config.insert("enabled".to_string(), json!(true));

        assert_eq!(
            ConfigValidator::require_bool(&config, "enabled").unwrap(),
            true
        );
    }

    #[test]
    fn test_parse_size() {
        // 基本单位
        assert_eq!(ConfigValidator::parse_size("1024").unwrap(), 1024);
        assert_eq!(ConfigValidator::parse_size("1KB").unwrap(), 1024);
        assert_eq!(ConfigValidator::parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(ConfigValidator::parse_size("2GB").unwrap(), 2 * 1024 * 1024 * 1024);

        // 大小写不敏感
        assert_eq!(ConfigValidator::parse_size("1kb").unwrap(), 1024);
        assert_eq!(ConfigValidator::parse_size("1Mb").unwrap(), 1024 * 1024);

        // 错误格式
        assert!(ConfigValidator::parse_size("invalid").is_err());
        assert!(ConfigValidator::parse_size("1XB").is_err()); // 未知单位
    }

    #[test]
    fn test_get_optional_fields() {
        let mut config = HashMap::new();
        config.insert("name".to_string(), json!("test"));
        config.insert("count".to_string(), json!(42));
        config.insert("enabled".to_string(), json!(true));

        assert_eq!(
            ConfigValidator::get_string(&config, "name"),
            Some("test".to_string())
        );
        assert_eq!(ConfigValidator::get_int(&config, "count"), Some(42));
        assert_eq!(ConfigValidator::get_bool(&config, "enabled"), Some(true));

        // 不存在的字段
        assert_eq!(ConfigValidator::get_string(&config, "missing"), None);
        assert_eq!(ConfigValidator::get_int(&config, "missing"), None);
        assert_eq!(ConfigValidator::get_bool(&config, "missing"), None);
    }
}
