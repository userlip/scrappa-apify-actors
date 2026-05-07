import assert from 'node:assert/strict';
import test from 'node:test';

import { extractFollowing, extractPagination, extractProfileUserId } from '../dist/response-utils.js';

test('extractFollowing returns top-level array data as-is', () => {
    const following = [{ user_id: '1' }];
    assert.deepEqual(extractFollowing(following), following);
});

test('extractFollowing reads nested following arrays', () => {
    const following = [{ user_id: '2' }];
    assert.deepEqual(extractFollowing({ following }), following);
    assert.deepEqual(extractFollowing({ followings: following }), following);
});

test('extractFollowing falls back to alternate user arrays', () => {
    const users = [{ user_id: '3' }];
    const userList = [{ user_id: '4' }];
    assert.deepEqual(extractFollowing({ users }), users);
    assert.deepEqual(extractFollowing({ user_list: userList }), userList);
});

test('extractProfileUserId reads profile object or first array item', () => {
    assert.equal(extractProfileUserId({ user_id: ' 107955 ' }), '107955');
    assert.equal(extractProfileUserId({ user_id: 107959 }), '107959');
    assert.equal(extractProfileUserId([{ user_id: '107956' }]), '107956');
    assert.equal(extractProfileUserId({ id: '107957' }), '107957');
    assert.equal(extractProfileUserId({ id: 107960 }), '107960');
    assert.equal(extractProfileUserId({ user: { user_id: 107961 } }), '107961');
    assert.equal(extractProfileUserId({ user: { id: '107958' } }), '107958');
    assert.equal(extractProfileUserId({ user: { id: 107962 } }), '107962');
});

test('extractProfileUserId returns null for missing profile IDs', () => {
    assert.equal(extractProfileUserId({ unique_id: '@tiktok' }), null);
    assert.equal(extractProfileUserId([]), null);
    assert.equal(extractProfileUserId(null), null);
});

test('extractPagination reads time pagination marker', () => {
    assert.deepEqual(
        extractPagination({ hasMore: true, time: 1711111111 }),
        { hasNextPage: true, nextTime: 1711111111 },
    );
});

test('extractPagination supports snake-case has_more', () => {
    assert.deepEqual(
        extractPagination({ has_more: false, time: '0' }),
        { hasNextPage: false, nextTime: '0' },
    );
});

test('extractPagination falls back to min_time and max_time pagination markers', () => {
    assert.deepEqual(
        extractPagination({ has_more: true, min_time: '1711111111', max_time: '1711112222' }),
        { hasNextPage: true, nextTime: '1711111111' },
    );
    assert.deepEqual(
        extractPagination({ hasMore: true, max_time: 1711112222 }),
        { hasNextPage: true, nextTime: 1711112222 },
    );
});

test('extractPagination handles array or null response data', () => {
    assert.deepEqual(extractPagination([{ user_id: '1' }]), { hasNextPage: false, nextTime: null });
    assert.deepEqual(extractPagination(null), { hasNextPage: false, nextTime: null });
});
