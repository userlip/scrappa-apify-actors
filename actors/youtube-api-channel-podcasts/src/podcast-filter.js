function normalizedText(value) {
    if (typeof value === 'string') {
        return value.toLowerCase();
    }

    if (value && typeof value === 'object') {
        return Object.values(value).map(normalizedText).join(' ');
    }

    return '';
}

function includesPodcast(value) {
    return normalizedText(value).includes('podcast');
}

export function isPodcastVideo(video = {}) {
    if (video.isPodcast === true) {
        return true;
    }

    const directFields = [
        video.type,
        video.videoType,
        video.contentType,
        video.category,
        video.playlistType,
    ];
    if (directFields.some(includesPodcast)) {
        return true;
    }

    const badges = [
        ...(Array.isArray(video.badges) ? video.badges : []),
        ...(Array.isArray(video.metadata?.badges) ? video.metadata.badges : []),
    ];

    return badges.some(includesPodcast);
}

export function podcastVideos(responseData = {}) {
    return (responseData?.videos ?? []).filter(isPodcastVideo);
}
