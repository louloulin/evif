//! Token counting and truncation module
//!
//! Provides token counting for memory buffer management.
//! Uses character-based estimation as the primary method (network-free).
//! When tiktoken is available, it can provide accurate BPE-based counting.
//!
//! # Usage
//! ```ignore
//! use evif_mem::token::{TokenBudget, estimation};
//!
//! // Character-based estimation (works offline)
//! let budget = TokenBudget::new(1000, 100).unwrap();
//! let count = budget.count("Hello, world!");
//!
//! // Truncate to fit
//! let truncated = budget.truncate("Very long text...");
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Token counting errors
#[derive(Error, Debug)]
pub enum TokenError {
    #[error("Token encoding failed: {0}")]
    EncodingError(String),
    #[error("Token decoding failed: {0}")]
    DecodingError(String),
}

/// Token budget for managing token limits
/// Uses character-based estimation (4 chars/token for English, 2 for CJK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Maximum tokens allowed
    max_tokens: usize,
    /// Reserved tokens for overhead (system prompt, etc.)
    reserved_tokens: usize,
    /// Average characters per token (for estimation)
    chars_per_token: f64,
}

impl TokenBudget {
    /// Create a new TokenBudget with default settings
    /// - 4 chars per token for English
    /// - 2 chars per token for CJK (Chinese/Japanese/Korean)
    pub fn new(max_tokens: usize, reserved_tokens: usize) -> Result<Self, TokenError> {
        Ok(Self {
            max_tokens,
            reserved_tokens,
            chars_per_token: 4.0, // Default for English
        })
    }

    /// Count tokens in text using character-based estimation
    pub fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let has_cjk = estimation::has_cjk(text);
        let chars_per_token = if has_cjk { 2.0 } else { 4.0 };

        (text.chars().count() as f64 / chars_per_token).ceil() as usize
    }

    /// Count tokens in multiple texts
    pub fn count_batch(&self, texts: &[&str]) -> usize {
        texts.iter().map(|t| self.count(t)).sum()
    }

    /// Get available tokens (max - reserved)
    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserved_tokens)
    }

    /// Check if text fits within budget
    pub fn fits(&self, text: &str) -> bool {
        self.count(text) <= self.available_tokens()
    }

    /// Truncate text to fit within available tokens
    /// Truncates by characters, which roughly corresponds to token limits
    pub fn truncate(&self, text: &str) -> String {
        let has_cjk = estimation::has_cjk(text);
        let chars_per_token = if has_cjk { 2.0 } else { 4.0 };
        let max_chars = (self.available_tokens() as f64 * chars_per_token) as usize;

        if text.chars().count() <= max_chars {
            return text.to_string();
        }

        // Truncate by character count
        text.chars().take(max_chars).collect()
    }

    /// Truncate multiple texts to fit within budget
    pub fn truncate_batch(&self, texts: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(texts.len());
        let mut used_tokens = 0;

        for text in texts {
            let tokens = self.count(text);

            if used_tokens + tokens > self.max_tokens {
                let remaining = self.max_tokens.saturating_sub(used_tokens);
                if remaining > 1 {
                    let truncated = self.truncate_to_tokens(text, remaining);
                    result.push(truncated);
                }
                break;
            } else {
                result.push(text.clone());
                used_tokens += tokens;
            }
        }

        result
    }

    /// Truncate text to specific number of tokens
    pub fn truncate_to_tokens(&self, text: &str, max_tokens: usize) -> String {
        if max_tokens == 0 {
            return String::new();
        }

        let has_cjk = estimation::has_cjk(text);
        let chars_per_token = if has_cjk { 2.0 } else { 4.0 };
        let max_chars = (max_tokens as f64 * chars_per_token) as usize;

        if text.chars().count() <= max_chars {
            return text.to_string();
        }

        text.chars().take(max_chars).collect()
    }

    /// Get max tokens
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Get reserved tokens
    pub fn reserved_tokens(&self) -> usize {
        self.reserved_tokens
    }
}

/// Token estimation utilities (network-free, character-based)
pub mod estimation {
    /// Estimate tokens using character count
    /// - English: ~4 characters per token
    /// - Chinese/Japanese/Korean: ~2 characters per token
    pub fn estimate_chars_to_tokens(chars: usize, is_cjk: bool) -> usize {
        if is_cjk {
            (chars as f64 / 2.0).ceil() as usize
        } else {
            (chars as f64 / 4.0).ceil() as usize
        }
    }

    /// Estimate tokens from word count
    /// Average ~1.3 tokens per word for English
    pub fn estimate_words_to_tokens(words: usize) -> usize {
        (words as f64 * 1.3).ceil() as usize
    }

