export interface StartpageOrganicResult {
    title?: string;
    description?: string;
    url?: string;
    domain?: string;
    position?: number;
    source?: string;
    [key: string]: unknown;
}

export interface StartpageSearchResponse {
    data?: StartpageOrganicResult[];
    organic_results?: StartpageOrganicResult[];
    results?: StartpageOrganicResult[];
    total_results?: number;
    source?: string;
    pagination?: unknown;
    scrappa_pagination?: unknown;
    [key: string]: unknown;
}

export function extractStartpageOrganicResults(response: unknown): StartpageOrganicResult[] {
    if (Array.isArray(response)) {
        return response;
    }

    if (response && typeof response === 'object') {
        const payload = response as StartpageSearchResponse;
        if (Array.isArray(payload.data)) {
            return payload.data;
        }
        if (Array.isArray(payload.organic_results)) {
            return payload.organic_results;
        }
        if (Array.isArray(payload.results)) {
            return payload.results;
        }
    }

    console.warn('Scrappa Startpage response did not include an organic result array');
    return [];
}

function cleanSource(value: unknown): string | undefined {
    if (typeof value !== 'string') {
        return undefined;
    }

    const source = value.trim();
    return source === '' ? undefined : source;
}

export function buildStartpageDatasetItem(
    result: StartpageOrganicResult,
    params: Record<string, unknown>,
    response: StartpageSearchResponse,
): Record<string, unknown> {
    return {
        ...result,
        query: params.query ?? null,
        position: result.position ?? null,
        title: result.title ?? null,
        description: result.description ?? null,
        url: result.url ?? null,
        domain: result.domain ?? null,
        source: cleanSource(result.source) ?? cleanSource(response.source) ?? 'startpage',
        request_query: params.query ?? null,
        request_language: params.language ?? null,
        request_page: params.page ?? null,
        request_safe_search: params.safe_search ?? null,
        total_results: response.total_results ?? null,
        pagination: response.pagination ?? null,
        scrappa_pagination: response.scrappa_pagination ?? null,
    };
}
