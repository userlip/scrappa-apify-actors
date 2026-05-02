const API_BASE_URL = 'https://ytapi.scrappa.co/videos';

function stringValue(value) {
    if (typeof value !== 'string') {
        return undefined;
    }

    const trimmedValue = value.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

export function buildVideoDetailsUrl(input = {}) {
    const actorInput = input && typeof input === 'object' ? input : {};
    const id = stringValue(actorInput.id);

    if (!id) {
        throw new Error('YouTube video ID "id" is required.');
    }

    const params = new URLSearchParams({ id });

    return `${API_BASE_URL}?${params.toString()}`;
}
