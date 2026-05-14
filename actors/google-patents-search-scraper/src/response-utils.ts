export interface PatentDates {
    priority?: string | null;
    filing?: string | null;
    grant?: string | null;
    publication?: string | null;
    [key: string]: unknown;
}

export interface PatentFamilyStatus {
    country?: string | null;
    status?: string | null;
    [key: string]: unknown;
}

export interface GooglePatentResult {
    patent_id?: string | null;
    rank?: number;
    title?: string | null;
    snippet?: string | null;
    publication_number?: string | null;
    language?: string | null;
    dates?: PatentDates;
    inventor?: string | null;
    assignee?: string | null;
    thumbnail?: string | null;
    pdf?: string | null;
    family_status?: PatentFamilyStatus[];
    [key: string]: unknown;
}

export interface GooglePatentsSearchData {
    total_results?: number;
    total_pages?: number;
    current_page?: number;
    many_results?: boolean;
    cached?: boolean;
    stale?: boolean;
    response_time_ms?: number;
    patents?: GooglePatentResult[];
    [key: string]: unknown;
}

export type GooglePatentsSearchResponse =
    | { success?: boolean; data?: GooglePatentsSearchData; [key: string]: unknown }
    | GooglePatentsSearchData;

export function extractPatentSearchData(response: GooglePatentsSearchResponse): GooglePatentsSearchData {
    if (response && typeof response === 'object' && 'data' in response && response.data && typeof response.data === 'object') {
        return response.data as GooglePatentsSearchData;
    }

    return response as GooglePatentsSearchData;
}

export function extractPatentResults(response: GooglePatentsSearchResponse): GooglePatentResult[] {
    const data = extractPatentSearchData(response);
    if (Array.isArray(data.patents)) {
        return data.patents;
    }

    console.warn('Scrappa Google Patents response did not include a patents result array');
    return [];
}

export function patentPageUrl(result: GooglePatentResult): string | null {
    const publicationNumber = result.publication_number
        ?? (typeof result.patent_id === 'string' ? result.patent_id.split('/')[1] : undefined);

    return publicationNumber ? `https://patents.google.com/patent/${publicationNumber}` : null;
}

export function enrichResult(result: GooglePatentResult, params: Record<string, unknown>): Record<string, unknown> {
    const dates = result.dates ?? {};
    const familyStatus = result.family_status ?? [];

    return {
        ...result,
        patent_id: result.patent_id ?? null,
        patent_page: patentPageUrl(result),
        rank: result.rank ?? null,
        title: result.title ?? null,
        snippet: result.snippet ?? null,
        publication_number: result.publication_number ?? null,
        language: result.language ?? null,
        priority_date: dates.priority ?? null,
        filing_date: dates.filing ?? null,
        grant_date: dates.grant ?? null,
        publication_date: dates.publication ?? null,
        inventor: result.inventor ?? null,
        assignee: result.assignee ?? null,
        thumbnail: result.thumbnail ?? null,
        pdf: result.pdf ?? null,
        family_status_count: familyStatus.length,
        family_countries: familyStatus
            .map((status) => status.country)
            .filter((country): country is string => typeof country === 'string' && country !== '')
            .join(',') || null,
        request_q: params.q ?? null,
        request_page: params.page ?? null,
        request_num: params.num ?? null,
        request_sort: params.sort ?? null,
        request_before: params.before ?? null,
        request_after: params.after ?? null,
        request_country: params.country ?? null,
        request_language: params.language ?? null,
        request_status: params.status ?? null,
        request_type: params.type ?? null,
        request_inventor: params.inventor ?? null,
        request_assignee: params.assignee ?? null,
    };
}
