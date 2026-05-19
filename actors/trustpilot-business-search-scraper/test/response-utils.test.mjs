import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildTrustpilotBusinessDatasetItem,
    getTrustpilotBusinesses,
    hasNextPage,
} = await import(responseUtilsModule);

test('extracts businesses from company search and category response shapes', () => {
    assert.deepEqual(getTrustpilotBusinesses({ businessUnits: [{ displayName: 'Company A' }] }), [{ displayName: 'Company A' }]);
    assert.deepEqual(
        getTrustpilotBusinesses({
            pageProps: {
                businessUnits: {
                    businesses: [{ displayName: 'Company B' }],
                },
            },
        }),
        [{ displayName: 'Company B' }],
    );
    assert.deepEqual(getTrustpilotBusinesses({}), []);
});

test('detects next pages across response shapes', () => {
    assert.equal(hasNextPage({ pagination: { totalPages: 3 } }, 1), true);
    assert.equal(hasNextPage({ pagination: { totalPages: 3 } }, 3), false);
    assert.equal(hasNextPage({ pageProps: { pagination: { has_next_page: true } } }, 1), true);
    assert.equal(hasNextPage({ pageProps: { pagination: { has_next_page: false, total_pages: 10 } } }, 1), false);
    assert.equal(hasNextPage({ pageProps: { pagination: { total_pages: 3 } } }, 1), true);
    assert.equal(hasNextPage({ pageProps: { pagination: { total_pages: 3 } } }, 3), false);
    assert.equal(hasNextPage({ pageProps: { businessUnits: { totalPages: 2 } } }, 2), false);
});

test('builds normalized Trustpilot company-search dataset item', () => {
    const item = buildTrustpilotBusinessDatasetItem(
        {
            id: 'abc123',
            displayName: 'Amazon',
            identifyingName: 'amazon.com',
            websiteUrl: 'www.amazon.com',
            score: { trustScore: 1.4, stars: 1.5 },
            numberOfReviews: 38765,
            logoUrl: '//example.com/logo.png',
            countryCode: 'US',
            location: { country: 'United States', city: 'Seattle' },
            address: { countryCode: 'US', city: 'Seattle' },
            isClaimed: true,
            verified: false,
            categories: [
                { displayName: 'Marketplace', slug: 'marketplace' },
                { name: 'Electronics', id: 'electronics' },
            ],
        },
        {
            searchType: 'company_search',
            params: {
                query: 'amazon',
                page: 1,
                country: 'US',
                min_rating: 1,
            },
            response: {
                pagination: {
                    totalResults: 42,
                    totalPages: 3,
                    pageSize: 20,
                },
                searchMode: 'keyword',
                meta: {
                    source: 'rapidapi',
                    scraped_at: '2026-05-19T00:00:00Z',
                },
            },
        },
    );

    assert.equal(item.business_id, 'abc123');
    assert.equal(item.business_name, 'Amazon');
    assert.equal(item.website_url, 'https://www.amazon.com');
    assert.equal(item.profile_url, 'https://www.trustpilot.com/review/amazon.com');
    assert.equal(item.trust_score, 1.4);
    assert.equal(item.stars, 1.5);
    assert.equal(item.review_count, 38765);
    assert.equal(item.logo_url, 'https://example.com/logo.png');
    assert.equal(item.is_verified, false);
    assert.equal(item.category_names, 'Marketplace, Electronics');
    assert.equal(item.category_slugs, 'marketplace, electronics');
    assert.equal(item.request_search_type, 'company_search');
    assert.equal(item.request_query, 'amazon');
    assert.equal(item.total_results, 42);
    assert.equal(item.total_pages, 3);
    assert.equal(item.per_page, 20);
});

test('builds normalized Trustpilot category dataset item', () => {
    const item = buildTrustpilotBusinessDatasetItem(
        {
            businessUnitId: 'def456',
            name: 'Example Store',
            identifyingName: 'example.com',
            contact: {
                website: 'https://example.com',
                email: 'support@example.com',
                phone: '+1 555 0100',
            },
            trustScore: 4.7,
            stars: 5,
            totalNumberOfReviews: 1234,
            isBusinessClaimed: true,
        },
        {
            searchType: 'category',
            params: {
                category: 'electronics_technology',
                page: 2,
                country: 'US',
            },
            response: {
                pageProps: {
                    businessUnits: {
                        totalHits: 80,
                        totalPages: 4,
                    },
                    categoryDisplayName: 'Electronics & Technology',
                },
            },
        },
    );

    assert.equal(item.business_id, 'def456');
    assert.equal(item.business_name, 'Example Store');
    assert.equal(item.email, 'support@example.com');
    assert.equal(item.phone, '+1 555 0100');
    assert.equal(item.request_search_type, 'category');
    assert.equal(item.request_category, 'electronics_technology');
    assert.equal(item.category_display_name, 'Electronics & Technology');
    assert.equal(item.total_results, 80);
    assert.equal(item.total_pages, 4);
});
