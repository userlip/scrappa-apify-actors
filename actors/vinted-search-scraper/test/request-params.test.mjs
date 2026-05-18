import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildPageParams,
    buildVintedSearchPlan,
    describeVintedSearchRequest,
} = await import(requestParamsModule);

test('builds a default Vinted search plan', () => {
    const plan = buildVintedSearchPlan({ query: ' nike shoes ' });

    assert.deepEqual(plan, {
        baseParams: {
            country: 'FR',
            per_page: 24,
            query: 'nike shoes',
        },
        startPage: 1,
        perPage: 24,
        maxPages: 1,
    });
    assert.deepEqual(buildPageParams(plan, 1), {
        country: 'FR',
        per_page: 24,
        query: 'nike shoes',
        page: 1,
    });
});

test('normalizes country, numeric strings, filters, and encoded query', () => {
    const plan = buildVintedSearchPlan({
        query: 'zara%20dress',
        country: 'de',
        page: '2',
        per_page: '50',
        max_pages: '3',
        order: 'newest_first',
        brand_ids: ' 53,  88 ',
        price_from: '10.5',
        price_to: '80',
    });

    assert.deepEqual(plan, {
        baseParams: {
            country: 'DE',
            per_page: 50,
            query: 'zara dress',
            order: 'newest_first',
            brand_ids: '53,88',
            price_from: 10.5,
            price_to: 80,
        },
        startPage: 2,
        perPage: 50,
        maxPages: 3,
    });
    assert.equal(describeVintedSearchRequest(plan), '"zara dress" in DE (pages 2-4, 50/page)');
});

test('supports filter-only searches without query', () => {
    const plan = buildVintedSearchPlan({
        country: 'FR',
        catalog_ids: '5',
        price_to: 25,
    });

    assert.equal(describeVintedSearchRequest(plan), 'all listings in FR (page 1, 24/page)');
    assert.deepEqual(plan.baseParams, {
        country: 'FR',
        per_page: 24,
        catalog_ids: '5',
        price_to: 25,
    });
});

test('keeps valid percent characters that are not URL encoding', () => {
    const plan = buildVintedSearchPlan({
        query: '100% cotton',
        country: 'FR',
    });

    assert.equal(plan.baseParams.query, '100% cotton');
});

test('validates country, sorting, filters, price, and pagination bounds', () => {
    assert.throws(
        () => buildVintedSearchPlan({ country: 'GB' }),
        /country must be one of/,
    );
    assert.throws(
        () => buildVintedSearchPlan({ order: 'oldest_first' }),
        /order must be one of/,
    );
    assert.throws(
        () => buildVintedSearchPlan({ brand_ids: '12,nike' }),
        /brand_ids must be a comma-separated list of numeric IDs/,
    );
    assert.throws(
        () => buildVintedSearchPlan({ price_from: 90, price_to: 20 }),
        /price_from cannot be greater than price_to/,
    );
    assert.throws(
        () => buildVintedSearchPlan({ page: 0 }),
        /page must be between 1 and 999/,
    );
    assert.throws(
        () => buildVintedSearchPlan({ max_pages: 21 }),
        /max_pages must be between 1 and 20/,
    );
    assert.throws(
        () => buildVintedSearchPlan({ page: 990, max_pages: 20 }),
        /page plus max_pages cannot exceed page 999/,
    );
});

test('input schema matches the Vinted search contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.required, undefined);
    assert.deepEqual(schema.properties.country.enum, ['FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT', 'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US']);
    assert.deepEqual(schema.properties.order.enum, ['relevance', 'newest_first', 'price_low_to_high', 'price_high_to_low']);
    assert.equal(schema.properties.page.minimum, 1);
    assert.equal(schema.properties.page.maximum, 999);
    assert.equal(schema.properties.per_page.maximum, 100);
    assert.equal(schema.properties.max_pages.maximum, 20);
});
