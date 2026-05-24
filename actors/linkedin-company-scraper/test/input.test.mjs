import assert from 'node:assert/strict';
import test from 'node:test';

import { getInputUrls } from '../dist/input.js';

test('getInputUrls supports backward-compatible url input', () => {
    assert.deepEqual(
        getInputUrls({ url: 'linkedin.com/company/microsoft/?trk=foo' }),
        [
            {
                input_url: 'linkedin.com/company/microsoft/?trk=foo',
                normalized_url: 'https://www.linkedin.com/company/microsoft',
            },
        ],
    );
});

test('getInputUrls combines url and urls inputs and deduplicates normalized URLs', () => {
    assert.deepEqual(
        getInputUrls({
            url: 'https://linkedin.com/company/microsoft',
            urls: [
                'https://www.linkedin.com/company/microsoft/about/',
                'https://m.linkedin.com/company/openai/?trk=foo',
            ],
        }),
        [
            {
                input_url: 'https://linkedin.com/company/microsoft',
                normalized_url: 'https://www.linkedin.com/company/microsoft',
            },
            {
                input_url: 'https://m.linkedin.com/company/openai/?trk=foo',
                normalized_url: 'https://www.linkedin.com/company/openai',
            },
        ],
    );
});

test('getInputUrls keeps invalid URLs as per-item validation failures', () => {
    assert.deepEqual(
        getInputUrls({ urls: ['https://example.com/company/acme'] }),
        [
            {
                input_url: 'https://example.com/company/acme',
                validation_error: 'Invalid LinkedIn company URL. Expected format: https://www.linkedin.com/company/company-slug',
            },
        ],
    );
});
