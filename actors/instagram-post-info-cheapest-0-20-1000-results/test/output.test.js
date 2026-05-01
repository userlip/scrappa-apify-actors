import assert from 'node:assert/strict';
import test from 'node:test';
import { flattenInstagramPostResponse, getInstagramPostPayload } from '../src/output.js';

test('extracts post payload from nested data', () => {
    const response = {
        success: true,
        data: {
            shortcode: 'DXHKcyvEWfr',
            caption: 'Example caption',
            likes_count: 123,
        },
    };

    assert.deepEqual(getInstagramPostPayload(response), response.data);
});

test('extracts post payload from common nested post wrappers', () => {
    const post = { shortcode: 'DXHKcyvEWfr' };

    assert.equal(getInstagramPostPayload({ success: true, data: { post } }), post);
    assert.equal(getInstagramPostPayload({ success: true, post }), post);
});

test('flattens post fields onto the dataset item', () => {
    assert.deepEqual(flattenInstagramPostResponse({
        success: true,
        data: {
            id: '123',
            shortcode: 'DXHKcyvEWfr',
            url: 'https://www.instagram.com/p/DXHKcyvEWfr/',
            caption: 'Example caption',
            likes_count: 123,
            comments_count: 4,
        },
    }), {
        success: true,
        id: '123',
        shortcode: 'DXHKcyvEWfr',
        url: 'https://www.instagram.com/p/DXHKcyvEWfr/',
        caption: 'Example caption',
        likes_count: 123,
        comments_count: 4,
    });
});

test('keeps error metadata when flattening an error payload', () => {
    assert.deepEqual(flattenInstagramPostResponse({
        success: false,
        error: 'Not found',
        data: {
            shortcode: 'missing',
        },
    }), {
        success: false,
        error: 'Not found',
        shortcode: 'missing',
    });
});

test('returns non-object responses unchanged', () => {
    assert.equal(flattenInstagramPostResponse(null), null);
    assert.equal(flattenInstagramPostResponse('not json'), 'not json');
});
