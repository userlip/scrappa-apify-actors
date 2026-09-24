pub(crate) fn build_transient_failure_status_message(
    failure_message: &str,
    total_results_written: usize,
    total_queries: usize,
) -> String {
    if total_results_written > 0 {
        format!(
            "{failure_message}; {total_results_written} Google Finance search results were already written and may have been charged. Remaining queries were not completed. Try the run again later for the unfinished queries."
        )
    } else if total_queries > 1 {
        format!(
            "{failure_message}; no Google Finance search results were written or charged. Remaining batch queries were not completed. Try the run again later."
        )
    } else {
        format!(
            "{failure_message}; no Google Finance search results were written or charged. Try the run again later."
        )
    }
}
