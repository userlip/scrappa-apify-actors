import assert from 'node:assert/strict';
import test from 'node:test';

import { buildSearchParams } from '../dist/search-params.js';

test('builds search params with default language and cache settings', () => {
    assert.deepEqual(buildSearchParams({ query: 'pizza' }), {
        query: 'pizza',
        hl: 'en',
        use_cache: 1,
    });
});

test('preserves an empty language value for schema validation to reject', () => {
    assert.deepEqual(buildSearchParams({ query: 'pizza', hl: '' }), {
        query: 'pizza',
        hl: '',
        use_cache: 1,
    });
});

test('omits cache age when cache is disabled', () => {
    assert.deepEqual(
        buildSearchParams({
            query: 'pizza',
            hl: 'de',
            gl: 'de',
            debug: false,
            use_cache: false,
            maximum_cache_age: 3600,
        }),
        {
            query: 'pizza',
            hl: 'de',
            gl: 'de',
            debug: false,
        }
    );
});

test('includes cache age when cache is enabled', () => {
    assert.deepEqual(
        buildSearchParams({
            query: 'pizza',
            maximum_cache_age: 3600,
        }),
        {
            query: 'pizza',
            hl: 'en',
            use_cache: 1,
            maximum_cache_age: 3600,
        }
    );
});

test('omits zero and invalid cache ages when cache is enabled', () => {
    for (const maximum_cache_age of [0, -1, 1.5, '3600', null, Number.NaN]) {
        assert.deepEqual(
            buildSearchParams({
                query: 'pizza',
                maximum_cache_age,
            }),
            {
                query: 'pizza',
                hl: 'en',
                use_cache: 1,
            }
        );
    }
});
