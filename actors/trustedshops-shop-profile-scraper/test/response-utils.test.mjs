import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildTrustedShopsShopProfileDatasetItem,
    buildTrustedShopsShopProfileOutputSummary,
    hasTrustedShopsShopProfileData,
} = await import(responseUtilsModule);

const TSID = 'XFB15FFBDE1DEE7A55D292A7D48598A6A';

test('flattens stable TrustedShops shop profile fields', () => {
    const item = buildTrustedShopsShopProfileDatasetItem({
        response: {
            data: {
                shop: {
                    tsId: TSID,
                    name: 'Example Shop',
                    url: 'example-shop.de',
                    profileUrl: `/bewertung/info_${TSID}.html`,
                    languageISO2: 'de',
                    targetMarketISO3: 'DEU',
                    averageRating: 4.8,
                    reviewCount: 1234,
                    certificationState: true,
                    shopCategories: [
                        { id: 1, name: 'Fashion' },
                        { id: 2, name: 'Shoes' },
                    ],
                },
            },
            responseInfo: { apiVersion: '2.4.18' },
        },
    }, {
        requestedTsid: TSID,
        sourceUrl: `https://www.trustedshops.de/bewertung/info_${TSID}.html`,
        includeRawResponse: false,
    });

    assert.equal(item.tsid, TSID);
    assert.equal(item.name, 'Example Shop');
    assert.equal(item.url, 'https://example-shop.de');
    assert.equal(item.profile_url, `https://www.trustedshops.de/bewertung/info_${TSID}.html`);
    assert.equal(item.language, 'de');
    assert.equal(item.target_market, 'DEU');
    assert.equal(item.rating, 4.8);
    assert.equal(item.review_count, 1234);
    assert.equal(item.certified, true);
    assert.equal(item.category_names, 'Fashion, Shoes');
    assert.equal(item.category_ids, '1, 2');
    assert.deepEqual(item.profile_metadata, { apiVersion: '2.4.18' });
    assert.equal('raw_response' in item, false);
});

test('falls back to canonical TrustedShops profile URL and can include raw response', () => {
    const response = {
        tsid: TSID,
        name: 'Fallback Shop',
        reviewSummary: { rating: 4.5, totalReviewCount: 99 },
    };
    const item = buildTrustedShopsShopProfileDatasetItem(response, {
        requestedTsid: TSID,
        includeRawResponse: true,
    });

    assert.equal(item.profile_url, `https://www.trustedshops.de/bewertung/info_${TSID}.html`);
    assert.equal(item.rating, 4.5);
    assert.equal(item.review_count, 99);
    assert.deepEqual(item.raw_response, response);
});

test('skips empty objects and falls back to populated profile objects', () => {
    const item = buildTrustedShopsShopProfileDatasetItem({
        response: {
            data: {
                shop: {},
            },
            responseInfo: {},
        },
        shop: {
            tsId: TSID,
            name: 'Populated Shop',
            targetMarketISO3: 'DEU',
            metadata: { source: 'fallback' },
        },
        metaData: { source: 'metadata fallback' },
    }, {
        requestedTsid: TSID,
        includeRawResponse: false,
    });

    assert.equal(item.name, 'Populated Shop');
    assert.equal(item.target_market, 'DEU');
    assert.deepEqual(item.profile_metadata, { source: 'fallback' });
});

test('falls back to shopCategories when categories is empty', () => {
    const item = buildTrustedShopsShopProfileDatasetItem({
        shop: {
            tsId: TSID,
            name: 'Category Shop',
            categories: [],
            shopCategories: [{ id: 7, name: 'Electronics' }],
        },
    }, {
        requestedTsid: TSID,
        includeRawResponse: false,
    });

    assert.equal(item.category_names, 'Electronics');
    assert.equal(item.category_ids, '7');
});

test('reports missing shop profile payloads as invalid before charging', () => {
    assert.equal(hasTrustedShopsShopProfileData({ response: { data: { shop: {} } } }), false);
    assert.equal(hasTrustedShopsShopProfileData({ response: { data: { shop: { tsId: TSID } } } }), false);
    assert.throws(
        () => buildTrustedShopsShopProfileDatasetItem({ response: { data: { shop: { tsId: TSID } } } }, {
            requestedTsid: TSID,
            includeRawResponse: false,
        }),
        /did not include a TrustedShops shop profile object/,
    );
});

test('builds output summary with failures', () => {
    const output = buildTrustedShopsShopProfileOutputSummary({
        requested: 2,
        savedProfiles: 1,
        failures: [{ tsid: 'bad', error: 'not found' }],
        statusMessage: '1 failed',
    });

    assert.equal(output.profiles_requested, 2);
    assert.equal(output.profiles_saved, 1);
    assert.equal(output.profiles_failed, 1);
    assert.equal(output.status_message, '1 failed');
});
