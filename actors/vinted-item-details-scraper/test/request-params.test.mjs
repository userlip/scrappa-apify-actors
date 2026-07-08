import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildVintedItemDetailsRequests,
    describeVintedItemDetailsRequest,
} = await import(requestParamsModule);

test('builds a single Vinted item details request', () => {
    const requests = buildVintedItemDetailsRequests({
        item_id: ' 1234567890 ',
        country: 'de',
    });

    assert.deepEqual(requests, [{
        itemId: '1234567890',
        params: {
            item_id: '1234567890',
            country: 'DE',
        },
        index: 0,
    }]);
    assert.equal(describeVintedItemDetailsRequest(requests[0]), '1234567890 in DE');
});

test('builds deduped batch requests and accepts integer IDs', () => {
    const requests = buildVintedItemDetailsRequests({
        item_id: 123,
        item_ids: ['123', '456', 789],
    });

    assert.deepEqual(requests.map((request) => request.itemId), ['123', '456', '789']);
    assert.deepEqual(requests.map((request) => request.index), [0, 1, 2]);
    assert.deepEqual(requests.map((request) => request.params.country), ['FR', 'FR', 'FR']);
});

test('validates required IDs, numeric IDs, country, and batch size', () => {
    assert.throws(
        () => buildVintedItemDetailsRequests({}),
        /Provide at least one Vinted item ID/,
    );
    assert.throws(
        () => buildVintedItemDetailsRequests({ item_id: 'abc' }),
        /item_id must be a numeric Vinted item ID/,
    );
    assert.throws(
        () => buildVintedItemDetailsRequests({ item_ids: '123' }),
        /item_ids must be an array/,
    );
    assert.throws(
        () => buildVintedItemDetailsRequests({ item_id: '123', country: 'GB' }),
        /country must be one of/,
    );
    assert.throws(
        () => buildVintedItemDetailsRequests({ item_ids: Array.from({ length: 51 }, (_, index) => String(index + 1)) }),
        /item_ids cannot include more than 50 IDs per run/,
    );
});

test('input schema matches the Vinted item details contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.item_id.pattern, '^\\d+$');
    assert.equal(schema.properties.item_ids.maxItems, 50);
    assert.equal(schema.properties.item_ids.items.pattern, '^\\d+$');
    assert.deepEqual(schema.properties.country.enum, ['FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT', 'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US']);
});
