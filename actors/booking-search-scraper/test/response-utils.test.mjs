import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const { buildBookingDatasetItem, getBookingSearchResults } = await import(responseUtilsModule);

test('extracts Booking.com results from data.results', () => {
    const results = [{ name: 'Hotel One' }, { name: 'Hotel Two' }];
    assert.deepEqual(getBookingSearchResults({ data: { results } }), results);
});

test('extracts Booking.com results from top-level results fallback', () => {
    const results = [{ name: 'Hotel One' }];
    assert.deepEqual(getBookingSearchResults({ results }), results);
});

test('returns empty list when no results are present', () => {
    assert.deepEqual(getBookingSearchResults({ data: {} }), []);
});

test('builds normalized Booking.com dataset item', () => {
    const item = buildBookingDatasetItem(
        {
            title: 'Hotel Example',
            link: 'https://www.booking.com/hotel/fr/example.html',
            thumbnail: 'https://example.com/image.jpg',
            review_score: '8.7',
            review_score_word: 'Excellent',
            review_count: '1240',
            address: 'Paris',
            price_for_display: 'EUR 420',
        },
        {
            ss: 'Paris',
            checkin: '2026-06-01',
            checkout: '2026-06-04',
            group_adults: 2,
            no_rooms: 1,
            lang: 'en-us',
            currency: 'EUR',
        },
        1,
    );

    assert.equal(item.name, 'Hotel Example');
    assert.equal(item.url, 'https://www.booking.com/hotel/fr/example.html');
    assert.equal(item.image, 'https://example.com/image.jpg');
    assert.equal(item.review_score, 8.7);
    assert.equal(item.review_score_word, 'Excellent');
    assert.equal(item.review_count, 1240);
    assert.equal(item.location, 'Paris');
    assert.equal(item.price, 'EUR 420');
    assert.equal(item.currency, 'EUR');
    assert.equal(item.request_search_index, 1);
    assert.equal(item.request_ss, 'Paris');
    assert.equal(item.request_checkin, '2026-06-01');
    assert.equal(item.request_checkout, '2026-06-04');
    assert.equal(item.request_group_adults, 2);
    assert.equal(item.request_no_rooms, 1);
    assert.equal(item.request_lang, 'en-us');
    assert.equal(item.request_currency, 'EUR');
});

test('ignores non-finite numeric strings in normalized fields', () => {
    const item = buildBookingDatasetItem(
        {
            name: 'Hotel Example',
            review_score: 'Infinity',
            review_count: '-Infinity',
        },
        { ss: 'Paris' },
        0,
    );

    assert.equal(item.review_score, null);
    assert.equal(item.review_count, null);
});
