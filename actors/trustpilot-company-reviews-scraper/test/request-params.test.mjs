import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildPageParams,
    buildTrustpilotCompanyReviewsPlan,
    describeTrustpilotCompanyReviewsRequest,
} from '../dist/request-params.js';

test('builds params for a basic reviews request', () => {
    const plan = buildTrustpilotCompanyReviewsPlan({
        company_domain: ' https://www.Amazon.com/review-path ',
        locale: 'en-US',
        page: 2,
        max_pages: 3,
        per_page: 20,
        sort: 'recency',
    });

    assert.deepEqual(plan, {
        baseParams: {
            company_domain: 'amazon.com',
            locale: 'en-US',
            per_page: 20,
            sort: 'recency',
        },
        startPage: 2,
        maxPages: 3,
    });

    assert.deepEqual(buildPageParams(plan, 3), {
        company_domain: 'amazon.com',
        locale: 'en-US',
        per_page: 20,
        sort: 'recency',
        page: 3,
    });
});

test('builds filter params', () => {
    const plan = buildTrustpilotCompanyReviewsPlan({
        company_domain: 'example.com',
        rating: ' 1, 2 ',
        verified: true,
        with_replies: false,
        query: ' refund ',
        date_posted: 'last_30_days',
        fields: 'reviews,businessUnit.displayName',
    });

    assert.deepEqual(plan.baseParams, {
        company_domain: 'example.com',
        rating: '1,2',
        verified: 1,
        with_replies: 0,
        query: 'refund',
        date_posted: 'last_30_days',
        fields: 'reviews,businessUnit.displayName',
    });
});

test('defaults pagination controls', () => {
    const plan = buildTrustpilotCompanyReviewsPlan({ company_domain: 'example.com' });

    assert.equal(plan.startPage, 1);
    assert.equal(plan.maxPages, 1);
    assert.equal(describeTrustpilotCompanyReviewsRequest(plan), 'example.com (page 1)');
});

test('rejects invalid domains and pagination beyond page 10', () => {
    assert.throws(
        () => buildTrustpilotCompanyReviewsPlan({ company_domain: 'invalid' }),
        /company_domain must be a valid domain name/,
    );

    assert.throws(
        () => buildTrustpilotCompanyReviewsPlan({ company_domain: 'example.com', page: 8, max_pages: 4 }),
        /page plus max_pages cannot request beyond page 10/,
    );
});

test('rejects invalid filters', () => {
    assert.throws(
        () => buildTrustpilotCompanyReviewsPlan({ company_domain: 'example.com', rating: '0,6' }),
        /rating must be a comma-separated list/,
    );

    assert.throws(
        () => buildTrustpilotCompanyReviewsPlan({ company_domain: 'example.com', locale: 'en' }),
        /locale must be one of/,
    );

    assert.throws(
        () => buildTrustpilotCompanyReviewsPlan({ company_domain: 'example.com', verified: 'true' }),
        /verified must be a boolean/,
    );
});
