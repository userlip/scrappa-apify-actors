import test from 'node:test';
import assert from 'node:assert/strict';

import {
    buildChannelAboutDetailsUrl,
    buildScrappaRequest,
    getChannelIds,
    getScrappaApiKey,
    toChannelAboutDetails,
} from '../src/youtube-request.js';

test('builds maintained channel about URL', () => {
    assert.equal(
        buildChannelAboutDetailsUrl('UC example'),
        'https://scrappa.co/api/youtube/channel?channel_id=UC+example',
    );
});

test('parses batch channel IDs', () => {
    assert.deepEqual(getChannelIds({ ids: 'UC1, UC2', id: 'UC2,UC3' }), ['UC1', 'UC2', 'UC3']);
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/channel?channel_id=UC1', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('maps maintained channel response to legacy about-details shape', () => {
    assert.deepEqual(toChannelAboutDetails({
        id: 'UC123',
        name: 'Example Channel',
        description: 'About text',
        subscriberCount: '2.64M subscribers 6.3K videos',
        viewCount: null,
        country: 'United States',
        joinedDate: 'Joined Aug 23, 2007',
        links: [{ title: 'Website', url: 'example.com' }],
    }), {
        channelId: 'UC123',
        stats: {
            joinDate: 'Joined Aug 23, 2007',
            viewCount: null,
            country: 'United States',
        },
        links: [{ title: 'Website', url: 'example.com' }],
        details: {
            description: 'About text',
            email: null,
            name: 'Example Channel',
            subscriberCount: '2.64M subscribers',
            videoCount: '6.3K videos',
            channelUrl: 'https://www.youtube.com/channel/UC123',
        },
    });
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
