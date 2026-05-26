import assert from 'node:assert/strict';
import test from 'node:test';

import { getInputUrls, getResponseType } from '../dist/input.js';

test('prefers batched urls while keeping single url compatibility', () => {
    assert.deepEqual(
        getInputUrls({
            url: ' https://example.com ',
            urls: [
                'https://example.com/',
                'https://www.iana.org/domains/reserved',
                '   ',
                'example.org',
            ],
        }),
        [
            {
                input_url: 'https://example.com',
                request_url: 'https://example.com',
            },
            {
                input_url: 'https://www.iana.org/domains/reserved',
                request_url: 'https://www.iana.org/domains/reserved',
            },
            {
                input_url: 'example.org',
                request_url: 'example.org',
            },
        ],
    );
});

test('defaults response_type to json and validates supported values', () => {
    assert.equal(getResponseType(null), 'json');
    assert.equal(getResponseType({ response_type: 'markdown' }), 'markdown');
    assert.throws(
        () => getResponseType({ response_type: 'xml' }),
        /response_type must be either "json" or "markdown"/,
    );
});
