import assert from 'node:assert/strict';
import test from 'node:test';

import { getLinkedInSearchResults } from '../dist/search-response.js';

test('returns organic results from LinkedIn search response', () => {
    const results = [
        {
            position: 1,
            title: 'Jane Founder - AI Startup',
            link: 'https://www.linkedin.com/in/example-founder',
        },
    ];

    assert.deepEqual(getLinkedInSearchResults({ organic_results: results }), results);
});

test('returns an empty array when organic results are missing', () => {
    assert.deepEqual(getLinkedInSearchResults({}), []);
});

test('returns an empty array when organic results are invalid', () => {
    assert.deepEqual(getLinkedInSearchResults({ organic_results: null }), []);
});
