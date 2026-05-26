export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const API_BASE_URL = 'https://scrappa.co/api/youtube/chapters';

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

export function getVideoIds(input = {}) {
    return [...new Set([...parseIds(input.ids), ...parseIds(input.id)])];
}

export function buildVideoChaptersUrl(id) {
    if (!id) {
        throw new Error('Video "id" not provided in input.');
    }

    const params = new URLSearchParams({ video_id: id });
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
