import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildPageParams,
    buildTrustedShopsReviewsPlan,
    describeTrustedShopsReviewsRequest,
    extractTrustedShopsTsid,
} = await import(requestParamsModule);

const TSID = 'XFB15FFBDE1DEE7A55D292A7D48598A6A';

test('builds params for batch TrustedShops review targets', () => {
    const plan = buildTrustedShopsReviewsPlan({
        tsids: [TSID],
        urls: [`https://www.trustedshops.de/bewertung/info_${TSID}.html?utm=test`, TSID.toLowerCase()],
        page: 2,
        max_pages: 3,
        size: 50,
        market: 'deu',
        include_raw_responses: true,
    });

    assert.deepEqual(plan.targets, [
        {
            tsid: TSID,
            input: TSID,
            sourceUrl: `https://www.trustedshops.de/bewertung/info_${TSID}.html`,
        },
    ]);
    assert.deepEqual(plan.baseParams, {
        size: 50,
        market: 'DEU',
    });
    assert.equal(plan.startPage, 2);
    assert.equal(plan.maxPages, 3);
    assert.equal(plan.includeRawResponses, true);
    assert.deepEqual(buildPageParams(plan, 2), {
        size: 50,
        market: 'DEU',
        page: 2,
    });
    assert.equal(describeTrustedShopsReviewsRequest(plan), '1 Trusted Shops target(s) (pages 2-4)');
});

test('extracts TSIDs from direct values and profile URLs', () => {
    assert.equal(extractTrustedShopsTsid(TSID.toLowerCase()), TSID);
    assert.equal(extractTrustedShopsTsid(`www.trustedshops.de/bewertung/info_${TSID}.html`), TSID);
    assert.equal(extractTrustedShopsTsid(`https://example.com/api/trustedshops/reviews/${TSID}`), TSID);
});

test('supports single compatibility inputs', () => {
    const plan = buildTrustedShopsReviewsPlan({
        url: `https://www.trustedshops.eu/buyerrating/info_${TSID}.html`,
    });

    assert.deepEqual(plan.targets, [
        {
            tsid: TSID,
            input: `https://www.trustedshops.eu/buyerrating/info_${TSID}.html`,
            sourceUrl: `https://www.trustedshops.eu/buyerrating/info_${TSID}.html`,
        },
    ]);
    assert.deepEqual(plan.baseParams, { size: 20 });
    assert.equal(plan.startPage, 1);
    assert.equal(plan.maxPages, 1);
    assert.equal(plan.includeRawResponses, false);
});

test('rejects invalid TrustedShops request params', () => {
    assert.throws(
        () => buildTrustedShopsReviewsPlan({}),
        /Provide at least one Trusted Shops TSID or profile URL/,
    );

    assert.throws(
        () => buildTrustedShopsReviewsPlan({ tsids: ['not-a-tsid'] }),
        /must contain a 33-character Trusted Shops TSID/,
    );

    assert.throws(
        () => buildTrustedShopsReviewsPlan({ tsids: [TSID], max_pages: 26 }),
        /max_pages must be between 1 and 25/,
    );

    assert.throws(
        () => buildTrustedShopsReviewsPlan({ tsids: [TSID], page: 99, max_pages: 3 }),
        /page plus max_pages cannot exceed page 100/,
    );

    assert.throws(
        () => buildTrustedShopsReviewsPlan({ tsids: [TSID], market: 'USA' }),
        /market must be one of/,
    );

    assert.throws(
        () => buildTrustedShopsReviewsPlan({ tsids: [TSID], include_raw_responses: 'true' }),
        /include_raw_responses must be a boolean/,
    );
});
