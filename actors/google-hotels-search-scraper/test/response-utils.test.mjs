import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const { buildHotelDatasetItem, getHotelProperties } = await import(responseUtilsModule);

test('extracts properties from primary and legacy response shapes', () => {
    assert.deepEqual(getHotelProperties({ properties: [{ name: 'Hotel A' }] }), [{ name: 'Hotel A' }]);
    assert.deepEqual(getHotelProperties({ data: { hotels: [{ name: 'Hotel B' }] } }), [{ name: 'Hotel B' }]);
    assert.deepEqual(getHotelProperties({}), []);
});

test('builds normalized hotel dataset item', () => {
    const item = buildHotelDatasetItem(
        {
            name: 'Hotel Le Test',
            entity_id: 'entity-1',
            place_id: 'place-1',
            gps_coordinates: { latitude: 48.85, longitude: 2.35 },
            rate_per_night: { lowest: '$200', extracted_lowest: 200 },
            total_rate: { lowest: '$600', extracted_lowest: 600 },
            property_link: 'https://example.com/book',
            prices: [{ source: 'Booking' }],
            amenities: ['Free Wi-Fi', 'Pool'],
        },
        {
            q: 'Paris',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            adults: 2,
            currency: 'USD',
        },
    );

    assert.equal(item.name, 'Hotel Le Test');
    assert.equal(item.property_token, 'entity-1');
    assert.equal(item.latitude, 48.85);
    assert.equal(item.longitude, 2.35);
    assert.equal(item.rate_per_night_lowest, '$200');
    assert.equal(item.total_rate_extracted_lowest, 600);
    assert.equal(item.booking_link, 'https://example.com/book');
    assert.equal(item.price_sources_count, 1);
    assert.equal(item.amenities_count, 2);
    assert.equal(item.request_q, 'Paris');
});
