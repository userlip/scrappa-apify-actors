import test from 'node:test';
import assert from 'node:assert/strict';

import { podcastVideos } from '../src/podcast-filter.js';

test('keeps only videos marked as podcast results', () => {
    const videos = podcastVideos({
        videos: [
            { id: 'flagged', isPodcast: true },
            { id: 'type', type: 'podcast' },
            { id: 'videoType', videoType: 'Podcast episode' },
            { id: 'contentType', contentType: 'podcasts' },
            { id: 'badge', badges: ['Podcast'] },
            { id: 'metadataBadge', metadata: { badges: [{ label: 'Podcast episode' }] } },
            { id: 'regular', type: 'video', title: 'Regular upload' },
        ],
    });

    assert.deepEqual(videos.map((video) => video.id), [
        'flagged',
        'type',
        'videoType',
        'contentType',
        'badge',
        'metadataBadge',
    ]);
});
