const API_BASE_URL = 'https://ytapi.scrappa.co/videos/batch';

function stringValue(value) {
    if (typeof value !== 'string') {
        return undefined;
    }

    const trimmedValue = value.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

export function buildBatchVideosUrl(input = {}) {
    const actorInput = input && typeof input === 'object' ? input : {};
    const ids = stringValue(actorInput.ids);

    if (!ids) {
        throw new Error('Video IDs "ids" are required.');
    }

    const normalizedIds = ids.split(',').map((id) => id.trim()).filter(Boolean);
    if (normalizedIds.length > 50) {
        throw new Error('Video IDs "ids" must contain 50 or fewer comma-separated IDs.');
    }

    const params = new URLSearchParams({ ids: normalizedIds.join(',') });

    return `${API_BASE_URL}?${params.toString()}`;
}
