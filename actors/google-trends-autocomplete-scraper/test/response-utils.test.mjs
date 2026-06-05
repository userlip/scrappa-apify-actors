import assert from 'node:assert/strict';
import test from 'node:test';

import { buildAutocompleteDatasetItems } from '../dist/response-utils.js';

test('builds one dataset item per string suggestion', () => {
    const items = buildAutocompleteDatasetItems(
        {
            search_parameters: { q: 'tesla', geo: 'US', hl: 'en' },
            suggestions: ['tesla stock', 'tesla model y'],
            response_time_ms: 623,
        },
        {
            q: 'tesla',
            geo: 'US',
            hl: 'en',
        },
    );

    assert.equal(items.length, 2);
    assert.deepEqual(items[0], {
        suggestion: 'tesla stock',
        position: 1,
        type: null,
        source_keyword: 'tesla',
        request_geo: 'US',
        request_hl: 'en',
        response_time_ms: 623,
        search_parameters: { q: 'tesla', geo: 'US', hl: 'en' },
    });
    assert.equal(items[1].suggestion, 'tesla model y');
    assert.equal(items[1].position, 2);
});

test('preserves object suggestion fields and normalizes common names', () => {
    const items = buildAutocompleteDatasetItems(
        {
            suggestions: [
                { query: 'tesla price', type: 'query', extra: 'kept' },
                { title: 'Tesla, Inc.', type: 'company' },
                { value: 'Tesla Model Y', type: 'SUV', id: '/g/11gb_4f22x' },
            ],
        },
        { q: 'tesla', geo: 'US', hl: 'en' },
    );

    assert.equal(items.length, 3);
    assert.equal(items[0].query, 'tesla price');
    assert.equal(items[0].suggestion, 'tesla price');
    assert.equal(items[0].type, 'query');
    assert.equal(items[0].extra, 'kept');
    assert.equal(items[1].suggestion, 'Tesla, Inc.');
    assert.equal(items[1].type, 'company');
    assert.equal(items[2].value, 'Tesla Model Y');
    assert.equal(items[2].suggestion, 'Tesla Model Y');
    assert.equal(items[2].id, '/g/11gb_4f22x');
});

test('accepts nested response containers used by autocomplete APIs', () => {
    assert.deepEqual(
        buildAutocompleteDatasetItems({ data: { suggestions: ['coffee shop'] } }, { q: 'coffee' })
            .map((item) => item.suggestion),
        ['coffee shop'],
    );

    assert.deepEqual(
        buildAutocompleteDatasetItems({ autocomplete: { results: [{ keyword: 'coffee beans' }] } }, { q: 'coffee' })
            .map((item) => item.suggestion),
        ['coffee beans'],
    );
});

test('returns no items when response has no suggestion arrays', () => {
    assert.deepEqual(buildAutocompleteDatasetItems({}, { q: 'coffee' }), []);
});
