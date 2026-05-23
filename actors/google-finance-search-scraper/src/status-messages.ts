export function buildTransientFailureStatusMessage(
    failureMessage: string,
    totalResultsWritten: number,
): string {
    if (totalResultsWritten > 0) {
        return `${failureMessage}; ${totalResultsWritten} Google Finance search results were already written and may have been charged. Remaining queries were not completed. Try the run again later for the unfinished queries.`;
    }

    return `${failureMessage}; no Google Finance search results were written or charged. Try the run again later.`;
}
