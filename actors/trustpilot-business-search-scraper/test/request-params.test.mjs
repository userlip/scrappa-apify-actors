import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildPageParams,
    buildTrustpilotBusinessSearchPlan,
    describeTrustpilotBusinessSearchRequest,
} = await import(requestParamsModule);

test('builds a default Trustpilot company search plan', () => {
    const plan = buildTrustpilotBusinessSearchPlan({ query: ' amazon ' });

    assert.deepEqual(plan, {
        searchType: 'company_search',
        endpoint: '/trustpilot/company-search',
        baseParams: {
            query: 'amazon',
            per_page: 20,
            locale: 'en-US',
        },
        startPage: 1,
        maxPages: 1,
    });
    assert.deepEqual(buildPageParams(plan, 1), {
        query: 'amazon',
        per_page: 20,
        locale: 'en-US',
        page: 1,
    });
});

test('normalizes company search filters and encoded query', () => {
    const plan = buildTrustpilotBusinessSearchPlan({
        search_type: 'company_search',
        query: 'm%C3%BCller & partner',
        country: 'de',
        page: '2',
        max_pages: '3',
        per_page: '10',
        min_rating: '4.2',
        min_review_count: '100',
        locale: 'de-DE',
    });

    assert.deepEqual(plan.baseParams, {
        query: 'müller & partner',
        per_page: 10,
        locale: 'de-DE',
        country: 'DE',
        min_rating: 4.2,
        min_review_count: 100,
    });
    assert.equal(describeTrustpilotBusinessSearchRequest(plan), 'company query "müller & partner" (pages 2-4)');
});

test('builds a category businesses plan', () => {
    const plan = buildTrustpilotBusinessSearchPlan({
        search_type: 'category',
        category: ' electronics_technology ',
        country: 'us',
        sort: 'reviews_count',
        claimed: true,
        limit: '30',
        trustscore: '4.5',
        max_pages: 2,
    });

    assert.deepEqual(plan, {
        searchType: 'category',
        endpoint: '/trustpilot/businesses',
        baseParams: {
            category: 'electronics_technology',
            limit: 30,
            country: 'US',
            sort: 'reviews_count',
            claimed: 1,
            trustscore: 4.5,
        },
        startPage: 1,
        maxPages: 2,
    });
    assert.equal(describeTrustpilotBusinessSearchRequest(plan), 'category "electronics_technology" (pages 1-2)');
});

test('infers category search when category is supplied without search_type', () => {
    const plan = buildTrustpilotBusinessSearchPlan({ category: 'restaurants_bars' });

    assert.equal(plan.searchType, 'category');
    assert.equal(plan.endpoint, '/trustpilot/businesses');
});

test('ignores blank category when inferring search type', () => {
    const plan = buildTrustpilotBusinessSearchPlan({
        query: 'amazon',
        category: '   ',
    });

    assert.equal(plan.searchType, 'company_search');
    assert.equal(plan.endpoint, '/trustpilot/company-search');
    assert.equal(plan.baseParams.query, 'amazon');
});

test('validates required fields and bounds', () => {
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ query: 'a' }),
        /query must be at least 2 characters/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ search_type: 'category', category: 'a' }),
        /category must be at least 2 characters/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ query: 'amazon', country: 'USA' }),
        /country must be an ISO-2 country code/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ query: 'amazon', page: 0 }),
        /page must be between 1 and 999/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ query: 'amazon', max_pages: 11 }),
        /max_pages must be between 1 and 10/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ query: 'amazon', min_rating: 6 }),
        /min_rating must be between 0 and 5/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ query: 'amazon', min_review_count: 1.5 }),
        /min_review_count must be an integer/,
    );
    assert.throws(
        () => buildTrustpilotBusinessSearchPlan({ search_type: 'category', category: 'electronics', sort: 'rating' }),
        /sort must be one of/,
    );
});

test('input schema matches the Trustpilot business search contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.properties.search_type.default, 'company_search');
    assert.deepEqual(schema.properties.search_type.enum, ['company_search', 'category']);
    assert.equal(schema.properties.page.minimum, 1);
    assert.equal(schema.properties.page.maximum, 999);
    assert.equal(schema.properties.max_pages.maximum, 10);
    assert.equal(schema.properties.per_page.maximum, 50);
    assert.equal(schema.properties.limit.maximum, 50);
});
