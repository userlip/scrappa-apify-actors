import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveTikTokFollowersUserId } from '../dist/user-id-resolver.js';

test('resolves unique_id lookups to user_id for followers requests', async () => {
    const calls = [];
    const client = {
        async get(endpoint, params) {
            calls.push({ endpoint, params });
            return {
                code: 0,
                data: { user_id: '107955' },
            };
        },
    };

    const params = await resolveTikTokFollowersUserId(client, { unique_id: '@tiktok', count: 10 });

    assert.deepEqual(calls, [
        { endpoint: '/tiktok/user/profile', params: { unique_id: '@tiktok' } },
    ]);
    assert.deepEqual(params, { unique_id: undefined, count: 10, user_id: '107955' });
});

test('extracts nested TikTok user ids from profile responses', async () => {
    const client = {
        async get() {
            return {
                code: 0,
                data: { user: { id: 107955 } },
            };
        },
    };

    const params = await resolveTikTokFollowersUserId(client, { unique_id: '@tiktok' });

    assert.equal(params.user_id, '107955');
    assert.equal(params.unique_id, undefined);
});

test('does not resolve when user_id is already present', async () => {
    const client = {
        async get() {
            throw new Error('profile lookup should not be called');
        },
    };

    const params = await resolveTikTokFollowersUserId(client, { user_id: '107955', count: 10 });

    assert.deepEqual(params, { user_id: '107955', count: 10 });
});

test('passes through empty unique_id params without profile lookup', async () => {
    const client = {
        async get() {
            throw new Error('profile lookup should not be called');
        },
    };

    const params = await resolveTikTokFollowersUserId(client, { unique_id: '', count: 10 });

    assert.deepEqual(params, { unique_id: '', count: 10 });
});

test('throws when profile lookup returns null data', async () => {
    const client = {
        async get() {
            return {
                code: 0,
                data: null,
            };
        },
    };

    await assert.rejects(
        () => resolveTikTokFollowersUserId(client, { unique_id: '@missing' }),
        /Could not resolve TikTok user_id/,
    );
});

test('throws when profile lookup returns empty data array', async () => {
    const client = {
        async get() {
            return {
                code: 0,
                data: [],
            };
        },
    };

    await assert.rejects(
        () => resolveTikTokFollowersUserId(client, { unique_id: '@missing' }),
        /Could not resolve TikTok user_id/,
    );
});

test('throws when profile lookup returns empty user_id', async () => {
    const client = {
        async get() {
            return {
                code: 0,
                data: { user_id: '   ' },
            };
        },
    };

    await assert.rejects(
        () => resolveTikTokFollowersUserId(client, { unique_id: '@missing' }),
        /Could not resolve TikTok user_id/,
    );
});

test('throws when profile lookup returns non-scalar user_id', async () => {
    const client = {
        async get() {
            return {
                code: 0,
                data: { user_id: { value: '107955' } },
            };
        },
    };

    await assert.rejects(
        () => resolveTikTokFollowersUserId(client, { unique_id: '@missing' }),
        /Could not resolve TikTok user_id/,
    );
});

test('accepts profile responses without a code field', async () => {
    const client = {
        async get() {
            return {
                data: { user_id: '107955' },
            };
        },
    };

    const params = await resolveTikTokFollowersUserId(client, { unique_id: '@tiktok' });

    assert.equal(params.user_id, '107955');
    assert.equal(params.unique_id, undefined);
});

test('throws when profile lookup does not return user_id', async () => {
    const client = {
        async get() {
            return {
                code: 0,
                data: {},
            };
        },
    };

    await assert.rejects(
        () => resolveTikTokFollowersUserId(client, { unique_id: '@missing' }),
        /Could not resolve TikTok user_id/,
    );
});

test('throws when profile lookup returns Scrappa API error code', async () => {
    const client = {
        async get() {
            return {
                code: -1,
                msg: 'not found',
            };
        },
    };

    await assert.rejects(
        () => resolveTikTokFollowersUserId(client, { unique_id: '@missing' }),
        /Scrappa TikTok Profile API returned code -1: not found/,
    );
});
