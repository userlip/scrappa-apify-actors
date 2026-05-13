import assert from 'node:assert/strict';
import test from 'node:test';

import { extractPagination, extractVideos } from '../dist/response-utils.js';

test('extractVideos returns top-level array data as-is', () => {
    const videos = [{ aweme_id: '1' }];
    assert.deepEqual(extractVideos(videos), videos);
});

test('extractVideos reads nested videos arrays', () => {
    const videos = [{ aweme_id: '2' }];
    assert.deepEqual(extractVideos({ videos }), videos);
});

test('extractVideos falls back through known TikTok list keys', () => {
    const posts = [{ aweme_id: '3' }];
    const awemeList = [{ aweme_id: '4' }];
    const itemList = [{ aweme_id: '5' }];

    assert.deepEqual(extractVideos({ posts }), posts);
    assert.deepEqual(extractVideos({ aweme_list: awemeList }), awemeList);
    assert.deepEqual(extractVideos({ item_list: itemList }), itemList);
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

test('extractPagination handles empty object response data', () => {
    assert.deepEqual(extractPagination({}), { hasNextPage: false, nextCursor: null });
});
