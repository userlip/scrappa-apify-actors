import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildRedfinValuationRequests,
    describeRedfinValuationRequest,
    extractRedfinIdsFromUrl,
} = await import(requestParamsModule);

test('builds a single Redfin valuation request', () => {
    assert.deepEqual(
        buildRedfinValuationRequests({ property_id: '194191988', listing_id: '207388793' }),
        [{
            index: 0,
            property_id: 194191988,
            listing_id: 207388793,
            url: null,
            params: {
                property_id: 194191988,
                listing_id: 207388793,
            },
        }],
    );
});

test('builds batch Redfin valuation requests from property IDs', () => {
    assert.deepEqual(
        buildRedfinValuationRequests({ property_ids: ['194191988', 123456789] }),
        [
            {
                index: 0,
                property_id: 194191988,
                listing_id: null,
                url: null,
                params: { property_id: 194191988 },
            },
            {
                index: 1,
                property_id: 123456789,
                listing_id: null,
                url: null,
                params: { property_id: 123456789 },
            },
        ],
    );
});

test('builds batch Redfin valuation requests from property objects', () => {
    assert.deepEqual(
        buildRedfinValuationRequests({
            properties: [
                { property_id: 194191988 },
                { property_id: '123456789', listing_id: '207388793' },
                { url: 'https://www.redfin.com/WA/Seattle/example/home/987654321?listing_id=222333444' },
            ],
        }),
        [
            {
                index: 0,
                property_id: 194191988,
                listing_id: null,
                url: null,
                params: { property_id: 194191988 },
            },
            {
                index: 1,
                property_id: 123456789,
                listing_id: 207388793,
                url: null,
                params: { property_id: 123456789, listing_id: 207388793 },
            },
            {
                index: 2,
                property_id: 987654321,
                listing_id: 222333444,
                url: 'https://www.redfin.com/WA/Seattle/example/home/987654321?listing_id=222333444',
                params: { property_id: 987654321, listing_id: 222333444 },
            },
        ],
    );
});

test('extracts Redfin IDs from URLs where possible', () => {
    assert.deepEqual(
        extractRedfinIdsFromUrl('https://www.redfin.com/WA/Seattle/example/home/194191988?listing_id=207388793'),
        {
            property_id: 194191988,
            listing_id: 207388793,
            url: 'https://www.redfin.com/WA/Seattle/example/home/194191988?listing_id=207388793',
        },
    );
});

test('input schema exposes batch fields', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.property_ids.type, 'array');
    assert.equal(schema.properties.property_ids.maxItems, 50);
    assert.equal(schema.properties.properties.type, 'array');
    assert.equal(schema.properties.properties.maxItems, 50);
});

test('validates Redfin valuation inputs', () => {
    assert.throws(
        () => buildRedfinValuationRequests({}),
        /property_id is required/,
    );
    assert.throws(
        () => buildRedfinValuationRequests({ property_id: 'abc' }),
        /property_id must be an integer/,
    );
    assert.throws(
        () => buildRedfinValuationRequests({ property_id: 0 }),
        /property_id must be greater than 0/,
    );
    assert.throws(
        () => buildRedfinValuationRequests({ property_ids: [] }),
        /property_ids must include at least one property/,
    );
    assert.throws(
        () => buildRedfinValuationRequests({ property_ids: Array.from({ length: 51 }, () => 194191988) }),
        /property_ids cannot include more than 50 properties/,
    );
    assert.throws(
        () => buildRedfinValuationRequests({ properties: ['194191988'] }),
        /properties\[0\] must be an object/,
    );
});

test('describes Redfin valuation request', () => {
    assert.equal(
        describeRedfinValuationRequest({
            index: 0,
            property_id: 194191988,
            listing_id: null,
            url: null,
            params: { property_id: 194191988 },
        }),
        'property 194191988',
    );
    assert.equal(
        describeRedfinValuationRequest({
            index: 0,
            property_id: 194191988,
            listing_id: 207388793,
            url: null,
            params: { property_id: 194191988, listing_id: 207388793 },
        }),
        'property 194191988 with listing 207388793',
    );
});
