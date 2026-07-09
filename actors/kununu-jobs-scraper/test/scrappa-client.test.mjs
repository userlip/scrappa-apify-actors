import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient } from '../dist/shared/index.js';

test('serializes boolean GET parameters as Laravel-compatible query values', async () => {
    const originalFetch = globalThis.fetch;
    const requestedUrls = [];

    globalThis.fetch = async (url) => {
        requestedUrls.push(String(url));

        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await client.get('/kununu/jobs', {
            query: 'software engineer',
            is_top_company: false,
        });
        await client.get('/kununu/jobs', {
            query: 'software engineer',
            is_top_company: true,
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(new URL(requestedUrls[0]).searchParams.get('is_top_company'), '0');
    assert.equal(new URL(requestedUrls[1]).searchParams.get('is_top_company'), '1');
});

test('serializes array GET parameters with bracket notation', async () => {
    const originalFetch = globalThis.fetch;
    const requestedUrls = [];

    globalThis.fetch = async (url) => {
        requestedUrls.push(String(url));

        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await client.get('/kununu/jobs', {
            workplace: ['FULL_REMOTE', 'PARTLY_REMOTE'],
            benefits: ['flexWorkingHours', 'pensionPlan'],
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    const params = new URL(requestedUrls[0]).searchParams;
    assert.deepEqual(params.getAll('workplace[]'), ['FULL_REMOTE', 'PARTLY_REMOTE']);
    assert.deepEqual(params.getAll('benefits[]'), ['flexWorkingHours', 'pensionPlan']);
});
