import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import {
    MAX_USERNAMES,
    REQUEST_CONCURRENCY,
    ScrappaAuthenticationError,
    fetchInstagramUser,
    flattenProfile,
    getUsernames,
    runInstagramUserBatch,
} from '../src/instagram-users.js';

test('getUsernames prefers batch input, keeps legacy input, and removes normalized duplicates', () => {
    assert.deepEqual(
        getUsernames({ usernames: ['@NatGeo', 'instagram', 'natgeo'], username: '@legacy' }),
        ['NatGeo', 'instagram', 'legacy'],
    );
});

test('getUsernames rejects empty, invalid, and oversized batches', () => {
    assert.throws(() => getUsernames({}), /At least one/);
    assert.throws(() => getUsernames({ usernames: ['invalid username'] }), /Invalid Instagram username/);
    assert.throws(
        () => getUsernames({ usernames: Array.from({ length: MAX_USERNAMES + 1 }, (_, index) => `user${index}`) }),
        /maximum of 100/,
    );
});

test('flattenProfile flattens a nested Scrappa user response', () => {
    assert.deepEqual(
        flattenProfile({ success: true, data: { user: { username: 'natgeo', follower_count: 5 } } }),
        {
            success: true,
            data: { user: { username: 'natgeo', follower_count: 5 } },
            username: 'natgeo',
            follower_count: 5,
        },
    );
});

test('fetchInstagramUser sends the normalized request and returns one enriched profile item', async () => {
    const calls = [];
    const item = await fetchInstagramUser('natgeo', 'secret', async (url, options) => {
        calls.push({ url: url.toString(), options });
        return new Response(JSON.stringify({ data: { user: { username: 'natgeo' } } }), { status: 200 });
    });

    assert.equal(calls.length, 1);
    assert.equal(calls[0].url, 'https://scrappa.co/api/instagram/user?username=natgeo');
    assert.equal(calls[0].options.headers['X-API-KEY'], 'secret');
    assert.equal(item.input_username, 'natgeo');
    assert.equal(item.username, 'natgeo');
});

test('fetchInstagramUser treats authentication failures as fatal', async () => {
    await assert.rejects(
        () => fetchInstagramUser('natgeo', 'bad', async () => new Response(
            JSON.stringify({ message: 'Invalid API key' }),
            { status: 401 },
        )),
        ScrappaAuthenticationError,
    );
});

test('runInstagramUserBatch writes one item per username in bounded groups', async () => {
    const usernames = Array.from({ length: REQUEST_CONCURRENCY + 2 }, (_, index) => `user${index}`);
    const writes = [];
    const summary = await runInstagramUserBatch(
        usernames,
        async (username) => {
            if (username === 'user1') throw new Error('not found');
            return { username, success: true };
        },
        async (items) => writes.push(items),
    );

    assert.deepEqual(summary, { requested: usernames.length, succeeded: 6, failed: 1 });
    assert.deepEqual(writes.map((items) => items.length), [REQUEST_CONCURRENCY, 2]);
    assert.equal(writes.flat().length, usernames.length);
    assert.deepEqual(writes.flat()[1], {
        success: false,
        input_username: 'user1',
        username: 'user1',
        error: 'not found',
    });
});

test('actor metadata exposes batch input and minimal wrapper memory', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    const actor = JSON.parse(await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'));

    assert.equal(schema.properties.usernames.maxItems, MAX_USERNAMES);
    assert.equal(schema.properties.usernames.editor, 'stringList');
    assert.equal(schema.properties.username.type, 'string');
    assert.equal(actor.defaultMemoryMbytes, 128);
    assert.equal(actor.resources, undefined);
});
