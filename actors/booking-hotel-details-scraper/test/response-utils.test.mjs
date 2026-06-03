import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildBookingHotelDatasetItem,
    buildBookingHotelErrorItem,
    getBookingHotelDetails,
} = await import(responseUtilsModule);

test('extracts Booking.com hotel details from data payload', () => {
    const details = { title: 'Ritz Paris', parsed: true };
    assert.deepEqual(getBookingHotelDetails({ success: true, data: details }), details);
});

test('falls back to top-level response when data payload is absent', () => {
    assert.deepEqual(
        getBookingHotelDetails({ title: 'Ritz Paris', parsed: true }),
        { title: 'Ritz Paris', parsed: true },
    );
});

test('builds normalized Booking.com hotel dataset item', () => {
    const item = buildBookingHotelDatasetItem(
        {
            canonical_url: 'https://www.booking.com/hotel/fr/ritz-paris.html',
            hotel_schema: {
                '@type': 'Hotel',
                name: 'Ritz Paris',
                aggregateRating: {
                    ratingValue: '9.4',
                    reviewCount: '512',
                },
            },
            json_ld: [{ '@type': 'Hotel' }],
            parsed: true,
        },
        {
            index: 2,
            inputType: 'country_slug',
            params: {
                country: 'fr',
                slug: 'ritz-paris',
            },
        },
    );

    assert.equal(item.title, 'Ritz Paris');
    assert.equal(item.canonical_url, 'https://www.booking.com/hotel/fr/ritz-paris.html');
    assert.deepEqual(item.aggregate_rating, {
        ratingValue: '9.4',
        reviewCount: '512',
    });
    assert.deepEqual(item.json_ld, [{ '@type': 'Hotel' }]);
    assert.equal(item.parsed, true);
    assert.equal(item.request_index, 2);
    assert.equal(item.request_input_type, 'country_slug');
    assert.equal(item.request_country, 'fr');
    assert.equal(item.request_slug, 'ritz-paris');
    assert.equal(item.request_url, null);
    assert.equal(item.request_success, true);
});

test('preserves explicit aggregate rating and nullable detail fields', () => {
    const item = buildBookingHotelDatasetItem(
        {
            title: 'Hotel Example',
            aggregate_rating: { ratingValue: '8.8' },
        },
        {
            index: 0,
            inputType: 'url',
            params: {
                url: 'https://www.booking.com/hotel/de/example.html',
            },
        },
    );

    assert.equal(item.title, 'Hotel Example');
    assert.deepEqual(item.aggregate_rating, { ratingValue: '8.8' });
    assert.equal(item.hotel_schema, null);
    assert.equal(item.json_ld, null);
    assert.equal(item.parsed, false);
    assert.equal(item.request_url, 'https://www.booking.com/hotel/de/example.html');
});

test('builds uncharged error dataset item metadata', () => {
    const item = buildBookingHotelErrorItem(
        {
            index: 1,
            inputType: 'url',
            params: {
                url: 'https://www.booking.com/hotel/fr/ritz-paris.html',
            },
        },
        new Error('Scrappa API error (422): Invalid request'),
    );

    assert.deepEqual(item, {
        request_index: 1,
        request_input_type: 'url',
        request_url: 'https://www.booking.com/hotel/fr/ritz-paris.html',
        request_country: null,
        request_slug: null,
        request_success: false,
        error_message: 'Scrappa API error (422): Invalid request',
    });
});
