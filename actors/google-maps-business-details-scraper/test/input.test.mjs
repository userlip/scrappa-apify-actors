import assert from 'node:assert/strict';
import test from 'node:test';

import { getBusinessIdRequests } from '../dist/input.js';

test('getBusinessIdRequests supports backward-compatible business_id input', () => {
    assert.deepEqual(
        getBusinessIdRequests({ business_id: ' 0x808fba02425dad8f:0x6c296c66619367e0 ' }),
        [
            {
                input_business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
                business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
            },
        ],
    );
});

test('getBusinessIdRequests combines business_id and business_ids inputs and deduplicates IDs', () => {
    assert.deepEqual(
        getBusinessIdRequests({
            business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
            business_ids: [
                '0x808fba02425dad8f:0x6c296c66619367e0',
                '0x80c2c7c292fef33d:0x9a4f4f5f89f8b8c7',
            ],
        }),
        [
            {
                input_business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
                business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
            },
            {
                input_business_id: '0x80c2c7c292fef33d:0x9a4f4f5f89f8b8c7',
                business_id: '0x80c2c7c292fef33d:0x9a4f4f5f89f8b8c7',
            },
        ],
    );
});

test('getBusinessIdRequests caps unique business IDs per run', () => {
    assert.equal(
        getBusinessIdRequests({
            business_ids: Array.from({ length: 10 }, (_, index) => `business-${index}`),
        }).length,
        10,
    );
    assert.equal(
        getBusinessIdRequests({
            business_id: 'business-0',
            business_ids: Array.from({ length: 10 }, (_, index) => `business-${index}`),
        }).length,
        10,
    );
    assert.throws(
        () => getBusinessIdRequests({
            business_ids: Array.from({ length: 11 }, (_, index) => `business-${index}`),
        }),
        /business_ids must contain 10 unique items or fewer/,
    );
});
