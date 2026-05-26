export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const API_BASE_URL = 'https://scrappa.co/api/youtube/search';

const SORT_PARAM_VALUES = {
    upload_date: 'date',
    view_count: 'viewCount',
};

export function getScrappaApiKey(env = process.env) {
    const apiKey = env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    return apiKey;
}

export function buildHashtagSearchUrl({
    hashtag,
    sort = 'relevance',
    limit,
    duration,
    upload_date,
    continuation = '',
    contentType,
    features,
} = {}) {
    if (!hashtag) {
        throw new Error('Search query "hashtag" not provided. Please provide a value for "searchHashtag" in the input.');
    }

    const normalizedHashtag = hashtag.startsWith('#') ? hashtag : `#${hashtag}`;
    const params = new URLSearchParams({ query: normalizedHashtag });

    if (sort && typeof sort === 'string' && sort.trim() !== '') {
        params.set('order', SORT_PARAM_VALUES[sort] ?? sort);
    }

    if (Number.isInteger(limit) && limit > 0) {
        params.set('limit', String(Math.min(limit, 20)));
    }

    if (duration && typeof duration === 'string' && duration.trim() !== '') {
        params.set('videoDuration', duration);
    }

    if (upload_date && typeof upload_date === 'string' && upload_date.trim() !== '') {
        params.set('upload_date', upload_date);
    }

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        params.set('continuation', continuation);
    }

    if (contentType && typeof contentType === 'string' && contentType.trim() !== '') {
        params.set('contentType', contentType);
    }

    if (features && typeof features === 'string' && features.trim() !== '') {
        params.set('features', features);
    }

    return `${API_BASE_URL}?${params.toString()}`;
}

export function buildScrappaRequest(apiUrl, apiKey) {
    return {
        apiUrl,
        requestOptions: {
            timeout: SCRAPPA_REQUEST_TIMEOUT_MS,
            headers: {
                'X-API-Key': apiKey,
                'Accept': 'application/json',
            },
        },
    };
}
