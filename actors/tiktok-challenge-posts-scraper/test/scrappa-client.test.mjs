import assert from 'node:assert/strict';
import test from 'node:test';
import { ScrappaClient } from '../dist/shared/scrappa-client.js';

test('authenticates with the Scrappa X-API-Key contract', async (context) => {
    context.mock.method(globalThis, 'fetch', async (url, options) => {
        assert.equal(url.searchParams.get('challenge_id'), '123');
        assert.equal(options.headers['X-API-Key'], 'secret');
        assert.equal(Object.hasOwn(options.headers, 'Authorization'), false);
        return new Response(JSON.stringify({ code: 0 }), { status: 200 });
    });

    const response = await new ScrappaClient('secret').get('/tiktok/challenges/posts', { challenge_id: '123' });
    assert.deepEqual(response, { code: 0 });
});
