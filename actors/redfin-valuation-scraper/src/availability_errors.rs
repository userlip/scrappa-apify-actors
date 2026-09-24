use crate::scrappa::ScrappaError;

pub fn is_transient_redfin_valuation_error(error: &ScrappaError) -> bool {
    match error {
        ScrappaError::Timeout { .. } => true,
        ScrappaError::Http { status, .. } => (500..=599).contains(status),
        ScrappaError::Network | ScrappaError::InvalidResponse(_) | ScrappaError::Request(_) => {
            false
        }
    }
}

pub fn get_transient_redfin_valuation_status(error: &ScrappaError) -> String {
    let reason = match error {
        ScrappaError::Http { status, .. } => {
            format!("Scrappa upstream returned {status} after retries")
        }
        ScrappaError::Timeout { timeout_ms } => {
            format!("Scrappa API request timed out after {timeout_ms}ms")
        }
        other => other.to_string(),
    };
    format!("{reason}; no Redfin valuation result was written or charged. Try the run again later.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treats_http_server_errors_and_timeouts_as_transient() {
        assert!(is_transient_redfin_valuation_error(&ScrappaError::Http {
            status: 500,
            details: "failed".to_owned()
        }));
        assert!(is_transient_redfin_valuation_error(&ScrappaError::Http {
            status: 503,
            details: "unavailable".to_owned()
        }));
        assert!(is_transient_redfin_valuation_error(
            &ScrappaError::Timeout { timeout_ms: 60_000 }
        ));
    }

    #[test]
    fn keeps_client_validation_and_network_errors_fatal_after_retries() {
        assert!(!is_transient_redfin_valuation_error(&ScrappaError::Http {
            status: 404,
            details: "not found".to_owned()
        }));
        assert!(!is_transient_redfin_valuation_error(&ScrappaError::Http {
            status: 422,
            details: "bad input".to_owned()
        }));
        assert!(!is_transient_redfin_valuation_error(&ScrappaError::Network));
    }

    #[test]
    fn describes_transient_failures_as_unwritten_and_uncharged() {
        assert_eq!(
            get_transient_redfin_valuation_status(&ScrappaError::Http { status: 500, details: "failed".to_owned() }),
            "Scrappa upstream returned 500 after retries; no Redfin valuation result was written or charged. Try the run again later.",
        );
        assert_eq!(
            get_transient_redfin_valuation_status(&ScrappaError::Timeout { timeout_ms: 60_000 }),
            "Scrappa API request timed out after 60000ms; no Redfin valuation result was written or charged. Try the run again later.",
        );
    }
}
