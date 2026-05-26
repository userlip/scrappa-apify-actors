export interface GooglePatentsDetailsData {
    patent_id?: string | null;
    publication_number?: string | null;
    title?: string | null;
    abstract?: string | null;
    inventors?: string[];
    assignees?: Record<string, string[]>;
    dates?: Record<string, string | null>;
    citations?: Record<string, unknown[]>;
    cached?: boolean;
    response_time_ms?: number;
    [key: string]: unknown;
}

export interface GooglePatentsDetailsResponse {
    success: boolean;
    data?: GooglePatentsDetailsData;
    error?: string;
    message?: string;
    [key: string]: unknown;
}

export interface GooglePatentsDetailsDatasetContext {
    inputPatentId: string;
    normalizedPatentId: string;
}

export function patentPageUrl(patentId: string): string {
    const match = /^patent\/([^/]+)\/[a-z]{2}$/i.exec(patentId);
    const publicationNumber = match?.[1] ?? patentId;

    return `https://patents.google.com/patent/${publicationNumber}`;
}

export function buildSuccessDatasetItem(
    response: GooglePatentsDetailsResponse,
    context: GooglePatentsDetailsDatasetContext,
): Record<string, unknown> {
    const data = response.data ?? {};
    const patentId = data.patent_id ?? context.normalizedPatentId;

    return {
        ...data,
        input_patent_id: context.inputPatentId,
        normalized_patent_id: context.normalizedPatentId,
        success: true,
        patent_id: patentId,
        publication_number: data.publication_number ?? null,
        patent_page: patentPageUrl(String(patentId)),
        title: data.title ?? null,
        abstract: data.abstract ?? null,
        inventor_count: Array.isArray(data.inventors) ? data.inventors.length : 0,
        assignee_count: data.assignees && typeof data.assignees === 'object' ? Object.keys(data.assignees).length : 0,
        citation_count: countCitations(data.citations),
        cached: data.cached ?? false,
        response_time_ms: data.response_time_ms ?? null,
    };
}

export function buildErrorDatasetItem(
    error: unknown,
    context: GooglePatentsDetailsDatasetContext,
): Record<string, unknown> {
    const message = error instanceof Error ? error.message : String(error);
    const statusCode = extractScrappaStatusCode(message);

    return {
        input_patent_id: context.inputPatentId,
        normalized_patent_id: context.normalizedPatentId,
        success: false,
        error: message,
        status_code: statusCode,
    };
}

export function extractScrappaStatusCode(message: string): number | null {
    const match = /Scrappa API error \((\d{3})\)/.exec(message);
    return match ? Number(match[1]) : null;
}

function countCitations(citations: GooglePatentsDetailsData['citations']): number {
    if (!citations || typeof citations !== 'object') {
        return 0;
    }

    return Object.values(citations).reduce((total, value) => {
        return total + (Array.isArray(value) ? value.length : 0);
    }, 0);
}
