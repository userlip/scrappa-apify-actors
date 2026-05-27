export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const API_BASE_URL = 'https://scrappa.co/api/youtube/channel';

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

export function buildChannelAboutDetailsUrl(id) {
    if (!id) {
        throw new Error('Channel "id" not provided in input.');
    }

    const params = new URLSearchParams({ channel_id: id });
    return `${API_BASE_URL}?${params.toString()}`;
}

function splitSubscriberAndVideoCount(value) {
    if (typeof value !== 'string') {
        return { subscriberCount: null, videoCount: null };
    }

    const match = value.match(/^(.*? subscribers)(?:\s+(.+? videos))?$/);
    if (!match) {
        return { subscriberCount: value, videoCount: null };
    }

    return {
        subscriberCount: match[1],
        videoCount: match[2] ?? null,
    };
}

export function toChannelAboutDetails(data = {}) {
    const channelId = data.channelId ?? data.id ?? null;
    const counts = splitSubscriberAndVideoCount(data.subscriberCount);

    return {
        channelId,
        stats: {
            joinDate: data.joinedDate ?? null,
            viewCount: data.viewCount ?? null,
            country: data.country ?? null,
        },
        links: Array.isArray(data.links) ? data.links : [],
        details: {
            description: data.description ?? null,
            email: data.email ?? null,
            name: data.name ?? null,
            subscriberCount: counts.subscriberCount,
            videoCount: data.videoCount ?? counts.videoCount,
            channelUrl: data.channelUrl ?? data.url ?? (channelId ? `https://www.youtube.com/channel/${channelId}` : null),
        },
    };
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
