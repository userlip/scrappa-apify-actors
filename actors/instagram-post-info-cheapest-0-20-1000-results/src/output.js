function isPlainObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function getInstagramPostPayload(response) {
    if (!isPlainObject(response)) {
        return null;
    }

    const candidates = [
        response.data?.post,
        response.data?.media,
        response.data?.item,
        response.post,
        response.media,
        response.item,
        response.data,
    ];

    return candidates.find(isPlainObject) ?? null;
}

export function flattenInstagramPostResponse(response) {
    const post = getInstagramPostPayload(response);

    if (!post) {
        return response;
    }

    const metadata = {};
    for (const key of ['success', 'message', 'error']) {
        if (Object.hasOwn(response, key)) {
            metadata[key] = response[key];
        }
    }

    return {
        ...metadata,
        ...post,
    };
}
