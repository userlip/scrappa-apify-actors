import assert from 'node:assert/strict';
import test from 'node:test';

import { getJobSearchResults } from '../dist/search-response.js';

test('returns organic results from LinkedIn jobs search response', () => {
    const results = [
        {
            position: 1,
            title: 'Software Engineer',
            link: 'https://www.linkedin.com/jobs/view/123',
        },
    ];

    assert.deepEqual(getJobSearchResults({ organic_results: results }), results);
});

test('returns an empty array when organic results are missing', () => {
    assert.deepEqual(getJobSearchResults({}), []);
});

test('returns an empty array when organic results are invalid', () => {
    assert.deepEqual(getJobSearchResults({ organic_results: null }), []);
});
