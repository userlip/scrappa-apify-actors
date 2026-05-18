import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildJamedaSearchPlan,
    buildPageParams,
    describeJamedaSearchRequest,
} = await import(requestParamsModule);

test('builds a default Jameda search plan', () => {
    const plan = buildJamedaSearchPlan({ q: ' Zahnarzt ' });

    assert.deepEqual(plan, {
        baseParams: {
            q: 'Zahnarzt',
            loc: undefined,
            per_page: 28,
        },
        startPage: 1,
        perPage: 28,
        maxPages: 1,
    });
    assert.deepEqual(buildPageParams(plan, 1), {
        q: 'Zahnarzt',
        loc: undefined,
        per_page: 28,
        page: 1,
    });
});

test('normalizes numeric strings and encoded German query text', () => {
    const plan = buildJamedaSearchPlan({
        q: 'HNO%20Arzt',
        loc: 'M%C3%BCnchen',
        page: '2',
        per_page: '10',
        max_pages: '3',
    });

    assert.deepEqual(plan, {
        baseParams: {
            q: 'HNO Arzt',
            loc: 'München',
            per_page: 10,
        },
        startPage: 2,
        perPage: 10,
        maxPages: 3,
    });
    assert.equal(describeJamedaSearchRequest(plan), '"HNO Arzt" in München (pages 2-4, 10 per page)');
});

test('preserves ampersands and percent characters in query text', () => {
    assert.equal(buildJamedaSearchPlan({ q: 'Hals & Nase' }).baseParams.q, 'Hals & Nase');
    assert.equal(buildJamedaSearchPlan({ q: '100% privat' }).baseParams.q, '100% privat');
});

test('validates required query and pagination bounds', () => {
    assert.throws(
        () => buildJamedaSearchPlan({ q: 'a' }),
        /q must be at least 2 characters/,
    );
    assert.throws(
        () => buildJamedaSearchPlan({ q: 'Zahnarzt', loc: 'x'.repeat(101) }),
        /loc must be 100 characters or fewer/,
    );
    assert.throws(
        () => buildJamedaSearchPlan({ q: 'Zahnarzt', page: 0 }),
        /page must be between 1 and 500/,
    );
    assert.throws(
        () => buildJamedaSearchPlan({ q: 'Zahnarzt', per_page: 29 }),
        /per_page must be between 1 and 28/,
    );
    assert.throws(
        () => buildJamedaSearchPlan({ q: 'Zahnarzt', max_pages: 11 }),
        /max_pages must be between 1 and 10/,
    );
    assert.throws(
        () => buildJamedaSearchPlan({ q: 'Zahnarzt', page: 495, max_pages: 10 }),
        /page plus max_pages cannot exceed page 500/,
    );
});

test('input schema matches the Jameda search contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.deepEqual(schema.required, ['q']);
    assert.equal(schema.properties.page.minimum, 1);
    assert.equal(schema.properties.page.maximum, 500);
    assert.equal(schema.properties.per_page.maximum, 28);
    assert.equal(schema.properties.max_pages.maximum, 10);
});
