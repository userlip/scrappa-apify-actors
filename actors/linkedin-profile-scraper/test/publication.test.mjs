import assert from 'node:assert/strict';
import test from 'node:test';

import { publishLinkedInProfileResults } from '../dist/publication.js';

function captureWrites() {
    const dataset = [];
    const records = new Map();

    return {
        dataset,
        records,
        storage: {
            async pushData(result) {
                dataset.push(result);
            },
            async setValue(key, value) {
                records.set(key, value);
            },
        },
    };
}

test('publishes only successful batch results to the billable default dataset', async () => {
    const writes = captureWrites();
    const success = { success: true, input_url: 'valid', name: 'Valid Profile' };
    const unavailable = { success: false, input_url: 'unavailable', status_code: 503 };
    const invalid = { success: false, input_url: 'invalid', error_type: 'error' };
    const notFound = { success: false, input_url: 'missing', status_code: 404 };

    const summary = await publishLinkedInProfileResults(
        [success, unavailable, invalid, notFound],
        writes.storage,
    );

    assert.deepEqual(writes.dataset, [success]);
    assert.deepEqual(writes.records.get('OUTPUT'), { requested: 4, succeeded: 1, failed: 3 });
    assert.deepEqual(writes.records.get('FAILURES'), [unavailable, invalid, notFound]);
    assert.deepEqual(summary, { requested: 4, succeeded: 1, failed: 3 });
});

test('stores a single 404 in OUTPUT without writing a billable dataset item', async () => {
    const writes = captureWrites();
    const notFound = {
        success: false,
        input_url: 'missing',
        normalized_url: 'https://www.linkedin.com/in/missing',
        status_code: 404,
        message: 'Profile not found or not publicly accessible',
    };

    await publishLinkedInProfileResults([notFound], writes.storage);

    assert.deepEqual(writes.dataset, []);
    assert.deepEqual(writes.records.get('OUTPUT'), {
        success: false,
        status_code: 404,
        message: 'Profile not found or not publicly accessible',
    });
    assert.deepEqual(writes.records.get('FAILURES'), [notFound]);
});
