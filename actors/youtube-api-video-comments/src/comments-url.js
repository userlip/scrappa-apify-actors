const API_BASE_URL = 'https://ytapi.scrappa.co/videos/comments';

function singleValue(value) {
    const rawValue = Array.isArray(value) ? value[0] : value;

    if (typeof rawValue !== 'string') {
        return undefined;
    }

    const trimmedValue = rawValue.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

export function buildVideoCommentsUrl(input = {}) {
    const actorInput = input && typeof input === 'object' ? input : {};
    const id = singleValue(actorInput.id);

    if (!id) {
        throw new Error('YouTube video ID "id" is required.');
    }

    const params = new URLSearchParams({ id });
    const sort = singleValue(actorInput.sort);
    const continuation = singleValue(actorInput.continuation);

    if (sort) {
        params.set('sort', sort);
    }

    if (continuation) {
        params.set('continuation', continuation);
    }

    return `${API_BASE_URL}?${params.toString()}`;
}
