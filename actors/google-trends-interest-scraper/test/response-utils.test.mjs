import assert from 'node:assert/strict';
import test from 'node:test';

import { buildTimelineDatasetItems, getTimelinePoints } from '../dist/response-utils.js';

test('uses timeline_data as the primary timeline source', () => {
    const response = {
        timeline_data: [
            { timestamp: 1704067200, date: '2024-01-01', value: 42 },
        ],
        interest_over_time: {
            data_points: [
                { timestamp: 1, date: 'fallback', value: 1 },
            ],
        },
    };

    assert.deepEqual(getTimelinePoints(response), [
        { timestamp: 1704067200, date: '2024-01-01', value: 42 },
    ]);
});

test('falls back to interest_over_time data_points', () => {
    const response = {
        interest_over_time: {
            data_points: [
                { timestamp: 1704067200, date: '2024-01-01', value: 42 },
            ],
        },
    };

    assert.deepEqual(getTimelinePoints(response), [
        { timestamp: 1704067200, date: '2024-01-01', value: 42 },
    ]);
});

test('builds Apify dataset items with request and summary context', () => {
    const items = buildTimelineDatasetItems(
        {
            search_parameters: { keyword: 'tesla', geo: 'US' },
            timeline_data: [
                { timestamp: 1704067200, date: '2024-01-01', value: 42 },
                { timestamp: 1704672000, date: '2024-01-08', value: 58 },
            ],
            interest_over_time: {
                average: 50,
                max_value: 100,
                min_value: 12,
            },
            response_time_ms: 587,
        },
        {
            q: 'tesla',
            geo: 'US',
            time_range: '1y',
            hl: 'en',
            search_type: 'web',
        },
    );

    assert.equal(items.length, 2);
    assert.deepEqual(items[0], {
        timestamp: 1704067200,
        date: '2024-01-01',
        value: 42,
        position: 1,
        average: 50,
        max_value: 100,
        min_value: 12,
        request_q: 'tesla',
        request_geo: 'US',
        request_time_range: '1y',
        request_hl: 'en',
        request_search_type: 'web',
        response_time_ms: 587,
        search_parameters: { keyword: 'tesla', geo: 'US' },
    });
});
