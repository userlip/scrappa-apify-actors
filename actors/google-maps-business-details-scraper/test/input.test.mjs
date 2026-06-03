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
