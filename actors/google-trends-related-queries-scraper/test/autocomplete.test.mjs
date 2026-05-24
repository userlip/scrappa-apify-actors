import assert from 'node:assert/strict';
import test from 'node:test';

import { fetchAutocompleteSummary } from '../dist/autocomplete.js';

test('fetches autocomplete summary with supported request params', async () => {
    const calls = [];
    const client = {
        async get(endpoint, params, options) {
            calls.push({ endpoint, params, options });
            return { suggestions: ['coffee shop'] };
        },
    };

    const summary = await fetchAutocompleteSummary(
        client,
        {
            q: 'coffee',
            geo: 'US',
            hl: 'en',
            time_range: '1y',
            search_type: 'web',
        },
        3,
    );

    assert.deepEqual(summary, {
        response: { suggestions: ['coffee shop'] },
        error: null,
    });
    assert.deepEqual(calls, [
        {
            endpoint: '/google-trends/autocomplete',
            params: { q: 'coffee', geo: 'US', hl: 'en' },
            options: { attempts: 3 },
        },
    ]);
});

test('keeps autocomplete failures non-fatal', async () => {
    const warnings = [];
    const originalWarn = console.warn;
    console.warn = (message) => warnings.push(message);

    try {
        const client = {
            async get() {
                throw new Error('Scrappa API error (503): temporarily busy');
            },
        };

        const summary = await fetchAutocompleteSummary(client, { q: 'coffee' }, 3);

        assert.deepEqual(summary, {
            response: null,
            error: 'Scrappa API error (503): temporarily busy',
        });
        assert.equal(
            warnings[0],
            'Google Trends autocomplete summary failed: Scrappa API error (503): temporarily busy',
        );
    } finally {
        console.warn = originalWarn;
    }
});
