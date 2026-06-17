import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    collectReviews,
    enrichReview,
    hasNextPage,
} = await import(responseUtilsModule);

const target = {
    tsid: 'XFB15FFBDE1DEE7A55D292A7D48598A6A',
    input: 'XFB15FFBDE1DEE7A55D292A7D48598A6A',
    sourceUrl: 'https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html',
};

test('collects TrustedShops reviews from common response shapes and dedupes by id', () => {
    assert.deepEqual(collectReviews({
        reviews: [
            { id: 'review-1', title: 'First' },
            { reviewId: 'review-1', title: 'Duplicate' },
        ],
        data: {
            reviews: [
                { review_id: 'review-2', title: 'Second' },
                { title: 'Missing ID' },
            ],
        },
    }).map((review) => review.title), ['First', 'Second', 'Missing ID']);

    assert.deepEqual(collectReviews({ data: [{ id: 'review-3' }] }), [{ id: 'review-3' }]);
    assert.deepEqual(
        collectReviews({ response: { data: { shop: { reviews: [{ id: 'review-4' }] } } } }),
        [{ id: 'review-4' }],
    );
    assert.deepEqual(collectReviews({}), []);
});

test('dedupes reviews without ids from stable review fields', () => {
    const collected = collectReviews({
        reviews: [
            {
                rating: 5,
                title: 'Same review',
                comment: 'Duplicate text',
                createdAt: 1780254823000,
                verificationStatus: 'MEMBER_VERIFIED',
            },
        ],
        data: {
            reviews: [
                {
                    rating: 5,
                    title: 'Same review',
                    comment: 'Duplicate text',
                    createdAt: 1780254823000,
                    verificationStatus: 'MEMBER_VERIFIED',
                },
                {
                    rating: 4,
                    title: 'Different review',
                    comment: 'Different text',
                    createdAt: 1780254823000,
                },
            ],
        },
    });

    assert.deepEqual(collected.map((review) => review.title), ['Same review', 'Different review']);
});

test('enriches review records with normalized fields and keeps raw fields', () => {
    const review = {
        id: 'review-1',
        shopName: 'Example Shop',
        ratingValue: 4.8,
        reviewTitle: 'Fast delivery',
        reviewText: 'Everything arrived quickly.',
        submittedAt: '2026-05-20T10:15:00Z',
        verifiedReview: true,
        criteria: { delivery: 5 },
        customField: 'preserved',
    };

    const enriched = enrichReview(
        review,
        target,
        { page: 2, size: 10, market: 'DEU' },
        {
            pagination: {
                total_count: 123,
                total_pages: 13,
            },
        },
    );

    assert.equal(enriched.tsid, target.tsid);
    assert.equal(enriched.shop_name, 'Example Shop');
    assert.equal(enriched.rating, 4.8);
    assert.equal(enriched.review_title, 'Fast delivery');
    assert.equal(enriched.review_text, 'Everything arrived quickly.');
    assert.equal(enriched.created_at, '2026-05-20T10:15:00Z');
    assert.equal(enriched.verified, true);
    assert.deepEqual(enriched.criteria, { delivery: 5 });
    assert.equal(enriched.review_id, 'review-1');
    assert.equal(enriched.page, 2);
    assert.equal(enriched.size, 10);
    assert.equal(enriched.source_url, target.sourceUrl);
    assert.equal(enriched.request_market, 'DEU');
    assert.equal(enriched.page_total_reviews, 123);
    assert.equal(enriched.page_total_pages, 13);
    assert.equal(enriched.customField, 'preserved');
    assert.equal(enriched.raw_review, undefined);
});

test('uses wrapped shop and pagination metadata when present', () => {
    const enriched = enrichReview(
        {
            reviewId: 'review-2',
            stars: 5,
            text: 'Great',
            createdAt: '2026-05-21',
            isVerified: false,
            ratings: [{ id: 'service', score: 5 }],
        },
        target,
        { page: 1, size: 20 },
        {
            data: {
                shop: { name: 'Wrapped Shop' },
                reviews: [],
            },
            metaData: {
                totalReviewCount: 9,
                totalPageCount: 1,
            },
        },
        { includeRawReview: true },
    );

    assert.equal(enriched.shop_name, 'Wrapped Shop');
    assert.equal(enriched.rating, 5);
    assert.equal(enriched.review_text, 'Great');
    assert.equal(enriched.created_at, '2026-05-21');
    assert.equal(enriched.verified, false);
    assert.deepEqual(enriched.criteria, [{ id: 'service', score: 5 }]);
    assert.equal(enriched.page_total_reviews, 9);
    assert.equal(enriched.page_total_pages, 1);
    assert.equal(typeof enriched.raw_review, 'object');
});

test('normalizes live Scrappa TrustedShops wrapper fields', () => {
    const enriched = enrichReview(
        {
            id: 'rev-live',
            rating: 5,
            title: 'Alles Gut',
            comment: 'Schnelle Lieferung.',
            createdAt: 1780254823000,
            verificationStatus: 'MEMBER_VERIFIED',
        },
        target,
        { page: 1, size: 10 },
        {
            response: {
                data: {
                    shop: {
                        reviewCount: 100,
                        reviews: [],
                    },
                },
            },
        },
    );

    assert.equal(enriched.review_id, 'rev-live');
    assert.equal(enriched.review_title, 'Alles Gut');
    assert.equal(enriched.review_text, 'Schnelle Lieferung.');
    assert.equal(enriched.created_at, '2026-05-31T19:13:43.000Z');
    assert.equal(enriched.verified, true);
    assert.equal(enriched.page_total_reviews, 100);
});

test('uses exact verification status matching', () => {
    assert.equal(enrichReview(
        { id: 'verified', verificationStatus: 'MEMBER_VERIFIED' },
        target,
        { page: 1, size: 10 },
        {},
    ).verified, true);

    assert.equal(enrichReview(
        { id: 'unverified', verificationStatus: 'UNVERIFIED' },
        target,
        { page: 1, size: 10 },
        {},
    ).verified, false);

    assert.equal(enrichReview(
        { id: 'unknown', verificationStatus: 'PENDING' },
        target,
        { page: 1, size: 10 },
        {},
    ).verified, null);
});

test('detects pagination stop conditions', () => {
    assert.equal(hasNextPage({ pagination: { has_next_page: false } }, 1), false);
    assert.equal(hasNextPage({ meta: { pagination: { hasNextPage: true } } }, 1), true);
    assert.equal(hasNextPage({ metaData: { totalPageCount: 2 } }, 1), true);
    assert.equal(hasNextPage({ metaData: { totalPageCount: 2 } }, 2), false);
    assert.equal(hasNextPage({}, 1), null);
});
