import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { getScrappaApiKey } from '../src/api-key.js';

describe('getScrappaApiKey', () => {
    it('returns SCRAPPA_API_KEY from the provided environment', () => {
        assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    });

    it('fails with an actionable error when SCRAPPA_API_KEY is missing', () => {
        assert.throws(
            () => getScrappaApiKey({}),
            /SCRAPPA_API_KEY environment variable is not set/
        );
    });
});
