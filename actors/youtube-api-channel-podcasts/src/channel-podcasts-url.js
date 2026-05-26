const API_BASE_URL = 'https://scrappa.co/api/youtube/channel-videos';

export function normalizeSort(sort) {
    if (Array.isArray(sort)) {
        return sort[0];
    }

    return sort;
}

export function parseIds(value) {
    if (Array.isArray(value)) {
        return value.flatMap((item) => parseIds(item));
    }

    if (typeof value !== 'string') {
        return [];
    }

    return value
        .split(',')
        .map((id) => id.trim())
        .filter(Boolean);
}

export function getChannelIds(input = {}) {
    const ids = [...parseIds(input.ids), ...parseIds(input.id)];
    return [...new Set(ids)];
}

export function buildChannelPodcastsUrl({ id, sort, continuation } = {}) {
    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }

    const params = new URLSearchParams({ channel_id: id });
    const normalizedSort = normalizeSort(sort);

    if (normalizedSort && typeof normalizedSort === 'string' && normalizedSort.trim() !== '') {
        params.set('sort', normalizedSort);
    }

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        params.set('continuation', continuation);
    }

    return `${API_BASE_URL}?${params.toString()}`;
}
