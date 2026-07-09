import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildJamedaReviewDatasetItem,
    buildJamedaReviewsOutputSummary,
    getJamedaReviews,
} = await import(responseUtilsModule);

const doctorUrl = 'https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin';

test('builds normalized Jameda review dataset item', () => {
    const response = {
        success: true,
        data: [
            {
                id: '4986285',
                text: 'Sehr freundlicher und kompetenter Zahnarzt!',
                rating: '4',
                date: '2025-07-15T17:38:48+02:00',
                date_formatted: '15. Juli 2025',
                verification_badge: 'Termin verifiziert',
                categories: [{ label: 'Behandlung', rating: '5' }],
            },
        ],
        meta: {
            url: doctorUrl,
            doctor_url: 'markus-lietzau-msc/zahnarzt/berlin',
            filters: {
                sort: 'newest',
                rating: null,
            },
            pagination: {
                currentPage: 1,
                totalPages: 44,
                totalReviews: 132,
                resultsPerPage: 3,
                hasNextPage: true,
                hasPreviousPage: false,
            },
            doctor: {
                name: 'Markus Lietzau M.Sc.',
                specializations: 'Zahnarzt',
                overall_rating: '4,5',
            },
            source: 'jameda_ajax_api',
        },
    };

    const item = buildJamedaReviewDatasetItem(response.data[0], {
        inputDoctorUrl: doctorUrl,
        params: {
            doctor_url: doctorUrl,
            page: 1,
            per_page: 3,
            sort: 'newest',
            rating: '4,5',
        },
        response,
    });

    assert.equal(item.review_id, '4986285');
    assert.equal(item.review_text, 'Sehr freundlicher und kompetenter Zahnarzt!');
    assert.equal(item.rating, '4');
    assert.equal(item.rating_number, 4);
    assert.equal(item.date, '2025-07-15T17:38:48+02:00');
    assert.equal(item.date_formatted, '15. Juli 2025');
    assert.equal(item.verification_badge, 'Termin verifiziert');
    assert.deepEqual(item.categories, [{ label: 'Behandlung', rating: '5' }]);
    assert.equal(item.doctor_name, 'Markus Lietzau M.Sc.');
    assert.equal(item.doctor_specializations, 'Zahnarzt');
    assert.equal(item.doctor_overall_rating_number, 4.5);
    assert.equal(item.input_doctor_url, doctorUrl);
    assert.equal(item.normalized_doctor_url, doctorUrl);
    assert.equal(item.request_page, 1);
    assert.equal(item.request_sort, 'newest');
    assert.equal(item.request_rating, '4,5');
    assert.equal(item.request_per_page, 3);
    assert.equal(item.response_url, doctorUrl);
    assert.equal(item.response_doctor_url, 'markus-lietzau-msc/zahnarzt/berlin');
    assert.equal(item.total_reviews, 132);
    assert.equal(item.total_pages, 44);
    assert.equal(item.has_next_page, true);
    assert.equal(item.response_source, 'jameda_ajax_api');
});

test('tolerates sparse review responses', () => {
    const item = buildJamedaReviewDatasetItem(
        { text: 'Sparse review' },
        {
            inputDoctorUrl: doctorUrl,
            params: {
                doctor_url: doctorUrl,
                page: 1,
                per_page: 20,
            },
            response: {},
        },
    );

    assert.equal(item.review_id, null);
    assert.equal(item.review_text, 'Sparse review');
    assert.equal(item.rating_number, null);
    assert.deepEqual(item.categories, []);
    assert.equal(item.total_reviews, null);
});

test('returns empty array for empty or malformed review payloads', () => {
    assert.deepEqual(getJamedaReviews({ success: true, data: [] }), []);
    assert.deepEqual(getJamedaReviews({ success: true, data: null }), []);
    assert.deepEqual(getJamedaReviews({ success: true }), []);
});

test('throws when Scrappa reports an unsuccessful reviews response', () => {
    assert.throws(
        () => getJamedaReviews({ success: false, message: 'Doctor not found' }),
        /Doctor not found/,
    );

    assert.throws(
        () => getJamedaReviews({ success: false }),
        /not successful/,
    );
});

test('builds OUTPUT summary without raw uncharged responses', () => {
    const summary = buildJamedaReviewsOutputSummary({
        doctorUrls: [doctorUrl, 'https://www.jameda.de/example/aerztin/hamburg'],
        savedReviews: 3,
        failures: [{ doctor_url: 'https://www.jameda.de/example/aerztin/hamburg', error: '404' }],
        statusMessage: '1 Jameda review request failed; 3 reviews saved.',
    });

    assert.deepEqual(summary, {
        request: {
            endpoint: '/jameda/reviews',
            doctor_urls: [doctorUrl, 'https://www.jameda.de/example/aerztin/hamburg'],
        },
        doctors_requested: 2,
        reviews_saved: 3,
        requests_failed: 1,
        status_message: '1 Jameda review request failed; 3 reviews saved.',
        failures: [{ doctor_url: 'https://www.jameda.de/example/aerztin/hamburg', error: '404' }],
    });
});
