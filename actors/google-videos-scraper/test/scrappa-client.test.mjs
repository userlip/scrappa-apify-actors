import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import test from 'node:test';
import { ScrappaClient } from '../dist/shared/scrappa-client.js';

async function serve(t, handler, timeoutMs = 1000) {
    const server = createServer(handler);
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    t.after(() => { server.closeAllConnections(); server.close(); });
    return new ScrappaClient({
        apiKey: 'test', baseUrl: `http://127.0.0.1:${server.address().port}`, timeoutMs,
    });
}

test('retries a 503 without duplicating a successful response', async (t) => {
    let calls = 0;
    const client = await serve(t, (req, res) => {
        assert.equal(req.headers['x-api-key'], 'test');
        assert.equal(req.url, '/google/videos?q=espresso');
        res.writeHead(++calls === 1 ? 503 : 200, { 'Content-Type': 'application/json' });
        res.end(calls === 1 ? '{}' : '{"video_results":[{"title":"Espresso"}]}');
    });
    assert.equal((await client.get('/google/videos', { q: 'espresso' })).video_results.length, 1);
    assert.equal(calls, 2);
});

test('stops after three transient failures', async (t) => {
    let calls = 0;
    const client = await serve(t, (_, res) => { calls++; res.writeHead(503); res.end('{}'); });
    await assert.rejects(client.get('/google/videos'), /Scrappa API error \(503\)/);
    assert.equal(calls, 3);
});

test('does not retry authentication failures', async (t) => {
    let calls = 0;
    const client = await serve(t, (_, res) => { calls++; res.writeHead(401); res.end('{}'); });
    await assert.rejects(client.get('/google/videos'), /Scrappa API error \(401\)/);
    assert.equal(calls, 1);
});

test('timeout covers a stalled response body and retries remain bounded', async (t) => {
    let calls = 0;
    const client = await serve(t, (_, res) => {
        calls++;
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.write('{');
    }, 50);
    await assert.rejects(client.get('/google/videos'), /timed out after 50ms/);
    assert.equal(calls, 3);
});
