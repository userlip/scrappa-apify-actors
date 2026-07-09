import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildUserPageParams,
    buildVintedUserItemsPlan,
    describeVintedUserItemsRequest,
} = await import(requestParamsModule);

test('builds a Vinted user-items plan from a single user_id', () => {
    const plan = buildVintedUserItemsPlan({ user_id: ' 12345678 ' });

    assert.deepEqual(plan, {
        baseParams: {
            country: 'FR',
            per_page: 24,
            order: 'newest_first',
        },
        userIds: ['12345678'],
        startPage: 1,
        perPage: 24,
        maxPages: 1,
    });
    assert.deepEqual(buildUserPageParams(plan, '12345678', 1), {
        country: 'FR',
        per_page: 24,
        order: 'newest_first',
        user_id: '12345678',
        page: 1,
    });
});

test('normalizes batched user_ids, country, pagination, and order', () => {
    const plan = buildVintedUserItemsPlan({
        user_id: 12345678,
        user_ids: ['87654321', '12345678', ' 22222222,33333333 '],
        country: 'de',
        page: '2',
        per_page: '50',
        max_pages: '3',
        order: 'newest_first',
    });

    assert.deepEqual(plan, {
        baseParams: {
            country: 'DE',
            per_page: 50,
            order: 'newest_first',
        },
        userIds: ['12345678', '87654321', '22222222', '33333333'],
        startPage: 2,
        perPage: 50,
        maxPages: 3,
    });
    assert.equal(describeVintedUserItemsRequest(plan), '4 sellers in DE (pages 2-4, 50/page)');
});

test('describes a single seller request', () => {
    const plan = buildVintedUserItemsPlan({
        user_ids: ['12345678'],
        country: 'FR',
    });

    assert.equal(describeVintedUserItemsRequest(plan), 'seller 12345678 in FR (page 1, 24/page)');
});

test('validates user IDs, country, sorting, and pagination bounds', () => {
    assert.throws(
        () => buildVintedUserItemsPlan({}),
        /Provide at least one Vinted seller user_id/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_ids: ['abc'] }),
        /user_ids must contain numeric Vinted user IDs/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_ids: Array.from({ length: 101 }, (_, index) => String(index + 1)) }),
        /user_ids supports at most 100 unique IDs per run/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_id: '123', country: 'GB' }),
        /country must be one of/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_id: '123', order: 'oldest_first' }),
        /order must be one of/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_id: '123', page: 0 }),
        /page must be between 1 and 999/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_id: '123', max_pages: 21 }),
        /max_pages must be between 1 and 20/,
    );
    assert.throws(
        () => buildVintedUserItemsPlan({ user_id: '123', page: 990, max_pages: 20 }),
        /page plus max_pages cannot exceed page 999/,
    );
});

test('input schema matches the Vinted user-items contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.user_id.pattern, '^\\d*$');
    assert.equal(schema.properties.user_ids.maxItems, 100);
    assert.equal(schema.properties.user_ids.items.pattern, '^\\d+$');
    assert.deepEqual(schema.properties.country.enum, ['FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT', 'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US']);
    assert.deepEqual(schema.properties.order.enum, ['newest_first', 'price_low_to_high', 'price_high_to_low', 'relevance']);
    assert.equal(schema.properties.page.minimum, 1);
    assert.equal(schema.properties.page.maximum, 999);
    assert.equal(schema.properties.per_page.maximum, 100);
    assert.equal(schema.properties.max_pages.maximum, 20);
});
