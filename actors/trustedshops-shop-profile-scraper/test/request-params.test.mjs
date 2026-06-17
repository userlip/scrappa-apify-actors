import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildTrustedShopsShopProfilePlan,
    extractTrustedShopsTsidFromUrl,
    normalizeTrustedShopsTsid,
} = await import(requestParamsModule);

const TSID = 'XFB15FFBDE1DEE7A55D292A7D48598A6A';

test('normalizes valid TSIDs to uppercase', () => {
    assert.equal(normalizeTrustedShopsTsid(TSID.toLowerCase()), TSID);
});

test('rejects malformed TSIDs', () => {
    assert.throws(
        () => normalizeTrustedShopsTsid('X123'),
        /33-character TrustedShops TSID/,
    );
});

test('extracts TSID from TrustedShops profile URL', () => {
    const result = extractTrustedShopsTsidFromUrl(
        `https://www.trustedshops.de/bewertung/info_${TSID}.html`,
    );

    assert.equal(result.tsid, TSID);
    assert.equal(result.source_url, `https://www.trustedshops.de/bewertung/info_${TSID}.html`);
});

test('builds a deduplicated batch plan from TSIDs and URLs', () => {
    const plan = buildTrustedShopsShopProfilePlan({
        tsids: [TSID, TSID.toLowerCase()],
        urls: [`https://www.trustedshops.de/bewertung/info_${TSID}.html`],
        include_raw_response: true,
    });

    assert.equal(plan.includeRawResponse, true);
    assert.deepEqual(plan.requests, [{ tsid: TSID }]);
});

test('keeps invalid URL inputs as per-input validation failures', () => {
    const plan = buildTrustedShopsShopProfilePlan({
        urls: ['https://www.trustedshops.de/bewertung/no-tsid.html'],
    });

    assert.equal(plan.requests.length, 1);
    assert.equal(plan.requests[0].tsid, undefined);
    assert.match(plan.requests[0].validation_error, /must contain a TrustedShops TSID/);
});

test('requires at least one TSID or URL', () => {
    assert.throws(
        () => buildTrustedShopsShopProfilePlan({}),
        /Provide tsids or urls/,
    );
});
