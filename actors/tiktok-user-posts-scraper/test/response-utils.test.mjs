import assert from 'node:assert/strict';
import test from 'node:test';

import { extractPagination, extractPosts } from '../dist/response-utils.js';

test('extractPosts returns top-level array data as-is', () => {
    const posts = [{ aweme_id: '1' }];
    assert.deepEqual(extractPosts(posts), posts);
});

test('extractPosts reads nested posts arrays', () => {
    const posts = [{ aweme_id: '2' }];
    assert.deepEqual(extractPosts({ posts }), posts);
});

test('extractPosts falls back to aweme_list', () => {
    const posts = [{ aweme_id: '3' }];
    assert.deepEqual(extractPosts({ aweme_list: posts }), posts);
});

test('extractPagination prefers explicit cursor when present', () => {
    assert.deepEqual(
        extractPagination({ hasMore: true, cursor: '100', max_cursor: '200', min_cursor: '300' }),
        { hasNextPage: true, nextCursor: '100' },
    );
});

test('extractPagination falls back through max_cursor and min_cursor', () => {
    assert.deepEqual(
        extractPagination({ has_more: true, max_cursor: '200' }),
        { hasNextPage: true, nextCursor: '200' },
    );
    assert.deepEqual(
        extractPagination({ has_more: false, min_cursor: '300' }),
        { hasNextPage: false, nextCursor: '300' },
    );
});

test('extractPagination handles array or null response data', () => {
    assert.deepEqual(extractPagination([{ aweme_id: '1' }]), { hasNextPage: false, nextCursor: null });
    assert.deepEqual(extractPagination(null), { hasNextPage: false, nextCursor: null });
});
