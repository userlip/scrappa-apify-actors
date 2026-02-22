import assert from 'node:assert/strict';
import test from 'node:test';

import { fetchWithFallback } from '../dist/fetch-with-fallback.js';

test('falls back to advanced search when simple search fails transiently', async () => {
    const calls = [];
    const client = {
        async get(endpoint, params) {
            calls.push({ endpoint, params });
            if (endpoint === '/maps/simple-search') {
                throw new Error('Scrappa API error (504): Gateway Time-out');
            }

            return { items: [{ name: "Joe's Pizza" }] };
        },
    };

    const baseParams = { query: "joe's pizza manhattan", hl: 'en', gl: 'us' };
    const response = await fetchWithFallback(client, baseParams, 13);

    assert.equal(calls.length, 2);
    assert.equal(calls[0].endpoint, '/maps/simple-search');
    assert.equal(calls[1].endpoint, '/maps/advanced-search');
    assert.equal(calls[1].params.zoom, 13);
    assert.equal(response.items.length, 1);
});

test('does not fall back for non-transient errors', async () => {
    const client = {
        async get() {
            throw new Error('Scrappa API error (422): Validation failed');
        },
    };

    const baseParams = { query: 'pizza', hl: 'en' };
    await assert.rejects(() => fetchWithFallback(client, baseParams, 13), /422/);
});
