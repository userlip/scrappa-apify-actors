export function buildRelatedVideosUrl({ id } = {}) {
    if (!id) {
        throw new Error('YouTube video ID "id" not provided. Please provide a value for "id" in the input.');
    }

    return `https://ytapi.scrappa.co/videos/related?id=${encodeURIComponent(id)}`;
}
