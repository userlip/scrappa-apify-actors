import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildRedfinPropertyDetailsRequests,
    describeRedfinPropertyDetailsRequest,
    extractRedfinPropertyIdFromUrl,
} = await import(requestParamsModule);

test('builds a single Redfin property details request from property_id', () => {
    assert.deepEqual(
        buildRedfinPropertyDetailsRequests({ property_id: '60791456' }),
        [{
            index: 0,
            input: '60791456',
            source: 'property_id',
            params: {
                property_id: 60791456,
            },
        }],
    );
});

test('builds batch Redfin property details requests from IDs and URLs', () => {
    assert.deepEqual(
        buildRedfinPropertyDetailsRequests({
            property_ids: [60791456, '194191988'],
            urls: ['https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/12345?utm=test'],
        }),
        [
            {
                index: 0,
                input: 60791456,
                source: 'property_ids',
                params: { property_id: 60791456 },
            },
            {
                index: 1,
                input: '194191988',
                source: 'property_ids',
                params: { property_id: 194191988 },
            },
            {
                index: 2,
                input: 'https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/12345?utm=test',
                source: 'urls',
                params: { property_id: 12345 },
            },
        ],
    );
});

test('deduplicates property IDs across input forms', () => {
    assert.deepEqual(
        buildRedfinPropertyDetailsRequests({
            property_id: 60791456,
            property_ids: ['60791456', 194191988],
            url: 'https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/194191988',
        }).map((request) => request.params.property_id),
        [60791456, 194191988],
    );
});

test('extracts Redfin property ID from URL', () => {
    assert.equal(
        extractRedfinPropertyIdFromUrl('https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456'),
        60791456,
    );
    assert.equal(
        extractRedfinPropertyIdFromUrl('https://redfin.com/home/194191988?utm_source=test'),
        194191988,
    );
});

test('input schema exposes batch property fields', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.property_ids.type, 'array');
    assert.equal(schema.properties.property_ids.maxItems, 100);
    assert.equal(schema.properties.urls.type, 'array');
    assert.equal(schema.properties.urls.maxItems, 100);
});

test('actor metadata keeps low memory and enough timeout for batch runs', async () => {
    const actor = JSON.parse(await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'));
    assert.equal(actor.defaultRunOptions.memoryMbytes, 128);
    assert.equal(actor.defaultRunOptions.timeoutSecs, 3600);
});

test('requires at least one property input', () => {
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({}),
        /Provide at least one property_id/,
    );
});

test('validates property IDs and Redfin URLs', () => {
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({ property_id: 'abc' }),
        /property_id must be an integer/,
    );
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({ property_id: 0 }),
        /property_id must be greater than 0/,
    );
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({ property_ids: '60791456' }),
        /property_ids must be an array/,
    );
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({ urls: 'https://www.redfin.com/home/60791456' }),
        /urls must be an array/,
    );
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({ url: 'https://example.com/home/60791456' }),
        /url must be a Redfin URL/,
    );
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({ url: 'https://www.redfin.com/TN/Memphis/example' }),
        /url must contain a \/home\/\{property_id\} path/,
    );
});

test('validates batch size after dedupe', () => {
    assert.throws(
        () => buildRedfinPropertyDetailsRequests({
            property_ids: Array.from({ length: 101 }, (_, index) => index + 1),
        }),
        /Input cannot include more than 100 unique properties/,
    );
});

test('describes Redfin property details request', () => {
    assert.equal(
        describeRedfinPropertyDetailsRequest({
            index: 0,
            input: 60791456,
            source: 'property_id',
            params: { property_id: 60791456 },
        }),
        'property_id 60791456 from property_id',
    );
});
