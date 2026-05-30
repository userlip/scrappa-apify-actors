import assert from 'node:assert/strict';
import test from 'node:test';

import { buildStartpageDatasetItem, extractStartpageOrganicResults } from '../dist/response-utils.js';

test('extracts organic results from supported Startpage response shapes', () => {
    const result = { position: 1, title: 'Privacy Tools' };

    assert.deepEqual(extractStartpageOrganicResults([result]), [result]);
    assert.deepEqual(extractStartpageOrganicResults({ data: [result] }), [result]);
    assert.deepEqual(extractStartpageOrganicResults({ organic_results: [result] }), [result]);
    assert.deepEqual(extractStartpageOrganicResults({ results: [result] }), [result]);
});

test('returns an empty result set for unexpected response shapes', () => {
    const originalWarn = console.warn;
    const warnings = [];
    console.warn = (message) => warnings.push(message);

    try {
        assert.deepEqual(extractStartpageOrganicResults({ items: [{ position: 1 }] }), []);
        assert.deepEqual(extractStartpageOrganicResults(null), []);
        assert.deepEqual(warnings, [
            'Scrappa Startpage response did not include an organic result array',
            'Scrappa Startpage response did not include an organic result array',
        ]);
    } finally {
        console.warn = originalWarn;
    }
});

test('builds dataset item with request and pagination metadata', () => {
    assert.deepEqual(
        buildStartpageDatasetItem(
            {
                position: 1,
                title: 'Privacy Tools',
                description: 'Private search result',
                url: 'https://www.privacytools.io/',
                domain: 'www.privacytools.io',
            },
            {
                query: 'privacy tools',
                language: 'english',
                page: 0,
                safe_search: 1,
            },
            {
                total_results: 20,
                source: 'startpage',
                pagination: { current: 0 },
                scrappa_pagination: { page: 0 },
            },
        ),
        {
            position: 1,
            title: 'Privacy Tools',
            description: 'Private search result',
            url: 'https://www.privacytools.io/',
            domain: 'www.privacytools.io',
            query: 'privacy tools',
            source: 'startpage',
            request_query: 'privacy tools',
            request_language: 'english',
            request_page: 0,
            request_safe_search: 1,
            total_results: 20,
            pagination: { current: 0 },
            scrappa_pagination: { page: 0 },
        },
    );
});
