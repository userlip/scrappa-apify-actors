const API_BASE_URL = 'https://ytapi.scrappa.co/search/suggestions';

function stringValue(value) {
    if (typeof value !== 'string') {
        return undefined;
    }

    const trimmedValue = value.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

function normalizedCountry(value) {
    const country = stringValue(value);
    return country ? country.toUpperCase() : undefined;
}

export function buildSuggestionsRequest(input = {}) {
    const query = stringValue(input?.q);
    if (!query) {
        throw new Error('Search query "q" is required.');
    }

    const params = new URLSearchParams({ q: query });
    const hl = stringValue(input?.hl);
    const gl = normalizedCountry(input?.gl);

    if (hl) {
        params.set('hl', hl);
    }

    if (gl) {
        params.set('gl', gl);
    }

    return {
        url: `${API_BASE_URL}?${params.toString()}`,
        query,
        hl,
        gl,
    };
}

export function suggestionsToDatasetItems(data = {}, fallback = {}) {
    const suggestions = Array.isArray(data?.suggestions) ? data.suggestions : [];
    const query = stringValue(data?.query) ?? fallback.query;
    const hl = stringValue(data?.locale?.hl) ?? fallback.hl;
    const gl = normalizedCountry(data?.locale?.gl) ?? fallback.gl;

    return suggestions
        .filter((suggestion) => typeof suggestion === 'string' && suggestion.trim() !== '')
        .map((suggestion, index) => ({
            query,
            suggestion,
            position: index + 1,
            hl,
            gl,
        }));
}
