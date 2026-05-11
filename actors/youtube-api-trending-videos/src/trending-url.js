const API_BASE_URL = 'https://ytapi.scrappa.co/trending';

function stringValue(value) {
    const rawValue = Array.isArray(value) ? value.find((item) => item !== undefined && item !== null) : value;

    if (typeof rawValue !== 'string') {
        return undefined;
    }

    const trimmedValue = rawValue.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

export function buildTrendingRequest(input = {}) {
    const category = stringValue(input?.category);
    const type = stringValue(input?.type);
    const params = new URLSearchParams();

    if (category) {
        params.set('category', category);
    }

    if (type) {
        params.set('type', type);
    }

    const queryString = params.toString();

    return {
        url: queryString ? `${API_BASE_URL}?${queryString}` : API_BASE_URL,
        category,
        type,
    };
}

export function trendingVideosToDatasetItems(data = {}) {
    const results = Array.isArray(data?.results) ? data.results : data?.videos;
    return Array.isArray(results) ? results : [];
}

export function continuationToken(data = {}) {
    return data?.continuation ?? data?.pagination?.continuationToken;
}
