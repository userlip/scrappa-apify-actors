export function buildTransientFailureStatusMessage(
    failureMessage: string,
    totalResultsWritten: number,
    totalQueries: number,
): string {
    if (totalResultsWritten > 0) {
        return `${failureMessage}; ${totalResultsWritten} Google Finance search results were already written and may have been charged. Remaining queries were not completed. Try the run again later for the unfinished queries.`;
    }

    if (totalQueries > 1) {
        return `${failureMessage}; no Google Finance search results were written or charged. Remaining batch queries were not completed. Try the run again later.`;
    }

    return `${failureMessage}; no Google Finance search results were written or charged. Try the run again later.`;
}
