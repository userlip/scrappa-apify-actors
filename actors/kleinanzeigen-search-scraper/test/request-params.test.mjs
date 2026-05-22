import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildKleinanzeigenSearchPlan,
    describeKleinanzeigenSearchRequest,
} = await import(requestParamsModule);

test('builds a single Kleinanzeigen search plan', () => {
    const plan = buildKleinanzeigenSearchPlan({
        query: ' iphone ',
        location: ' Berlin ',
        page: '2',
        category: ' elektronik ',
        price_min: '50',
        price_max: '500',
    });

    assert.deepEqual(plan, {
        searches: [
            {
                index: 0,
                params: {
                    query: 'iphone',
                    page: 2,
                    location: 'Berlin',
                    category: 'elektronik',
                    price_min: 50,
                    price_max: 500,
                },
            },
        ],
    });
    assert.equal(describeKleinanzeigenSearchRequest(plan), '"iphone" in Berlin (page 2, category elektronik)');
});

test('normalizes encoded query and defaults page', () => {
    const plan = buildKleinanzeigenSearchPlan({
        query: 'e-bike%20fully',
        location: '',
        category: '',
    });

    assert.deepEqual(plan.searches[0]?.params, {
        query: 'e-bike fully',
        page: 1,
    });
});

test('supports batch searches in one run', () => {
    const plan = buildKleinanzeigenSearchPlan({
        query: 'ignored',
        searches: [
            {
                query: 'iphone',
                location: 'Berlin',
                page: 1,
            },
            {
                query: 'fahrrad',
                location: 'Hamburg',
                price_max: '500',
            },
        ],
    });

    assert.deepEqual(plan, {
        searches: [
            {
                index: 0,
                params: {
                    query: 'iphone',
                    page: 1,
                    location: 'Berlin',
                },
            },
            {
                index: 1,
                params: {
                    query: 'fahrrad',
                    page: 1,
                    location: 'Hamburg',
                    price_max: 500,
                },
            },
        ],
    });
    assert.equal(describeKleinanzeigenSearchRequest(plan), '2 Kleinanzeigen searches');
});

test('keeps valid percent characters that are not URL encoding', () => {
    const plan = buildKleinanzeigenSearchPlan({
        query: '100% baumwolle',
    });

    assert.equal(plan.searches[0]?.params.query, '100% baumwolle');
});

test('validates required query, price range, pagination, filters, and batch shape', () => {
    assert.throws(
        () => buildKleinanzeigenSearchPlan({}),
        /query is required/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ query: '' }),
        /query is required/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ query: 'iphone', page: 101 }),
        /page must be between 1 and 100/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ query: 'iphone', location: 'x'.repeat(101) }),
        /location must be 100 characters or fewer/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ query: 'iphone', price_min: '10.5' }),
        /price_min must be an integer/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ query: 'iphone', price_min: 500, price_max: 50 }),
        /price_max cannot be less than price_min/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ searches: [] }),
        /searches must contain at least one search/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ searches: ['iphone'] }),
        /searches\[0\] must be an object/,
    );
    assert.throws(
        () => buildKleinanzeigenSearchPlan({ searches: Array.from({ length: 26 }, () => ({ query: 'iphone' })) }),
        /searches cannot contain more than 25 search objects/,
    );
});

test('input schema matches the Kleinanzeigen search contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.query.maxLength, 500);
    assert.equal(schema.properties.page.minimum, 1);
    assert.equal(schema.properties.page.maximum, 100);
    assert.equal(schema.properties.location.maxLength, 100);
    assert.equal(schema.properties.category.maxLength, 100);
    assert.equal(schema.properties.price_min.type, 'integer');
    assert.equal(schema.properties.price_max.type, 'integer');
    assert.equal(schema.properties.searches.maxItems, 25);
    assert.deepEqual(schema.properties.searches.items.required, ['query']);
});
