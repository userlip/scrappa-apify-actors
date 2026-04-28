const API_BASE_URL = 'https://ytapi.scrappa.co/search';

function singleValue(value) {
    const rawValue = Array.isArray(value) ? value.find((item) => item !== undefined && item !== null) : value;

    if (typeof rawValue !== 'string') {
        return undefined;
    }

    const trimmedValue = rawValue.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

function positiveInteger(value) {
    if (!Number.isInteger(value) || value <= 0) {
        return undefined;
    }

    return value;
}

function featureValue(value) {
    if (Array.isArray(value)) {
        const features = value
            .map((item) => singleValue(item))
            .filter(Boolean);

        return features.length > 0 ? features.join(',') : undefined;
    }

    return singleValue(value);
}

export function buildSearchUrl(input = {}) {
    const actorInput = input && typeof input === 'object' ? input : {};
    const query = singleValue(actorInput.q);
    if (!query) {
        throw new Error('Search query "q" is required.');
    }

    const params = new URLSearchParams({ q: query });

    const stringParams = {
        sort: singleValue(actorInput.sort) || 'relevance',
        duration: singleValue(actorInput.duration),
        upload_date: singleValue(actorInput.upload_date),
        continuation: singleValue(actorInput.continuation),
        contentType: singleValue(actorInput.contentType),
        features: featureValue(actorInput.features),
        type: singleValue(actorInput.type),
    };

    for (const [key, value] of Object.entries(stringParams)) {
        if (value) {
            params.set(key, value);
        }
    }

    const limit = positiveInteger(actorInput.limit);
    if (limit) {
        params.set('limit', String(limit));
    }

    return `${API_BASE_URL}?${params.toString()}`;
}
