import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildPinterestDatasetItem,
    getPinterestPins,
    limitPinterestSearchResponse,
    selectPinterestPins,
} = await import(responseUtilsModule);

test('extracts pins from supported response shapes', () => {
    assert.deepEqual(getPinterestPins({ pins: [{ id: 'pin-1' }] }), [{ id: 'pin-1' }]);
    assert.deepEqual(getPinterestPins({ data: { pins: [{ id: 'pin-2' }] } }), [{ id: 'pin-2' }]);
    assert.deepEqual(getPinterestPins({ results: [{ id: 'pin-3' }] }), [{ id: 'pin-3' }]);
    assert.deepEqual(getPinterestPins({ data: { results: [{ id: 'pin-4' }] } }), [{ id: 'pin-4' }]);
    assert.deepEqual(getPinterestPins({}), []);
});

test('selects pins with source metadata for response limiting', () => {
    assert.deepEqual(selectPinterestPins({ pins: [{ id: 'pin-1' }] }), {
        pins: [{ id: 'pin-1' }],
        source: 'pins',
    });
    assert.deepEqual(selectPinterestPins({ data: { results: [{ id: 'pin-2' }] } }), {
        pins: [{ id: 'pin-2' }],
        source: 'data.results',
    });
    assert.deepEqual(selectPinterestPins({}), {
        pins: [],
        source: null,
    });
});

test('preserves empty primary pins instead of inventing fallback data', () => {
    assert.deepEqual(
        getPinterestPins({
            pins: [],
            data: { pins: [{ id: 'wrapped-pin' }] },
        }),
        [],
    );
});

test('builds normalized Pinterest dataset item', () => {
    const item = buildPinterestDatasetItem(
        {
            id: '123',
            title: 'Small apartment storage',
            description: 'Ideas for small spaces',
            images: {
                orig: { url: 'https://i.pinimg.com/originals/example.jpg' },
            },
            link: 'https://example.com/storage',
            domain: 'example.com',
            pinner: { id: 'u1', username: 'homeideas' },
            board: { id: 'b1', name: 'Home Decor' },
            video: { duration: 12 },
            repin_count: 5,
            comment_count: 2,
        },
        {
            query: 'home decor',
            limit: 25,
            bookmark: 'abc',
        },
        {
            query: 'home decor',
            count: 25,
            results_count: 25,
            nextBookmark: 'def',
            pins: [{ id: '123' }],
        },
    );

    assert.equal(item.id, '123');
    assert.equal(item.image_url, 'https://i.pinimg.com/originals/example.jpg');
    assert.equal(item.link, 'https://example.com/storage');
    assert.equal(item.pinner_id, 'u1');
    assert.equal(item.pinner_username, 'homeideas');
    assert.equal(item.board_id, 'b1');
    assert.equal(item.board_name, 'Home Decor');
    assert.equal(item.has_video, true);
    assert.equal(item.request_query, 'home decor');
    assert.equal(item.request_limit, 25);
    assert.equal(item.request_bookmark, 'abc');
    assert.equal(item.count, 25);
    assert.equal(item.results_count, 25);
    assert.equal(item.nextBookmark, 'def');
});

test('uses returned pin length when results_count is absent', () => {
    const item = buildPinterestDatasetItem(
        { id: '123', image_url: 'https://example.com/image.jpg' },
        { query: 'desk setup', limit: 2 },
        { pins: [{ id: '123' }, { id: '456' }] },
    );

    assert.equal(item.image_url, 'https://example.com/image.jpg');
    assert.equal(item.results_count, 2);
});

test('limits Pinterest response payloads to saved pins only', () => {
    assert.deepEqual(limitPinterestSearchResponse(null, 1), {});
    assert.deepEqual(
        limitPinterestSearchResponse({
            pins: [{ id: 1 }, { id: 2 }],
            results: [{ id: 3 }],
        }, 1, 'pins'),
        { pins: [{ id: 1 }] },
    );
    assert.deepEqual(
        limitPinterestSearchResponse({
            data: {
                pins: [{ id: 1 }],
                results: [{ id: 2 }, { id: 3 }],
                keep: true,
            },
        }, 1, 'data.results'),
        {
            data: {
                results: [{ id: 2 }],
                keep: true,
            },
        },
    );
});
