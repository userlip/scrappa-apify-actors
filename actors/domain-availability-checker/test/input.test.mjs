import assert from 'node:assert/strict';
import test from 'node:test';

import { getDomainRequests } from '../dist/input.js';

test('getDomainRequests supports backward-compatible domain input', () => {
    assert.deepEqual(
        getDomainRequests({ domain: ' https://Example.com/path?x=1 ' }),
        [
            {
                input_domain: 'https://Example.com/path?x=1',
                domain: 'example.com',
            },
        ],
    );
});

test('getDomainRequests combines domain and domains inputs and deduplicates normalized domains', () => {
    assert.deepEqual(
        getDomainRequests({
            domain: 'example.com',
            domains: ['https://example.com/pricing', 'Sub.Example.org.', ''],
        }),
        [
            {
                input_domain: 'example.com',
                domain: 'example.com',
            },
            {
                input_domain: 'Sub.Example.org.',
                domain: 'sub.example.org',
            },
        ],
    );
});

test('getDomainRequests keeps invalid domains as per-item failures', () => {
    assert.deepEqual(
        getDomainRequests({ domains: ['localhost', 'bad_domain.com'] }),
        [
            {
                input_domain: 'localhost',
                validation_error: 'Invalid domain "localhost". Provide a fully qualified domain such as example.com.',
            },
            {
                input_domain: 'bad_domain.com',
                validation_error: 'Invalid domain "bad_domain.com". Provide a valid fully qualified domain name.',
            },
        ],
    );
});

test('getDomainRequests returns no requests for empty input', () => {
    assert.deepEqual(getDomainRequests(null), []);
    assert.deepEqual(getDomainRequests({}), []);
});

test('getDomainRequests preserves distinct invalid raw inputs', () => {
    assert.deepEqual(
        getDomainRequests({ domains: [' localhost ', 'localhost'] }),
        [
            {
                input_domain: 'localhost',
                validation_error: 'Invalid domain "localhost". Provide a fully qualified domain such as example.com.',
            },
            {
                input_domain: 'localhost',
                validation_error: 'Invalid domain "localhost". Provide a fully qualified domain such as example.com.',
            },
        ],
    );
});

test('getDomainRequests rejects a non-array domains field', () => {
    assert.throws(
        () => getDomainRequests({ domains: 'example.com' }),
        /domains must be an array of strings/,
    );
});
