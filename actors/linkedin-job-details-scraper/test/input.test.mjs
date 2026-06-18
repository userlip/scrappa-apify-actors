import assert from 'node:assert/strict';
import test from 'node:test';

import { getInputUrls } from '../dist/input.js';

test('getInputUrls supports backward-compatible url input', () => {
    assert.deepEqual(
        getInputUrls({ url: 'linkedin.com/jobs/view/1234567890/?trk=foo' }),
        [
            {
                input_url: 'linkedin.com/jobs/view/1234567890/?trk=foo',
                normalized_url: 'https://www.linkedin.com/jobs/view/1234567890',
            },
        ],
    );
});

test('getInputUrls combines url and urls inputs and deduplicates normalized URLs', () => {
    assert.deepEqual(
        getInputUrls({
            url: 'https://linkedin.com/jobs/view/1234567890',
            urls: [
                'https://de.linkedin.com/jobs/view/software-engineer-at-example-2345678901/?trk=foo',
                'https://www.linkedin.com/jobs/view/1234567890/?refId=abc',
            ],
        }),
        [
            {
                input_url: 'https://linkedin.com/jobs/view/1234567890',
                normalized_url: 'https://www.linkedin.com/jobs/view/1234567890',
            },
            {
                input_url: 'https://de.linkedin.com/jobs/view/software-engineer-at-example-2345678901/?trk=foo',
                normalized_url: 'https://www.linkedin.com/jobs/view/software-engineer-at-example-2345678901',
            },
        ],
    );
});

test('getInputUrls keeps invalid URLs as per-item validation failures', () => {
    assert.deepEqual(
        getInputUrls({ urls: ['https://example.com/jobs/view/123'] }),
        [
            {
                input_url: 'https://example.com/jobs/view/123',
                validation_error: 'Invalid LinkedIn job URL. Expected format: https://www.linkedin.com/jobs/view/job-id',
            },
        ],
    );
});
