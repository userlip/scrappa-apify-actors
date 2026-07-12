export interface GoogleHotelsSuggestion {
    position?: number;
    value?: string;
    type?: string;
    highlighted_words?: string[];
    autocomplete_suggestion?: string;
    property_token?: string;
    scrappa_google_hotels_link?: string;
    thumbnail?: string;
    [key: string]: unknown;
}

export interface GoogleHotelsAutocompleteResponse {
    search_parameters?: Record<string, unknown>;
    suggestions?: unknown;
    response_time_ms?: number;
    [key: string]: unknown;
}

const SUGGESTION_STRING_FIELDS = ['type', 'value', 'autocomplete_suggestion', 'property_token'] as const;

function isGoogleHotelsSuggestion(value: unknown): value is GoogleHotelsSuggestion {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
        return false;
    }

    const suggestion = value as Record<string, unknown>;
    return SUGGESTION_STRING_FIELDS.every((field) => (
        suggestion[field] === undefined || typeof suggestion[field] === 'string'
    ));
}

function suggestionIdentity(suggestion: GoogleHotelsSuggestion): string {
    const type = suggestion.type?.trim().toLocaleLowerCase('en-US') ?? '';
    const value = (suggestion.property_token ?? suggestion.value ?? suggestion.autocomplete_suggestion ?? '')
        .trim()
        .toLocaleLowerCase('en-US');
    return `${type}\u0000${value}`;
}

export function buildSuggestionDatasetItems(
    response: GoogleHotelsAutocompleteResponse,
    sourceQuery: string,
    params: Record<string, string>,
): Record<string, unknown>[] {
    const suggestions = Array.isArray(response.suggestions) ? response.suggestions : [];
    const seen = new Set<string>();
    const items: Record<string, unknown>[] = [];

    for (const suggestion of suggestions) {
        if (!isGoogleHotelsSuggestion(suggestion)) {
            continue;
        }

        const identity = suggestionIdentity(suggestion);
        if (identity.endsWith('\u0000') || seen.has(identity)) {
            continue;
        }
        seen.add(identity);

        items.push({
            ...suggestion,
            position: suggestion.position ?? null,
            value: suggestion.value ?? suggestion.autocomplete_suggestion ?? null,
            type: suggestion.type ?? null,
            autocomplete_suggestion: suggestion.autocomplete_suggestion ?? null,
            highlighted_words: Array.isArray(suggestion.highlighted_words) ? suggestion.highlighted_words : [],
            property_token: suggestion.property_token ?? null,
            thumbnail: suggestion.thumbnail ?? null,
            scrappa_google_hotels_link: suggestion.scrappa_google_hotels_link ?? null,
            source_query: sourceQuery,
            request_gl: params.gl ?? null,
            request_hl: params.hl ?? null,
            request_currency: params.currency ?? null,
            request_type: params.type ?? null,
            response_time_ms: response.response_time_ms ?? null,
        });
    }

    return items;
}
