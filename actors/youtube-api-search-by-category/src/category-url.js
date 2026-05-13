const API_BASE_URL = 'https://ytapi.scrappa.co/search/category';
const MAX_LIMIT = 20;

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

    if (typeof value !== 'string') {
        return undefined;
    }

    const trimmedValue = value.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

function positiveInteger(value) {
    if (!Number.isInteger(value) || value <= 0) {
        return undefined;
    }

    return value;
}

function featureValues(value) {
    if (Array.isArray(value)) {
        return value
            .filter((item) => typeof item === 'string')
            .flatMap((item) => item.split(','))
            .map((item) => item.trim())
            .filter(Boolean);
    }

    if (typeof value !== 'string') {
        return [];
    }

    return value
        .split(',')
        .map((item) => item.trim())
        .filter(Boolean);
}

export function buildCategoryRequest(input = {}) {
    const category = stringValue(input?.category, 'category');
    if (!category) {
        throw new Error('Search category is required.');
    }

    const params = new URLSearchParams({ category });

    const stringParams = {
        sort: stringValue(input?.sort, 'sort') ?? 'relevance',
        duration: stringValue(input?.duration, 'duration'),
        upload_date: stringValue(input?.upload_date, 'upload_date'),
        continuation: stringValue(input?.continuation, 'continuation'),
        contentType: stringValue(input?.contentType, 'contentType'),
    };

    for (const [key, value] of Object.entries(stringParams)) {
        if (value !== undefined) {
            params.set(key, value);
        }
    }

    const features = featureValues(input?.features);
    if (features.length > 0) {
        params.set('features', features.join(','));
    }

    const limit = positiveInteger(input?.limit);
    if (limit) {
        if (limit > MAX_LIMIT) {
            throw new Error(`Limit must be less than or equal to ${MAX_LIMIT}.`);
        }

        params.set('limit', String(limit));
    }

    return {
        url: `${API_BASE_URL}?${params.toString()}`,
        category,
    };
}

export function categoryVideosToDatasetItems(data = {}) {
    const results = Array.isArray(data?.results) ? data.results : data?.videos;
    return Array.isArray(results) ? results : [];
}

export function continuationToken(data = {}) {
    return data?.pagination?.continuationToken ?? data?.continuation ?? undefined;
}
