export interface GoogleTrendsAutocompleteSuggestion {
    suggestion?: unknown;
    query?: unknown;
    keyword?: unknown;
    title?: unknown;
    value?: unknown;
    name?: unknown;
    type?: unknown;
    [key: string]: unknown;
}

export interface GoogleTrendsAutocompleteResponse {
    search_parameters?: Record<string, unknown>;
    suggestions?: unknown;
    autocomplete?: unknown;
    results?: unknown;
    data?: unknown;
    response_time_ms?: number;
    [key: string]: unknown;
}

function asSuggestionEntries(value: unknown): GoogleTrendsAutocompleteSuggestion[] {
    if (Array.isArray(value)) {
        return value
            .map((item): GoogleTrendsAutocompleteSuggestion | null => {
                if (typeof item === 'string') {
                    return { suggestion: item };
                }

                if (item !== null && typeof item === 'object' && !Array.isArray(item)) {
                    return item as GoogleTrendsAutocompleteSuggestion;
                }

                return null;
            })
            .filter((item): item is GoogleTrendsAutocompleteSuggestion => item !== null);
    }

    if (value !== null && typeof value === 'object') {
        const record = value as Record<string, unknown>;
        return asSuggestionEntries(record.suggestions ?? record.results ?? record.data);
    }

    return [];
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return null;
}

function getSuggestionEntries(response: GoogleTrendsAutocompleteResponse): GoogleTrendsAutocompleteSuggestion[] {
    return asSuggestionEntries(
        response.suggestions
        ?? response.autocomplete
        ?? response.results
        ?? response.data,
    );
}

function buildDatasetItem(
    entry: GoogleTrendsAutocompleteSuggestion,
    params: Record<string, unknown>,
    response: GoogleTrendsAutocompleteResponse,
    position: number,
): Record<string, unknown> {
    const suggestion = firstString(entry.suggestion, entry.query, entry.keyword, entry.title, entry.value, entry.name);

    return {
        ...entry,
        position,
        suggestion,
        type: firstString(entry.type),
        source_keyword: params.q ?? null,
        request_geo: params.geo ?? null,
        request_hl: params.hl ?? null,
        response_time_ms: response.response_time_ms ?? null,
        search_parameters: response.search_parameters ?? null,
    };
}

export function buildAutocompleteDatasetItems(
    response: GoogleTrendsAutocompleteResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    return getSuggestionEntries(response)
        .map((entry, index) => buildDatasetItem(entry, params, response, index + 1));
}
