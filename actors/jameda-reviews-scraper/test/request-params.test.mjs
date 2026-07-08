import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildJamedaReviewsParams,
    buildJamedaReviewsPlan,
    cleanJamedaDoctorUrl,
    cleanRatingFilter,
    describeJamedaReviewsRequest,
} = await import(requestParamsModule);

const markusUrl = 'https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin';

test('builds params for a single Jameda reviews request', () => {
    const plan = buildJamedaReviewsPlan({
        doctor_url: ' https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin?utm_source=test ',
        page: '2',
        sort: 'highest',
        rating: '4,5',
        per_page: '50',
    });

    assert.deepEqual(plan, {
        doctorUrls: [markusUrl],
        inputFailures: [],
        page: 2,
        sort: 'highest',
        rating: '4,5',
        perPage: 50,
    });
    assert.deepEqual(buildJamedaReviewsParams(plan, markusUrl), {
        doctor_url: markusUrl,
        page: 2,
        per_page: 50,
        sort: 'highest',
        rating: '4,5',
    });
    assert.equal(describeJamedaReviewsRequest(plan), `${markusUrl} (page 2, 50 per page, sort highest, rating 4,5)`);
});

test('accepts and deduplicates batch doctor URLs', () => {
    const plan = buildJamedaReviewsPlan({
        doctor_url: markusUrl,
        doctor_urls: [
            markusUrl,
            '/markus-lietzau-msc/zahnarzt/berlin',
            'markus-lietzau-msc/zahnarzt/berlin/',
            'https://www.jameda.de/anna-example/aerztin/hamburg',
        ],
    });

    assert.deepEqual(plan.doctorUrls, [
        markusUrl,
        'https://www.jameda.de/anna-example/aerztin/hamburg',
    ]);
    assert.equal(describeJamedaReviewsRequest(plan), '2 doctor URLs (page 1, 20 per page)');
});

test('accepts comma and newline separated batch doctor URLs', () => {
    const plan = buildJamedaReviewsPlan({
        doctor_urls: `${markusUrl}, /anna-example/aerztin/hamburg\n/hans-example/orthopaede/muenchen`,
    });

    assert.deepEqual(plan.doctorUrls, [
        markusUrl,
        'https://www.jameda.de/anna-example/aerztin/hamburg',
        'https://www.jameda.de/hans-example/orthopaede/muenchen',
    ]);
});

test('normalizes http, host-style, path-only, and facility URLs', () => {
    assert.equal(
        cleanJamedaDoctorUrl('http://jameda.de/markus-lietzau-msc/zahnarzt/berlin'),
        markusUrl,
    );

    assert.equal(
        cleanJamedaDoctorUrl('www.jameda.de/markus-lietzau-msc/zahnarzt/berlin'),
        markusUrl,
    );

    assert.equal(
        cleanJamedaDoctorUrl('/gesundheitseinrichtungen/example-klinik#filters[doctor_id]=123'),
        'https://www.jameda.de/gesundheitseinrichtungen/example-klinik#filters[doctor_id]=123',
    );
});

test('rejects generic Jameda listing paths that are not review profile URLs', () => {
    assert.throws(
        () => cleanJamedaDoctorUrl('/aerzte/berlin'),
        /doctor profile path/,
    );

    assert.throws(
        () => cleanJamedaDoctorUrl('/search'),
        /doctor profile path/,
    );
});

test('normalizes rating filters from strings and arrays', () => {
    assert.equal(cleanRatingFilter('5, 4,4'), '5,4');
    assert.equal(cleanRatingFilter([1, '2', ' 5 ']), '1,2,5');
    assert.equal(cleanRatingFilter(''), undefined);

    assert.throws(
        () => cleanRatingFilter('0,5'),
        /rating must contain only values from 1 to 5/,
    );
});

test('collects invalid URL entries when valid entries are present', () => {
    const plan = buildJamedaReviewsPlan({
        doctor_urls: [
            markusUrl,
            'https://example.com/markus-lietzau-msc/zahnarzt/berlin',
            '/search',
            123,
        ],
    });

    assert.deepEqual(plan.doctorUrls, [markusUrl]);
    assert.equal(plan.inputFailures.length, 3);
    assert.match(plan.inputFailures[0].error, /jameda.de domain/);
    assert.match(plan.inputFailures[1].error, /doctor profile path/);
    assert.match(plan.inputFailures[2].error, /doctor_urls must be a string/);
});

test('rejects invalid overall input and filters', () => {
    assert.throws(
        () => buildJamedaReviewsPlan({}),
        /Provide doctor_urls or doctor_url/,
    );

    assert.throws(
        () => buildJamedaReviewsPlan({ doctor_urls: 123 }),
        /doctor_urls must be an array/,
    );

    assert.throws(
        () => buildJamedaReviewsPlan({ doctor_url: 'https://example.com/markus-lietzau-msc/zahnarzt/berlin' }),
        /No valid Jameda doctor URLs/,
    );

    assert.throws(
        () => buildJamedaReviewsPlan({ doctor_url: markusUrl, sort: 'popular' }),
        /sort must be one of/,
    );

    assert.throws(
        () => buildJamedaReviewsPlan({ doctor_url: markusUrl, per_page: 101 }),
        /per_page must be between 1 and 100/,
    );

    assert.throws(
        () => buildJamedaReviewsPlan({
            doctor_urls: Array.from({ length: 101 }, (_, index) => `/doctor-${index}/zahnarzt/berlin`),
        }),
        /at most 100 doctor URLs/,
    );
});
