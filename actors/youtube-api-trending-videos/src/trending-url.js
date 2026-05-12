const API_BASE_URL = 'https://ytapi.scrappa.co/trending';

function stringValue(value, fieldName) {
    if (Array.isArray(value)) {
        const values = value
            .filter((item) => typeof item === 'string')
            .map((item) => item.trim())
            .filter(Boolean);

        if (values.length > 1) {
            throw new Error(`${fieldName} accepts only one selected value.`);
        }

        return values[0];
    }

    const rawValue = value;

    if (typeof rawValue !== 'string') {
        return undefined;
    }

    const trimmedValue = rawValue.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

export function buildTrendingRequest(input = {}) {
    const category = stringValue(input?.category, 'category');
    const type = stringValue(input?.type, 'type');
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
    return data?.pagination?.continuationToken ?? data?.continuation ?? undefined;
}
