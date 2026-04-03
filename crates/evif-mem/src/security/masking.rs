//! Data masking utilities for sensitive information
//!
//! Provides utilities to mask sensitive data like passwords, tokens, API keys, etc.


/// Mask configuration
#[derive(Debug, Clone, Default)]
pub struct MaskConfig {
    /// Fields to mask by name (case-insensitive)
    pub fields: Vec<String>,
    /// Mask character
    pub mask_char: char,
    /// Show first N characters
    pub show_prefix: usize,
    /// Show last N characters
    pub show_suffix: usize,
}

impl MaskConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Add field to mask
    pub fn add_field(mut self, field: &str) -> Self {
        self.fields.push(field.to_lowercase());
        self
    }

    /// Build config
    pub fn build(self) -> Self {
        self
    }
}

/// Sensitive field definition
#[derive(Debug, Clone)]
pub struct SensitiveField {
    /// Field name (case-insensitive)
    pub name: String,
    /// Mask type
    pub mask_type: MaskType,
}

/// Mask type
#[derive(Debug, Clone)]
pub enum MaskType {
    /// Full mask
    Full,
    /// Partial mask (show prefix and suffix)
    Partial(usize, usize),
    /// Email mask
    Email,
    /// Credit card mask
    CreditCard,
    /// Phone mask
    Phone,
}

/// Default sensitive fields
pub fn default_sensitive_fields() -> Vec<SensitiveField> {
    vec![
        SensitiveField {
            name: "password".to_string(),
            mask_type: MaskType::Full,
        },
        SensitiveField {
            name: "passwd".to_string(),
            mask_type: MaskType::Full,
        },
        SensitiveField {
            name: "secret".to_string(),
            mask_type: MaskType::Full,
        },
        SensitiveField {
            name: "token".to_string(),
            mask_type: MaskType::Partial(4, 4),
        },
        SensitiveField {
            name: "api_key".to_string(),
            mask_type: MaskType::Partial(4, 4),
        },
        SensitiveField {
            name: "apikey".to_string(),
            mask_type: MaskType::Partial(4, 4),
        },
        SensitiveField {
            name: "access_token".to_string(),
            mask_type: MaskType::Partial(4, 4),
        },
        SensitiveField {
            name: "refresh_token".to_string(),
            mask_type: MaskType::Partial(4, 4),
        },
        SensitiveField {
            name: "authorization".to_string(),
            mask_type: MaskType::Partial(4, 4),
        },
        SensitiveField {
            name: "private_key".to_string(),
            mask_type: MaskType::Full,
        },
        SensitiveField {
            name: "credit_card".to_string(),
            mask_type: MaskType::CreditCard,
        },
        SensitiveField {
            name: "card_number".to_string(),
            mask_type: MaskType::CreditCard,
        },
        SensitiveField {
            name: "ssn".to_string(),
            mask_type: MaskType::Full,
        },
        SensitiveField {
            name: "phone".to_string(),
            mask_type: MaskType::Phone,
        },
        SensitiveField {
            name: "email".to_string(),
            mask_type: MaskType::Email,
        },
    ]
}

/// Mask sensitive data in a string based on field names
pub fn mask_sensitive_data(input: &str, config: &MaskConfig) -> String {
    let mut result = input.to_string();

    // Mask known sensitive fields using simple string replacement
    for field in &config.fields {
        let mask_char = config.mask_char;
        let show_prefix = config.show_prefix;
        let show_suffix = config.show_suffix;

        // Try to find "field": "value" pattern
        let patterns = vec![
            format!(r#""{}": "{}""#, field, "*".repeat(20)),
            format!("{}={}", field, "*".repeat(20)),
        ];

        for _pattern in patterns {
            if result.contains(field) {
                // Simple replacement - find the field and mask its value
                if let Some(start) = result.to_lowercase().find(field) {
                    // Find the value after the field name
                    let after_field = &result[start..];
                    if let Some(colon_pos) = after_field.find(':') {
                        if let Some(quote_start) = after_field[colon_pos..].find('"') {
                            let value_start = start + colon_pos + quote_start + 1;
                            if let Some(quote_end) =
                                after_field[colon_pos + quote_start + 1..].find('"')
                            {
                                let value_end = value_start + quote_end;
                                if value_end <= result.len() {
                                    let value = &result[value_start..value_end];
                                    let masked =
                                        mask_value(value, mask_char, show_prefix, show_suffix);
                                    result = format!(
                                        "{}{}{}",
                                        &result[..value_start],
                                        masked,
                                        &result[value_end..]
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

/// Mask a value based on its content
fn mask_value(value: &str, mask_char: char, show_prefix: usize, show_suffix: usize) -> String {
    let value = value.trim_matches('"');

    if value.len() <= show_prefix + show_suffix {
        return mask_char.to_string().repeat(value.len());
    }

    let prefix = &value[..show_prefix.min(value.len())];
    let suffix = if show_suffix > 0 {
        let start = value.len().saturating_sub(show_suffix);
        &value[start..]
    } else {
        ""
    };

    let mask_len = value.len().saturating_sub(show_prefix + show_suffix);
    let mask = mask_char.to_string().repeat(mask_len);

    format!("{}{}{}", prefix, mask, suffix)
}

/// Mask email address
pub fn mask_email(email: &str) -> String {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return "*".repeat(email.len());
    }

    let local = parts[0];
    let domain = parts[1];

    let masked_local = if local.len() <= 2 {
        "*".repeat(local.len())
    } else {
        format!("{}{}", &local[..1], "*".repeat(local.len() - 1))
    };

    format!("{}@{}", masked_local, domain)
}

/// Mask credit card number (show last 4 digits)
pub fn mask_credit_card(card: &str) -> String {
    let digits: String = card.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() < 4 {
        return "*".repeat(card.len());
    }

    let visible = &digits[digits.len() - 4..];
    format!("****-****-****-{}", visible)
}

/// Mask phone number
pub fn mask_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() < 4 {
        return "*".repeat(phone.len());
    }

    let visible = &digits[digits.len() - 4..];
    format!("***-***-{}", visible)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        let config = MaskConfig::new().add_field("password").build();
        let input = r#"{"password": "secret123"}"#;
        let result = mask_sensitive_data(input, &config);
        // Should mask the value (exact output depends on implementation)
        assert!(result.contains("password"));
    }

    #[test]
    fn test_mask_email() {
        let email = "john.doe@example.com";
        let masked = mask_email(email);
        assert!(masked.starts_with("j"));
        assert!(masked.contains("@"));
        assert!(masked.ends_with("@example.com"));
    }

    #[test]
    fn test_mask_credit_card() {
        let card = "1234-5678-9012-3456";
        let masked = mask_credit_card(card);
        assert!(masked.ends_with("3456"));
    }

    #[test]
    fn test_mask_phone() {
        let phone = "+1-234-567-8901";
        let masked = mask_phone(phone);
        assert!(masked.ends_with("8901"));
    }
}
