use std::sync::OnceLock;

use regex::Regex;

static SENSITIVE_VALUE_PATTERN: OnceLock<Regex> = OnceLock::new();

pub fn error_summary(message: &str) -> String {
    let pattern = SENSITIVE_VALUE_PATTERN.get_or_init(|| {
        Regex::new(r#"(?i)(?:x-api-key|api[_ -]?key|authorization|access[_ -]?token|token|secret)\s*["']?\s*[:=]\s*["']?[^\s,"'}\]]+"#)
            .expect("sensitive value pattern is valid")
    });
    pattern
        .replace_all(message, "[redacted]")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::error_summary;

    #[test]
    fn redacts_secrets_and_bounds_error_messages() {
        let summary =
            error_summary(r#"Bad gateway: {"token":"secret-value","api_key":"another-secret"}"#);
        assert!(!summary.contains("secret-value"));
        assert!(!summary.contains("another-secret"));
        assert!(summary.contains("[redacted]"));
        assert_eq!(error_summary(&"x".repeat(600)).len(), 500);
    }
}
