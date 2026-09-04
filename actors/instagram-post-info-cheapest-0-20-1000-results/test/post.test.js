import assert from 'node:assert/strict';
import test from 'node:test';
import { fetchPost, getPostIdentity } from '../src/post.js';

const url = 'https://www.instagram.com/instagram/p/Dc30nJeRKKz/';
const unavailable = { response: { status: 503, data: { success: false, retryable: true } } };

test('returns successful single-post data without a feed request', async () => {
    let calls = 0;
    const response = { data: { success: true, data: { shortcode: 'Dc30nJeRKKz' } } };
    assert.equal(await fetchPost(async () => { calls++; return response; }, { url }, 'test'), response);
    assert.equal(calls, 1);
});

test('fallback returns only the matching post and shares the request deadline', async () => {
    const requests = [];
    const post = { shortcode: 'Dc30nJeRKKz', caption: 'Actual caption' };
    const response = await fetchPost(async (endpoint, options) => {
        requests.push({ endpoint, options });
        if (requests.length === 1) throw unavailable;
        return { data: { success: true, posts: [{ shortcode: 'OTHER' }, post] } };
    }, { url }, 'test');
    assert.deepEqual(response.data, { success: true, found: true, data: post });
    assert.equal(requests[1].endpoint, 'https://scrappa.co/api/instagram/user/posts');
    assert.deepEqual(requests[1].options.params, { username: 'instagram' });
    assert.equal(requests[0].options.signal, requests[1].options.signal);
});

test('does not fabricate a result when the requested post is absent from the feed', async () => {
    let calls = 0;
    await assert.rejects(fetchPost(async () => {
        if (++calls === 1) throw unavailable;
        return { data: { success: true, posts: [{ shortcode: 'OTHER' }] } };
    }, { url }, 'test'), unavailable);
});

test('does not mask authentication errors or use feed fallback for bare shortcodes', async () => {
    for (const [params, error] of [
        [{ url }, { response: { status: 401 } }],
        [{ shortcode: 'Dc30nJeRKKz' }, unavailable],
        [{ url }, { response: { status: 503, data: { retryable: false } } }],
    ]) {
        let calls = 0;
        await assert.rejects(fetchPost(async () => { calls++; throw error; }, params, 'test'), error);
        assert.equal(calls, 1);
    }
});

test('rejects unsuccessful feed responses instead of publishing their posts', async () => {
    let calls = 0;
    await assert.rejects(fetchPost(async () => {
        if (++calls === 1) throw unavailable;
        return { data: { success: false, error: 'Feed unavailable', posts: [{ shortcode: 'Dc30nJeRKKz' }] } };
    }, { url }, 'test'), /Feed unavailable/);
});

test('only extracts account identity from Instagram post URLs', () => {
    assert.deepEqual(getPostIdentity(url), { username: 'instagram', shortcode: 'Dc30nJeRKKz' });
    assert.deepEqual(getPostIdentity('instagram.com/instagram/reel/Dc30nJeRKKz/'), { username: 'instagram', shortcode: 'Dc30nJeRKKz' });
    for (const invalid of [null, 'Dc30nJeRKKz', 'https://example.com/instagram/p/Dc30nJeRKKz/', 'https://www.instagram.com/p/Dc30nJeRKKz/']) {
        assert.equal(getPostIdentity(invalid), null);
    }
});
