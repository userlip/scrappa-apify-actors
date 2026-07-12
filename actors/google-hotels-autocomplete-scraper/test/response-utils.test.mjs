import assert from 'node:assert/strict';
import test from 'node:test';

const sourceRoot = process.env.TEST_SOURCE === 'src' ? '../src' : '../dist';
const { buildSuggestionDatasetItems } = await import(`${sourceRoot}/response-utils.js`);

test('maps suggestions with source context and nullable optional fields', () => {
    const items = buildSuggestionDatasetItems({
        suggestions: [{ position: 1, value: 'Berlin hotels', type: 'location' }],
        response_time_ms: 935,
    }, 'Berlin', { gl: 'de', hl: 'en', currency: 'EUR', type: 'all' });

    assert.deepEqual(items, [{
        position: 1,
        value: 'Berlin hotels',
        type: 'location',
        autocomplete_suggestion: null,
        highlighted_words: [],
        property_token: null,
        thumbnail: null,
        scrappa_google_hotels_link: null,
        source_query: 'Berlin',
        request_gl: 'de',
        request_hl: 'en',
        request_currency: 'EUR',
        request_type: 'all',
        response_time_ms: 935,
    }]);
});

test('deduplicates equivalent suggestions and drops empty suggestions', () => {
    const items = buildSuggestionDatasetItems({ suggestions: [
        { value: 'Park Inn', type: 'accommodation', property_token: 'token-1' },
        { value: 'Renamed Park Inn', type: 'accommodation', property_token: 'TOKEN-1' },
        { type: 'location' },
    ] }, 'park', {});

    assert.equal(items.length, 1);
    assert.equal(items[0].value, 'Park Inn');
    assert.equal(items[0].source_query, 'park');
});

test('returns no rows for an unexpected response shape', () => {
    assert.deepEqual(buildSuggestionDatasetItems({}, 'Berlin', {}), []);
});
