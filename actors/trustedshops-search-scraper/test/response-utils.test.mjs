import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildTrustedShopsDatasetItem,
    getTrustedShops,
} = await import(responseUtilsModule);

test('extracts shops from primary and wrapped response shapes', () => {
    assert.deepEqual(getTrustedShops({ shops: [{ shopName: 'shop-a.de' }] }), [{ shopName: 'shop-a.de' }]);
    assert.deepEqual(getTrustedShops({ data: { shops: [{ shopName: 'shop-b.de' }] } }), [{ shopName: 'shop-b.de' }]);
    assert.deepEqual(getTrustedShops({}), []);
});

test('builds normalized Trusted Shops dataset item', () => {
    const item = buildTrustedShopsDatasetItem(
        {
            profileType: 'member',
            accountName: 'Example Shop GmbH',
            tsID: 'XFB15FFBDE1DEE7A55D292A7D48598A6A',
            shopDescription: 'Online shop.',
            shopName: 'example-shop.de',
            shopUrl: 'www.example-shop.de',
            shopCategories: [
                { name: 'Fashion', id: 23, urlPath: 'fashion' },
                { name: 'Shoes', id: 24, urlPath: 'shoes' },
            ],
            shopLogoUrl: 'https://example.com/logo.png',
            averageRating: 4.8,
            reviewCount: 12000,
            certificationState: true,
            profileUrl: 'www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html',
            contractStartDate: 1610582400000,
        },
        {
            q: 'zalando',
            market: 'DEU',
            page: 0,
        },
        {
            metaData: {
                totalShopCount: 66,
                totalPageCount: 4,
            },
        },
    );

    assert.equal(item.tsID, 'XFB15FFBDE1DEE7A55D292A7D48598A6A');
    assert.equal(item.shop_url, 'https://www.example-shop.de');
    assert.equal(item.profile_url, 'https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html');
    assert.equal(item.category_names, 'Fashion, Shoes');
    assert.equal(item.category_ids, '23, 24');
    assert.equal(item.category_url_paths, 'fashion, shoes');
    assert.equal(item.request_q, 'zalando');
    assert.equal(item.request_market, 'DEU');
    assert.equal(item.total_shop_count, 66);
    assert.equal(item.total_page_count, 4);
});
