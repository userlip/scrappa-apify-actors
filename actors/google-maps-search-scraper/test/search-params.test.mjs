import assert from 'node:assert/strict';
import test from 'node:test';

import { buildSearchParams } from '../dist/search-params.js';

test('builds search params with default language and cache settings', () => {
    assert.deepEqual(buildSearchParams({ query: 'pizza' }), {
        query: 'pizza',
        hl: 'en',
        gl: undefined,
        debug: undefined,
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
            gl: undefined,
            debug: undefined,
            use_cache: 1,
            maximum_cache_age: 3600,
        }
    );
});
