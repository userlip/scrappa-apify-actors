import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildPageParams,
    buildTrustedShopsSearchPlan,
    describeTrustedShopsSearchRequest,
} = await import(requestParamsModule);

test('builds a default Trusted Shops search plan', () => {
    const plan = buildTrustedShopsSearchPlan({ q: ' zalando ' });

    assert.deepEqual(plan, {
        baseParams: {
            q: 'zalando',
            market: 'DEU',
        },
        startPage: 0,
        maxPages: 1,
    });
    assert.deepEqual(buildPageParams(plan, 0), {
        q: 'zalando',
        market: 'DEU',
        page: 0,
    });
});

test('normalizes market, numeric strings, encoded query, and ampersands', () => {
    const plan = buildTrustedShopsSearchPlan({
        q: 'm%C3%BCller & partner',
        market: 'fra',
        page: '2',
        max_pages: '3',
    });

    assert.deepEqual(plan, {
        baseParams: {
            q: 'müller partner',
            market: 'FRA',
        },
        startPage: 2,
        maxPages: 3,
    });
    assert.equal(describeTrustedShopsSearchRequest(plan), '"müller partner" in FRA (pages 2-4)');
});

test('validates required query, market, and pagination bounds', () => {
    assert.throws(
        () => buildTrustedShopsSearchPlan({ q: 'a' }),
        /q must be at least 2 characters/,
    );
    assert.throws(
        () => buildTrustedShopsSearchPlan({ q: 'zalando', market: 'USA' }),
        /market must be one of/,
    );
    assert.throws(
        () => buildTrustedShopsSearchPlan({ q: 'zalando', page: 101 }),
        /page must be between 0 and 100/,
    );
    assert.throws(
        () => buildTrustedShopsSearchPlan({ q: 'zalando', max_pages: 11 }),
        /max_pages must be between 1 and 10/,
    );
    assert.throws(
        () => buildTrustedShopsSearchPlan({ q: 'zalando', page: 95, max_pages: 10 }),
        /page plus max_pages cannot exceed page 100/,
    );
});

test('input schema matches the Trusted Shops search contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.deepEqual(schema.required, ['q']);
    assert.deepEqual(schema.properties.market.enum, ['DEU', 'GBR', 'AUT', 'CHE', 'NLD', 'ESP', 'ITA', 'FRA', 'BEL', 'POL', 'PRT']);
    assert.equal(schema.properties.page.minimum, 0);
    assert.equal(schema.properties.page.maximum, 100);
    assert.equal(schema.properties.max_pages.maximum, 10);
});