    /// Check if text contains CJK characters
    pub fn has_cjk(text: &str) -> bool {
        text.chars().any(|c| {
            let code = c as u32;
            // CJK Unified Ideographs (Chinese, Japanese, Korean)
            (0x4E00..=0x9FFF).contains(&code)
                || (0x3400..=0x4DBF).contains(&code) // CJK Extension A
                || (0x3000..=0x303F).contains(&code) // CJK Symbols
                || (0xFF00..=0xFFEF).contains(&code) // Halfwidth CJK
                || (0xAC00..=0xD7AF).contains(&code) // Korean Hangul
        })
    }

    /// Get the ratio of CJK characters to total
    pub fn cjk_ratio(text: &str) -> f64 {
        let total = text.chars().count();
        if total == 0 {
            return 0.0;
        }

        let cjk_count = text.chars().filter(|c| is_cjk_char(*c)).count();
        cjk_count as f64 / total as f64
    }

    fn is_cjk_char(c: char) -> bool {
        let code = c as u32;
        (0x4E00..=0x9FFF).contains(&code)
            || (0x3400..=0x4DBF).contains(&code)
            || (0x3000..=0x303F).contains(&code)
            || (0xFF00..=0xFFEF).contains(&code)
            || (0xAC00..=0xD7AF).contains(&code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_budget_count() {
        let budget = TokenBudget::new(1000, 0).unwrap();

        // Count tokens in simple text (~4 chars per token)
        let text = "Hello, world!";
        let count = budget.count(text);
        assert!(count >= 2 && count <= 4);
    }

    #[test]
    fn test_token_budget_truncate() {
        let budget = TokenBudget::new(10, 0).unwrap();

        let text = "This is a very long text that should be truncated to fit within the token limit";
        let truncated = budget.truncate(&text);

        // Should be roughly 40 chars (10 tokens * 4 chars)
        assert!(truncated.chars().count() <= 45);
    }

    #[test]
    fn test_token_budget_available() {
        let budget = TokenBudget::new(1000, 100).unwrap();

        assert_eq!(budget.available_tokens(), 900);
    }

    #[test]
    fn test_token_budget_fits() {
        let budget = TokenBudget::new(10, 0).unwrap();

        assert!(budget.fits("Short"));
        assert!(!budget.fits(&"x".repeat(100)));
    }

    #[test]
    fn test_cjk_estimation() {
        let text = "你好世界";
        assert!(estimation::has_cjk(text));

        // 4 CJK characters (use char count, not byte count)
        let char_count = text.chars().count();
        let tokens = estimation::estimate_chars_to_tokens(char_count, true);
        assert_eq!(tokens, 2); // 4 chars / 2 = 2 tokens
    }

    #[test]
    fn test_budget_cjk_count() {
        let budget = TokenBudget::new(100, 0).unwrap();

        // CJK text should be counted at 2 chars per token
        let text = "你好世界"; // 4 characters
        let count = budget.count(text);
        assert_eq!(count, 2); // 4 chars / 2 = 2 tokens
    }

    #[test]
    fn test_english_estimation() {
        let text = "Hello world";
        assert!(!estimation::has_cjk(text));

        let tokens = estimation::estimate_chars_to_tokens(text.len(), false);
        assert_eq!(tokens, 3); // 11 chars / 4 = 2.75 -> 3
    }

    #[test]
    fn test_truncate_batch() {
        let budget = TokenBudget::new(20, 0).unwrap();

        let texts = vec![
            "First short text".to_string(),
            "Second medium length text".to_string(),
            "Third very long text that will likely be truncated or skipped entirely".to_string(),
        ];

        let result = budget.truncate_batch(&texts);
        assert!(!result.is_empty());

        // Total tokens should not exceed max
        let total: usize = result.iter().map(|t| budget.count(t)).sum();
        assert!(total <= 20);
    }

    #[test]
    fn test_truncate_to_tokens() {
        let budget = TokenBudget::new(1000, 0).unwrap();

        let text = "This is a long piece of text";
        let truncated = budget.truncate_to_tokens(text, 5);

        // 5 tokens * 4 chars = 20 chars
        assert!(truncated.chars().count() <= 25);
    }

    #[test]
    fn test_mixed_cjk_english() {
        let budget = TokenBudget::new(100, 0).unwrap();

        let text = "Hello 你好 World 世界";
        let count = budget.count(text);

        // 5 English chars + 4 CJK chars + 5 English chars + 3 CJK chars = 17 total
        // But CJK is counted differently
        // 10 English / 4 = 2.5 -> 3 tokens
        // 7 CJK / 2 = 3.5 -> 4 tokens
        // Total: ~7 tokens
        assert!(count >= 6 && count <= 10);
    }
}
