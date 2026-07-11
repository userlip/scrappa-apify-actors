export interface RunSummaryItem {
    status: string;
    videos_saved: number;
}

export function isTotalFailure(summaries: RunSummaryItem[]): boolean {
    return summaries.length > 0
        && summaries.every((summary) => summary.status === 'failed' && summary.videos_saved === 0);
}
