export function firstNonEmptyString(...values) {
    return values.find((value) => typeof value === 'string' && value.trim() !== '')?.trim();
}

export function looksLikeUrl(value) {
    return /^(?:https?:\/\/|www\.)/i.test(value);
}

export function resolveInstagramPostInput(input = {}) {
    input = input ?? {};

    const url = firstNonEmptyString(input.url);
    const shortcode = firstNonEmptyString(input.shortcode, input.media_id);
    const identifier = url ?? shortcode;

    if (!identifier) {
        throw new Error('Instagram post URL or shortcode is required. Provide url, shortcode, or media_id in the input.');
    }

    return {
        identifier,
        params: url && looksLikeUrl(url) ? { url } : { shortcode: identifier },
    };
}
