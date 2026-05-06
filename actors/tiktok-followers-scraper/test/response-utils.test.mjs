import assert from 'node:assert/strict';
import test from 'node:test';

import { extractFollowers, extractPagination } from '../dist/response-utils.js';

test('extractFollowers returns top-level array data as-is', () => {
    const followers = [{ user_id: '1' }];
    assert.deepEqual(extractFollowers(followers), followers);
});

test('extractFollowers reads nested followers arrays', () => {
    const followers = [{ user_id: '2' }];
    assert.deepEqual(extractFollowers({ followers }), followers);
});

test('extractFollowers falls back to alternate user arrays', () => {
    const users = [{ user_id: '3' }];
    const userList = [{ user_id: '4' }];
    assert.deepEqual(extractFollowers({ users }), users);
    assert.deepEqual(extractFollowers({ user_list: userList }), userList);
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

test('extractPagination handles array or null response data', () => {
    assert.deepEqual(extractPagination([{ user_id: '1' }]), { hasNextPage: false, nextTime: null });
    assert.deepEqual(extractPagination(null), { hasNextPage: false, nextTime: null });
});
