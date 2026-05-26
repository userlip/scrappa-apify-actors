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
                input_url: 'https://example.com/',
                request_url: 'https://example.com/',
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

test('deduplicates scheme and host case without merging case-sensitive paths', () => {
    assert.deepEqual(
        getInputUrls({
            urls: [
                'HTTPS://EXAMPLE.COM/Docs',
                'https://example.com/Docs/',
                'https://example.com/docs',
            ],
        }),
        [
            {
                input_url: 'HTTPS://EXAMPLE.COM/Docs',
                request_url: 'HTTPS://EXAMPLE.COM/Docs',
            },
            {
                input_url: 'https://example.com/Docs/',
                request_url: 'https://example.com/Docs/',
            },
            {
                input_url: 'https://example.com/docs',
                request_url: 'https://example.com/docs',
            },
        ],
    );
});

test('deduplicates host case without merging case-sensitive userinfo', () => {
    assert.deepEqual(
        getInputUrls({
            urls: [
                'https://User:PaSs@EXAMPLE.COM/Docs',
                'https://user:pass@example.com/Docs',
                'https://User:PaSs@example.com/Docs',
            ],
        }),
        [
            {
                input_url: 'https://User:PaSs@EXAMPLE.COM/Docs',
                request_url: 'https://User:PaSs@EXAMPLE.COM/Docs',
            },
            {
                input_url: 'https://user:pass@example.com/Docs',
                request_url: 'https://user:pass@example.com/Docs',
            },
        ],
    );
});

test('ignores malformed non-string url entries', () => {
    assert.deepEqual(
        getInputUrls({
            urls: [
                'https://example.com',
                null,
                42,
                { url: 'https://example.org' },
                '   ',
            ],
        }),
        [
            {
                input_url: 'https://example.com',
                request_url: 'https://example.com',
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
