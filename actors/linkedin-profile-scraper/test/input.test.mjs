import assert from 'node:assert/strict';
import test from 'node:test';

import { getInputUrls } from '../dist/input.js';

test('getInputUrls supports backward-compatible url input', () => {
    assert.deepEqual(
        getInputUrls({ url: 'linkedin.com/in/williamhgates/?trk=foo' }),
        [
            {
                input_url: 'linkedin.com/in/williamhgates/?trk=foo',
                normalized_url: 'https://www.linkedin.com/in/williamhgates',
            },
        ],
    );
});

test('getInputUrls combines url and urls inputs and deduplicates normalized URLs', () => {
    assert.deepEqual(
        getInputUrls({
            url: 'https://linkedin.com/in/williamhgates',
            urls: [
                'https://www.linkedin.com/in/williamhgates/details/experience/',
                'https://de.linkedin.com/in/satyanadella/?trk=foo',
            ],
        }),
        [
            {
                input_url: 'https://linkedin.com/in/williamhgates',
                normalized_url: 'https://www.linkedin.com/in/williamhgates',
            },
            {
                input_url: 'https://de.linkedin.com/in/satyanadella/?trk=foo',
                normalized_url: 'https://www.linkedin.com/in/satyanadella',
            },
        ],
    );
});

test('getInputUrls keeps the legacy url value first when urls contains the same profile', () => {
    assert.deepEqual(
        getInputUrls({
            url: 'linkedin.com/in/williamhgates/?trk=legacy',
            urls: [
                'https://www.linkedin.com/in/williamhgates/details/contact-info/',
                'https://m.linkedin.com/in/williamhgates',
            ],
        }),
        [
            {
                input_url: 'linkedin.com/in/williamhgates/?trk=legacy',
                normalized_url: 'https://www.linkedin.com/in/williamhgates',
            },
        ],
    );
});

test('getInputUrls keeps invalid URLs as per-item validation failures', () => {
    assert.deepEqual(
        getInputUrls({ urls: ['https://example.com/in/acme'] }),
        [
            {
                input_url: 'https://example.com/in/acme',
                validation_error: 'Invalid LinkedIn profile URL. Expected format: https://www.linkedin.com/in/profile-slug',
            },
        ],
    );
});
