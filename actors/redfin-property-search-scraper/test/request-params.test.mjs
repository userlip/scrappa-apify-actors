import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const { buildRedfinPropertySearchRequests, describeRedfinPropertySearchRequest } = await import(requestParamsModule);

test('builds a single Redfin property search request', () => {
    assert.deepEqual(
        buildRedfinPropertySearchRequests({
            region_id: '16163',
            region_type: '6',
            market: ' seattle ',
            min_price: '100000',
            max_price: 800000,
            num_beds: '2',
            num_baths: '1.5',
            property_types: '1,2,3',
            status: '9',
            sold_within_days: '30',
            num_homes: '50',
            page: '2',
        }),
        [{
            index: 0,
            params: {
                region_id: 16163,
                region_type: 6,
                market: 'seattle',
                min_price: 100000,
                max_price: 800000,
                num_beds: 2,
                num_baths: 1.5,
                property_types: '1,2,3',
                status: 9,
                sold_within_days: 30,
                num_homes: 50,
                page: 2,
            },
        }],
    );
});

test('builds batch Redfin property search requests', () => {
    assert.deepEqual(
        buildRedfinPropertySearchRequests({
            searches: [
                { region_id: 16163, region_type: 6, market: 'seattle', num_homes: 25 },
                { region_id: 11203, region_type: 6, market: 'socal', min_price: 500000 },
            ],
        }),
        [
            {
                index: 0,
                params: {
                    region_id: 16163,
                    region_type: 6,
                    market: 'seattle',
                    num_homes: 25,
                },
            },
            {
                index: 1,
                params: {
                    region_id: 11203,
                    region_type: 6,
                    market: 'socal',
                    min_price: 500000,
                },
            },
        ],
    );
});

test('input schema exposes batch searches', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.searches.type, 'array');
    assert.equal(schema.properties.searches.maxItems, 25);
    assert.deepEqual(schema.properties.searches.items.required, ['region_id', 'region_type', 'market']);
});

test('input schema uses Apify-compatible select fields', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.properties.region_type.type, 'string');
    assert.deepEqual(schema.properties.region_type.enum, ['1', '2', '4', '5', '6']);
    assert.equal(schema.properties.region_type.default, '6');
    assert.equal(schema.properties.status.type, 'string');
    assert.deepEqual(schema.properties.status.enum, ['1', '9', '130', '131']);
    assert.equal(schema.properties.status.default, '9');
});

test('input schema describes batch search fields for Apify validation', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    for (const [field, property] of Object.entries(schema.properties.searches.items.properties)) {
        assert.equal(typeof property.title, 'string', `${field} title`);
        assert.notEqual(property.title.trim(), '', `${field} title`);
        assert.equal(typeof property.description, 'string', `${field} description`);
        assert.notEqual(property.description.trim(), '', `${field} description`);
    }
});

test('requires Redfin region fields', () => {
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_type: 6, market: 'seattle' }),
        /region_id is required/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, market: 'seattle' }),
        /region_type is required/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, region_type: 6, market: '' }),
        /market is required/,
    );
});

test('validates filter fields', () => {
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, region_type: 99, market: 'seattle' }),
        /region_type must be one of: 1, 2, 4, 5, 6/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, region_type: 6, market: 'Seattle WA' }),
        /market must contain only lowercase letters and numbers/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({
            region_id: 16163,
            region_type: 6,
            market: 'seattle',
            min_price: 500000,
            max_price: 400000,
        }),
        /max_price must be greater than or equal to min_price/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, region_type: 6, market: 'seattle', property_types: '1,9' }),
        /property_types must be comma-separated numbers 1-8/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, region_type: 6, market: 'seattle', num_homes: 451 }),
        /num_homes must be between 1 and 450/,
    );
});

test('validates batch constraints', () => {
    assert.throws(
        () => buildRedfinPropertySearchRequests({ searches: [] }),
        /searches must include at least one search/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ searches: ['seattle'] }),
        /searches\[0\] must be an object/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({ region_id: 16163, region_type: 6, market: 'seattle', searches: 'seattle' }),
        /searches must be an array of search objects/,
    );
    assert.throws(
        () => buildRedfinPropertySearchRequests({
            searches: Array.from({ length: 26 }, () => ({ region_id: 16163, region_type: 6, market: 'seattle' })),
        }),
        /searches cannot include more than 25 searches/,
    );
});

test('describes Redfin property search request', () => {
    assert.equal(
        describeRedfinPropertySearchRequest({
            region_id: 16163,
            region_type: 6,
            market: 'seattle',
            num_homes: 50,
            page: 2,
        }),
        'region 16163 (seattle, type 6) (num_homes=50, page=2)',
    );
});
