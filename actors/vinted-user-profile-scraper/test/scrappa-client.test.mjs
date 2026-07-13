import test from 'node:test';
import assert from 'node:assert/strict';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const { ScrappaClient } = await import(`../${sourceDirectory}/shared/scrappa-client.js`);

test('calls only the configured Scrappa endpoint and retries transient failures', async () => {
    const originalFetch = globalThis.fetch;
    const originalRandom = Math.random;
    const calls = [];
    let attempts = 0;
    Math.random = () => 0;

    globalThis.fetch = async (url, options) => {
        calls.push({ url: String(url), options });
        attempts += 1;
        if (attempts === 1) {
            return new Response('temporary outage', { status: 503, statusText: 'Unavailable' });
        }

        return Response.json({ success: true, data: { user: { id: 255914028 } } });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test-key', baseUrl: 'https://scrappa.test/api' });
        const response = await client.get('/vinted/user-profile', { user_id: '255914028', country: 'DE' }, { attempts: 2 });
        assert.equal(response.success, true);
        assert.equal(calls.length, 2);
        assert.match(calls[0].url, /^https:\/\/scrappa\.test\/api\/vinted\/user-profile\?/);
        assert.match(calls[0].url, /user_id=255914028/);
        assert.match(calls[0].url, /country=DE/);
        assert.equal(calls[0].options.headers['User-Agent'], 'thescrappa-vinted-user-profile-scraper/1.0');
    } finally {
        globalThis.fetch = originalFetch;
        Math.random = originalRandom;
    }
});
