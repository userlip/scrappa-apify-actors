import assert from 'node:assert/strict';
import test from 'node:test';

const mainModule = process.env.TEST_SOURCE === 'src'
    ? '../src/main.ts'
    : '../dist/main.js';
const sharedModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';

const { isHandledEmptySearchError } = await import(mainModule);
const { ScrappaApiError } = await import(sharedModule);

test('handles only location-related 400 responses as empty ImmobilienScout24 searches', () => {
    assert.equal(
        isHandledEmptySearchError(new ScrappaApiError(400, "Location 'Berlin' not found")),
        true,
    );
    assert.equal(
        isHandledEmptySearchError(new ScrappaApiError(400, 'INVALID_LOCATION')),
        true,
    );
    assert.equal(
        isHandledEmptySearchError(new ScrappaApiError(400, 'Bad Request')),
        false,
    );
    assert.equal(
        isHandledEmptySearchError(new ScrappaApiError(400, 'type must be one of: apartment-rent, apartment-buy')),
        false,
    );
});

test('handles Scrappa 502 responses as clean zero-result search responses', () => {
    assert.equal(isHandledEmptySearchError(new ScrappaApiError(502, 'Bad Gateway')), true);
});
