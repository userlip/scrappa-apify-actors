export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const API_BASE_URL = 'https://scrappa.co/api/youtube/channel-videos';
const DEFAULT_TARGET_RESULT_COUNT = 10;
const MAX_FILTER_SCAN_PAGES = 10;

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

export function assertContinuationMatchesBatch(input = {}, ids = getChannelIds(input)) {
    if (ids.length > 1 && typeof input.continuation === 'string' && input.continuation.trim() !== '') {
        throw new Error('The "continuation" token can only be used with a single YouTube channel ID.');
    }
}

export function buildChannelLivestreamsUrl({ id, sort, continuation = '' } = {}) {
    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }

    const params = new URLSearchParams({ channel_id: id });
    if (sort && typeof sort === 'string' && sort.trim() !== '') {
        params.set('sort', sort);
    }
    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        params.set('continuation', continuation);
    }

    return `${API_BASE_URL}?${params.toString()}`;
}

export function responseVideos(responseData = {}) {
    return Array.isArray(responseData.videos) ? responseData.videos : [];
}

export function responseContinuation(responseData = {}) {
    return responseData.continuation
        ?? responseData.continuationToken
        ?? responseData.pagination?.continuation
        ?? responseData.pagination?.continuationToken
        ?? '';
}

export async function collectFilteredChannelVideos(input, fetchPage, filterVideo, {
    targetResultCount = DEFAULT_TARGET_RESULT_COUNT,
    maxPages = MAX_FILTER_SCAN_PAGES,
} = {}) {
    const videos = [];
    let continuation = input.continuation ?? '';
    let nextContinuation = '';

    for (let page = 0; page < maxPages && videos.length < targetResultCount; page += 1) {
        const responseData = await fetchPage({ ...input, continuation });
        videos.push(...responseVideos(responseData).filter(filterVideo));

        nextContinuation = responseContinuation(responseData);
        if (!nextContinuation || nextContinuation === continuation) {
            break;
        }

        continuation = nextContinuation;
    }

    return { videos, continuation: nextContinuation };
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
