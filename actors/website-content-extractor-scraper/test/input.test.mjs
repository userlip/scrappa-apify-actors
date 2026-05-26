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

test('deduplicates host case without collapsing path or query case', () => {
    assert.deepEqual(
        getInputUrls({
            urls: [
                'https://EXAMPLE.com/Docs',
                'https://example.com/docs',
                'https://example.com/Docs?A=1',
                'https://example.com/Docs?a=1',
                'https://example.com/Docs#section',
            ],
        }),
        [
            {
                input_url: 'https://EXAMPLE.com/Docs',
                request_url: 'https://EXAMPLE.com/Docs',
            },
            {
                input_url: 'https://example.com/docs',
                request_url: 'https://example.com/docs',
            },
            {
                input_url: 'https://example.com/Docs?A=1',
                request_url: 'https://example.com/Docs?A=1',
            },
            {
                input_url: 'https://example.com/Docs?a=1',
                request_url: 'https://example.com/Docs?a=1',
            },
        ],
    );
});

test('validates URL input values before trimming', () => {
    assert.throws(
        () => getInputUrls({ url: 123 }),
        /url must be a string/,
    );
    assert.throws(
        () => getInputUrls({ urls: ['https://example.com', 123] }),
        /urls\[1\] must be a string/,
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
