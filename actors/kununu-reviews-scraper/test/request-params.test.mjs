import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildKununuReviewsPlan,
    buildPageParams,
    describeKununuReviewsRequest,
} from '../dist/request-params.js';

test('builds params for batch Kununu review targets', () => {
    const plan = buildKununuReviewsPlan({
        targets: ['de/bmwgroup', 'https://www.kununu.com/de/sap-se?utm=test'],
        page: 2,
        max_pages: 3,
        review_type: 'candidates',
        sort: 'newest',
        fetch_factor_scores: true,
    });

    assert.deepEqual(plan.targets, [
        { country: 'de', company_slug: 'bmwgroup', input: 'de/bmwgroup' },
        { country: 'de', company_slug: 'sap-se', input: 'https://www.kununu.com/de/sap-se?utm=test' },
    ]);
    assert.deepEqual(plan.baseParams, {
        review_type: 'candidates',
        sort: 'newest',
        fetch_factor_scores: 1,
    });
    assert.equal(plan.startPage, 2);
    assert.equal(plan.maxPages, 3);

    assert.deepEqual(buildPageParams(plan, plan.targets[0], 2), {
        review_type: 'candidates',
        sort: 'newest',
        fetch_factor_scores: 1,
        country: 'de',
        company_slug: 'bmwgroup',
        page: 2,
    });
});

test('supports single slug compatibility input with default country', () => {
    const plan = buildKununuReviewsPlan({
        company_slug: ' BMWGROUP ',
        country: 'DE',
    });

    assert.deepEqual(plan.targets, [
        { country: 'de', company_slug: 'bmwgroup', input: 'BMWGROUP' },
    ]);
    assert.deepEqual(plan.baseParams, {});
    assert.equal(describeKununuReviewsRequest(plan), 'de/bmwgroup (page 1)');
});

test('supports object targets with uuid alias', () => {
    const companyId = '123e4567-e89b-12d3-a456-426614174000';
    const plan = buildKununuReviewsPlan({
        companies: [
            {
                countryCode: 'AT',
                slug: 'example-company',
                uuid: companyId,
            },
        ],
    });

    assert.deepEqual(plan.targets, [
        {
            country: 'at',
            company_slug: 'example-company',
            company_id: companyId,
            input: 'at/example-company',
        },
    ]);
});

test('builds filter arrays as Scrappa params', () => {
    const plan = buildKununuReviewsPlan({
        targets: ['bmwgroup'],
        score_filters: ['Excellent', 'good'],
        recommended_filters: 'yes',
        jobstatus_filters: ['current'],
        position_filters: ['employee', 'manager'],
        department_filters: ['it'],
        response_filters: ['no'],
        date_filters: ['30days'],
    });

    assert.deepEqual(plan.baseParams, {
        score_filters: ['excellent', 'good'],
        recommended_filters: ['yes'],
        jobstatus_filters: ['current'],
        position_filters: ['employee', 'manager'],
        department_filters: ['it'],
        response_filters: ['no'],
        date_filters: ['30days'],
    });
});

test('rejects invalid Kununu request params', () => {
    assert.throws(
        () => buildKununuReviewsPlan({}),
        /Provide at least one Kununu company target/,
    );

    assert.throws(
        () => buildKununuReviewsPlan({ targets: ['fr/example'] }),
        /targets with a slash must use a supported country\/slug pair/,
    );

    assert.throws(
        () => buildKununuReviewsPlan({ targets: ['bmwgroup'], country: 'fr' }),
        /country must be one of/,
    );

    assert.throws(
        () => buildKununuReviewsPlan({ targets: ['bmwgroup'], max_pages: 26 }),
        /max_pages must be between 1 and 25/,
    );

    assert.throws(
        () => buildKununuReviewsPlan({ targets: ['bmwgroup'], review_type: 'customers' }),
        /review_type must be one of/,
    );

    assert.throws(
        () => buildKununuReviewsPlan({ targets: ['bmwgroup'], score_filters: ['bad'] }),
        /score_filters values must be one of/,
    );
});
