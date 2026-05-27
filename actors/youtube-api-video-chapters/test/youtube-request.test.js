import test from 'node:test';
import assert from 'node:assert/strict';

import {
    buildScrappaRequest,
    buildVideoChaptersUrl,
    getScrappaApiKey,
    getVideoIds,
} from '../src/youtube-request.js';

test('builds maintained video chapters URL', () => {
    assert.equal(
        buildVideoChaptersUrl('video id'),
        'https://scrappa.co/api/youtube/chapters?video_id=video+id',
    );
});

test('parses batch video IDs', () => {
    assert.deepEqual(getVideoIds({ ids: 'vid1, vid2', id: 'vid2,vid3' }), ['vid1', 'vid2', 'vid3']);
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/chapters?video_id=vid1', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
