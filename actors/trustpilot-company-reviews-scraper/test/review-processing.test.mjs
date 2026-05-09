import assert from 'node:assert/strict';
import test from 'node:test';

import {
    collectReviews,
    enrichReview,
    publishedDate,
} from '../dist/review-processing.js';

test('collects reviews from all Trustpilot review arrays and dedupes only by ID', () => {
    const duplicate = { id: 'review-1', title: 'Same review' };
    const missingIdOne = { title: 'Repeated title', text: 'Repeated text', rating: 5 };
    const missingIdTwo = { title: 'Repeated title', text: 'Repeated text', rating: 5 };

    const collected = collectReviews({
        reviews: [duplicate, missingIdOne],
        relevantReviews: [{ id: ' review-1 ', title: 'Duplicate by trimmed ID' }, missingIdTwo],
        aiSummaryReviews: [{ id: 'review-2', title: 'Unique AI summary review' }],
    });

    assert.deepEqual(
        collected.map(({ review, source }) => ({ id: review.id, title: review.title, source })),
        [
            { id: 'review-1', title: 'Same review', source: 'reviews' },
            { id: undefined, title: 'Repeated title', source: 'reviews' },
            { id: undefined, title: 'Repeated title', source: 'relevantReviews' },
            { id: 'review-2', title: 'Unique AI summary review', source: 'aiSummaryReviews' },
        ],
    );
});

test('normalizes published dates from Trustpilot review fields', () => {
    assert.equal(
        publishedDate({ dates: { publishedDate: '2026-05-01T12:00:00.000Z' }, createdAt: 1 }),
        '2026-05-01T12:00:00.000Z',
    );
    assert.equal(publishedDate({ createdAt: 1 }), '1970-01-01T00:00:01.000Z');
    assert.equal(publishedDate({ createdAt: '2026-05-02' }), '2026-05-02');
    assert.equal(publishedDate({ createdAt: '   ' }), undefined);
});

test('enriches reviews with request and pagination context', () => {
    const enriched = enrichReview(
        {
            id: 'review-1',
            consumer: { displayName: 'Jane Doe' },
            createdAt: 1,
        },
        {
            company_domain: 'example.com',
            locale: 'en-US',
            page: 2,
        },
        {
            pagination: {
                total_count: 123,
                total_pages: 7,
            },
        },
        'reviews',
    );

    assert.equal(enriched.id, 'review-1');
    assert.equal(enriched.review_source, 'reviews');
    assert.equal(enriched.consumer_name, 'Jane Doe');
    assert.equal(enriched.published_date, '1970-01-01T00:00:01.000Z');
    assert.equal(enriched.company_domain, 'example.com');
    assert.equal(enriched.request_sort, 'recency');
    assert.equal(enriched.request_date_posted, 'any');
    assert.equal(enriched.request_rating, null);
    assert.equal(enriched.page_total_count, 123);
    assert.equal(enriched.page_total_pages, 7);
});
