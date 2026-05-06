export function normalizeSort(sort) {
    if (Array.isArray(sort)) {
        return sort[0];
    }

    return sort;
}

export function buildChannelVideosUrl({ id, sort, continuation } = {}) {
    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }

    let apiUrl = `https://ytapi.scrappa.co/channels/videos?id=${encodeURIComponent(id)}`;
    const normalizedSort = normalizeSort(sort);

    if (normalizedSort && typeof normalizedSort === 'string' && normalizedSort.trim() !== '') {
        apiUrl += `&sort=${encodeURIComponent(normalizedSort)}`;
    }

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        apiUrl += `&continuation=${encodeURIComponent(continuation)}`;
    }

    return apiUrl;
}
