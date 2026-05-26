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

export function buildPlaylistSearchUrl({ q, sort = 'relevance', limit = 20, continuation = '' } = {}) {
    if (!q) {
        throw new Error('Search query "q" not provided. Please provide a value for "searchPlaylistQuery" in the input.');
    }

    const params = new URLSearchParams({
        query: q,
        type: 'playlist',
    });

    if (sort && typeof sort === 'string' && sort.trim() !== '') {
        params.set('order', SORT_PARAM_VALUES[sort] ?? sort);
    }

    if (Number.isInteger(limit) && limit > 0) {
        params.set('limit', String(Math.min(limit, 20)));
    }

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        params.set('continuation', continuation);
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
