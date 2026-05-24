import assert from 'node:assert/strict';
import test from 'node:test';

import { buildRelatedDatasetItems } from '../dist/response-utils.js';

test('flattens top and rising related query arrays', () => {
    const items = buildRelatedDatasetItems(
        {
            search_parameters: { keyword: 'coffee', geo: 'US' },
            related_queries: {
                top: [
                    { query: 'coffee near me', value: 100, formatted_value: '100', link: 'https://trends.google.com/trends/explore?q=coffee+near+me' },
                ],
                rising: [
                    { query: 'mushroom coffee', value: 850, formatted_value: '+850%' },
                ],
            },
            response_time_ms: 623,
        },
        {
            q: 'coffee',
            geo: 'US',
            time_range: '90d',
            hl: 'en',
            search_type: 'web',
        },
    );

    assert.equal(items.length, 2);
    assert.deepEqual(items[0], {
        query: 'coffee near me',
        value: 100,
        formatted_value: '100',
        link: 'https://trends.google.com/trends/explore?q=coffee+near+me',
        position: 1,
        result_kind: 'query',
        type: 'top',
        topic: null,
        topic_type: null,
        source_keyword: 'coffee',
        request_geo: 'US',
        request_time_range: '90d',
        request_hl: 'en',
        request_search_type: 'web',
        response_time_ms: 623,
        search_parameters: { keyword: 'coffee', geo: 'US' },
    });
    assert.equal(items[1].type, 'rising');
    assert.equal(items[1].query, 'mushroom coffee');
});

test('flattens flat related query arrays from the current Scrappa response shape', () => {
    const items = buildRelatedDatasetItems(
        {
            related_queries: [
                { query: 'coffee shops', value: 85, formatted_value: '85%' },
            ],
        },
        { q: 'coffee' },
    );

    assert.equal(items.length, 1);
    assert.equal(items[0].type, null);
    assert.equal(items[0].query, 'coffee shops');
    assert.equal(items[0].source_keyword, 'coffee');
});

test('flattens related topics and preserves the topic category separately', () => {
    const items = buildRelatedDatasetItems(
        {
            related_topics: {
                top: [
                    { topic: 'Coffee', type: 'Drink', value: 100, formatted_value: '100%' },
                ],
                rising: [
                    { title: 'Cold brew', type: 'Topic', value: 200, formatted_value: '+200%' },
                ],
            },
        },
        { q: 'coffee', geo: 'US' },
    );

    assert.equal(items.length, 2);
    assert.equal(items[0].result_kind, 'topic');
    assert.equal(items[0].type, 'top');
    assert.equal(items[0].topic, 'Coffee');
    assert.equal(items[0].topic_type, 'Drink');
    assert.equal(items[1].type, 'rising');
    assert.equal(items[1].topic, 'Cold brew');
});

test('returns no items when response has no related arrays', () => {
    assert.deepEqual(buildRelatedDatasetItems({}, { q: 'coffee' }), []);
});
