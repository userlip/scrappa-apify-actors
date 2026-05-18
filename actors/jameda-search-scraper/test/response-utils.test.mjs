import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildJamedaDoctorDatasetItem,
    getJamedaDoctors,
} = await import(responseUtilsModule);

test('extracts doctors from the Scrappa Jameda response shape', () => {
    assert.deepEqual(getJamedaDoctors({ data: [{ name: 'Dr. A' }] }), [{ name: 'Dr. A' }]);
    assert.deepEqual(getJamedaDoctors({ data: [] }), []);
    assert.deepEqual(getJamedaDoctors({}), []);
});

test('builds a normalized Jameda doctor dataset item', () => {
    const item = buildJamedaDoctorDatasetItem(
        {
            name: 'Dr. med. Beispiel',
            specialty: 'Zahnarzt',
            url: '/beispiel/zahnarzt/berlin',
            rating: '1,2',
            review_count: '1.234 Bewertungen',
            address: 'Beispielstr. 1, 10115 Berlin',
            image_url: 'https://www.jameda.de/example.jpg',
        },
        {
            q: 'Zahnarzt',
            loc: 'Berlin',
            page: 1,
            per_page: 28,
        },
        {
            meta: {
                total_results: 120,
                total_pages: 5,
                has_next_page: true,
            },
        },
    );

    assert.equal(item.name, 'Dr. med. Beispiel');
    assert.equal(item.profile_url, 'https://www.jameda.de/beispiel/zahnarzt/berlin');
    assert.equal(item.review_count_number, 1234);
    assert.equal(item.request_q, 'Zahnarzt');
    assert.equal(item.request_loc, 'Berlin');
    assert.equal(item.total_results, 120);
    assert.equal(item.total_pages, 5);
});

test('normalizes fallback rating data and full HTTP Jameda URLs', () => {
    const item = buildJamedaDoctorDatasetItem(
        {
            name: 'Dr. A',
            url: 'http://www.jameda.de/a/hausarzt/hamburg',
            jameda_rating: {
                rating: '1,0',
                count: '9',
            },
        },
        { q: 'Hausarzt', page: 2, per_page: 10 },
        {},
    );

    assert.equal(item.profile_url, 'https://www.jameda.de/a/hausarzt/hamburg');
    assert.equal(item.rating, '1,0');
    assert.equal(item.review_count, '9');
    assert.equal(item.review_count_number, 9);
});
