const API_BASE_URL = 'https://scrappa.co/api/youtube/transcript';

function singleValue(value) {
    const rawValue = Array.isArray(value) ? value[0] : value;

    if (typeof rawValue !== 'string') {
        return undefined;
    }

    const trimmedValue = rawValue.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

function boolValue(value) {
    const rawValue = Array.isArray(value) ? value[0] : value;

    if (typeof rawValue === 'boolean') {
        return rawValue;
    }

    if (typeof rawValue === 'string') {
        const normalizedValue = rawValue.trim().toLowerCase();
        if (['1', 'true', 'yes'].includes(normalizedValue)) {
            return true;
        }

        if (['0', 'false', 'no', ''].includes(normalizedValue)) {
            return false;
        }
    }

    return false;
}

export function buildTranscriptUrl(input = {}) {
    const actorInput = input && typeof input === 'object' ? input : {};
    const id = singleValue(actorInput.id);

    if (!id) {
        throw new Error('YouTube video ID "id" is required.');
    }

    const params = new URLSearchParams({ video_id: id });
    const language = singleValue(actorInput.language);
    const lang = singleValue(actorInput.lang);
    const hl = singleValue(actorInput.hl);
    const gl = singleValue(actorInput.gl);

    if (language) {
        params.set('language', language);
    }

    if (lang) {
        params.set('lang', lang);
    }

    if (hl) {
        params.set('hl', hl.toLowerCase());
    }

    if (gl) {
        params.set('gl', gl.toUpperCase());
    }

    if (boolValue(actorInput.debug)) {
        params.set('debug', '1');
    }

    return `${API_BASE_URL}?${params.toString()}`;
}
