import assert from 'node:assert/strict';
import test from 'node:test';

import {
    collectReviews,
    enrichReview,
} from '../dist/review-processing.js';

test('collects Kununu reviews and dedupes by uuid', () => {
    const collected = collectReviews({
        data: [
            { uuid: 'review-1', title: 'First' },
            { uuid: ' review-1 ', title: 'Duplicate' },
            { title: 'Missing UUID' },
            { uuid: 'review-2', title: 'Second' },
        ],
    });

    assert.deepEqual(
        collected.map((review) => review.title),
        ['First', 'Missing UUID', 'Second'],
    );
});

test('returns no reviews when Scrappa response has no data array', () => {
    assert.deepEqual(collectReviews({ success: true }), []);
});

test('enriches reviews with target, normalized text, and reviewer metadata', () => {
    const review = {
        uuid: 'review-1',
        type: 'employees',
        title: 'Good employer',
        score: 4.2,
        roundedScore: 4,
        createdAt: '2026-05-01T12:00:00.000Z',
        updatedAt: '2026-05-02T12:00:00.000Z',
        texts: [
            { id: 'pros', text: 'Good team' },
            { id: 'cons', text: 'Long meetings' },
        ],
        company: {
            uuid: 'company-1',
            name: 'Example GmbH',
        },
        position: 'employee',
        department: 'it',
        employmentType: 'full-time',
        jobStatus: 'current',
        isCurrentEmployee: true,
        isRecommended: true,
        city: 'Munich',
        responses: [{ id: 'response-1', text: 'Thanks' }],
        ratings: [{ id: 'workLifeBalance', score: 4.5 }],
    };

    const enriched = enrichReview(
        review,
        { country: 'de', company_slug: 'example-gmbh', input: 'de/example-gmbh' },
        { page: 2, sort: 'newest', score_filters: ['good'] },
        {
            meta: {
                pagination: {
                    totalResults: 123,
                    totalPages: 7,
                },
            },
        },
    );

    assert.equal(enriched.review_id, 'review-1');
    assert.equal(enriched.company_target, 'de/example-gmbh');
    assert.equal(enriched.company_country, 'de');
    assert.equal(enriched.company_slug, 'example-gmbh');
    assert.equal(enriched.company_id, 'company-1');
    assert.equal(enriched.company_name, 'Example GmbH');
    assert.equal(enriched.rating, 4.2);
    assert.equal(enriched.rounded_rating, 4);
    assert.equal(enriched.text, 'Good team\n\nLong meetings');
    assert.equal(enriched.texts, undefined);
    assert.equal(enriched.date, '2026-05-01T12:00:00.000Z');
    assert.equal(enriched.reviewer_position, 'employee');
    assert.equal(enriched.reviewer_department, 'it');
    assert.equal(enriched.reviewer_employment_status, 'current');
    assert.equal(enriched.reviewer_recommended, true);
    assert.equal(enriched.page, 2);
    assert.equal(enriched.source_url, 'https://www.kununu.com/de/example-gmbh');
    assert.deepEqual(enriched.request_score_filters, ['good']);
    assert.equal(enriched.page_total_results, 123);
    assert.equal(enriched.page_total_pages, 7);
    assert.equal(enriched.raw_review, undefined);
});

test('includes raw review only when requested', () => {
    const review = { uuid: 'review-raw', title: 'Raw review' };
    const enriched = enrichReview(
        review,
        { country: 'de', company_slug: 'example-gmbh', input: 'de/example-gmbh' },
        { page: 1 },
        {},
        { includeRawReview: true },
    );

    assert.equal(enriched.raw_review, review);
});
