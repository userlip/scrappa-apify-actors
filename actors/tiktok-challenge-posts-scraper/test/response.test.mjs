import assert from 'node:assert/strict';
import test from 'node:test';
import { getVideoId, parsePage } from '../dist/response.js';

test('parses live challenge posts pagination shape', () => {
    assert.deepEqual(parsePage({ videos: [{ video_id: '1' }], cursor: 10, hasMore: true }), {
        videos: [{ video_id: '1' }], cursor: '10', hasMore: true,
    });
});

test('supports upstream fallback shapes', () => {
    assert.deepEqual(parsePage({ posts: [{ video_id: '1' }] }), {
        videos: [{ video_id: '1' }], cursor: null, hasMore: false,
    });
    assert.deepEqual(parsePage({ aweme_list: [{ aweme_id: '2' }], max_cursor: '20', has_more: false }), {
        videos: [{ aweme_id: '2' }], cursor: '20', hasMore: false,
    });
});

test('prefers stable video identifiers', () => {
    assert.equal(getVideoId({ video_id: '1', aweme_id: '2' }), '1');
    assert.equal(getVideoId({ aweme_id: '2' }), '2');
    assert.equal(getVideoId({}), null);
});
