import test from 'node:test';
import assert from 'node:assert/strict';
import { buildLinkedInCompanyParams } from './request-params.js';

test('buildLinkedInCompanyParams omits cache fields when caching is disabled', () => {
    assert.deepEqual(
        buildLinkedInCompanyParams({
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: false,
            maximum_cache_age: 86400,
        }),
        { url: 'https://www.linkedin.com/company/microsoft' },
    );
});

test('buildLinkedInCompanyParams forwards valid cache fields when caching is enabled', () => {
    assert.deepEqual(
        buildLinkedInCompanyParams({
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: true,
            maximum_cache_age: 86400,
        }),
        {
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: 1,
            maximum_cache_age: 86400,
        },
    );
});

test('buildLinkedInCompanyParams omits invalid cache age values', () => {
    assert.deepEqual(
        buildLinkedInCompanyParams({
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: true,
            maximum_cache_age: 0,
        }),
        {
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: 1,
        },
    );

    assert.deepEqual(
        buildLinkedInCompanyParams({
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: true,
            maximum_cache_age: 42.5,
        }),
        {
            url: 'https://www.linkedin.com/company/microsoft',
            use_cache: 1,
        },
    );
});
