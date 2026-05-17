import assert from 'node:assert/strict';
import test from 'node:test';

import { enrichResult, extractVideoResults } from '../dist/response-utils.js';

test('extracts video results from supported response shapes', () => {
    const result = { position: 1, title: 'Coffee Tutorial' };

    assert.deepEqual(extractVideoResults([result]), [result]);
    assert.deepEqual(extractVideoResults({ video_results: [result] }), [result]);
    assert.deepEqual(extractVideoResults({ data: [result] }), [result]);
});

test('returns an empty result set for unexpected response shapes', () => {
    const originalWarn = console.warn;
    const warnings = [];
    console.warn = (message) => warnings.push(message);

    try {
        assert.deepEqual(extractVideoResults({ results: [{ position: 1 }] }), []);
        assert.deepEqual(extractVideoResults(null), []);
        assert.deepEqual(extractVideoResults('unexpected'), []);
        assert.deepEqual(warnings, [
            'Scrappa Google Videos response did not include a video result array',
            'Scrappa Google Videos response did not include a video result array',
            'Scrappa Google Videos response did not include a video result array',
        ]);
    } finally {
        console.warn = originalWarn;
    }
});

test('enriches video results with dataset aliases and request metadata', () => {
    assert.deepEqual(
        enrichResult(
            {
                position: 1,
                title: 'Coffee Tutorial',
                link: 'https://www.youtube.com/watch?v=example',
                video_link: 'https://www.google.com/url?q=https://www.youtube.com/watch?v=example',
                thumbnail: 'https://example.com/thumb.jpg',
                key_moments: [
                    { time: '00:00', title: 'Intro' },
                    { time: '02:30', title: 'Setup' },
                ],
            },
            {
                q: 'coffee',
                page: 1,
                hl: 'en',
                gl: 'us',
                google_domain: 'google.com',
                safe: 'off',
            },
        ),
        {
            position: 1,
            title: 'Coffee Tutorial',
            link: 'https://www.youtube.com/watch?v=example',
            video_link: 'https://www.google.com/url?q=https://www.youtube.com/watch?v=example',
            thumbnail: 'https://example.com/thumb.jpg',
            key_moments: [
                { time: '00:00', title: 'Intro' },
                { time: '02:30', title: 'Setup' },
            ],
            video_url: 'https://www.youtube.com/watch?v=example',
            google_redirect_url: 'https://www.google.com/url?q=https://www.youtube.com/watch?v=example',
            source_url: 'https://www.youtube.com/watch?v=example',
            displayed_link: null,
            thumbnail_url: 'https://example.com/thumb.jpg',
            snippet: null,
            duration: null,
            date: null,
            key_moments_count: 2,
            request_q: 'coffee',
            request_page: 1,
            request_start: null,
            request_hl: 'en',
            request_gl: 'us',
            request_google_domain: 'google.com',
            request_location: null,
            request_uule: null,
            request_tbs: null,
            request_safe: 'off',
            request_filter: null,
            request_nfpr: null,
            request_lr: null,
        },
    );
});
