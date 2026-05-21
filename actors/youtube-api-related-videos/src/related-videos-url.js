export function buildRelatedVideosUrl({ id, continuation } = {}) {
    if (!id) {
        throw new Error('YouTube video ID "id" not provided. Please provide a value for "id" in the input.');
    }

    let apiUrl = `https://ytapi.scrappa.co/videos/related?id=${encodeURIComponent(id)}`;
    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        apiUrl += `&continuation=${encodeURIComponent(continuation)}`;
    }

    return apiUrl;
}
