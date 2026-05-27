export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const API_BASE_URL = 'https://scrappa.co/api/youtube/channel-playlists';

export function getScrappaApiKey(env = process.env) {
    const apiKey = env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    return apiKey;
}

export function parseIds(value) {
    if (Array.isArray(value)) {
        return value.flatMap((item) => parseIds(item));
    }

    if (typeof value !== 'string') {
        return [];
    }

    return value.split(',').map((id) => id.trim()).filter(Boolean);
}

export function getChannelIds(input = {}) {
    return [...new Set([...parseIds(input.ids), ...parseIds(input.id)])];
}

export function assertNoUnsupportedContinuation(input = {}) {
    if (typeof input.continuation === 'string' && input.continuation.trim() !== '') {
        throw new Error('The "continuation" token is not supported by the Scrappa YouTube channel playlists endpoint.');
    }
}

export function buildChannelPlaylistsUrl({ id, continuation = '' } = {}) {
    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }
    assertNoUnsupportedContinuation({ continuation });

    const params = new URLSearchParams({ channel_id: id });

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
