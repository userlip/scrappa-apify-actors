import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildCompanyDetailsParams,
    buildTrustpilotCompanyDetailsPlan,
    describeTrustpilotCompanyDetailsRequest,
} = await import(requestParamsModule);

test('builds params for a single company details request', () => {
    const plan = buildTrustpilotCompanyDetailsPlan({
        company_domain: ' https://www.Trustpilot.com/review-path ',
        locale: 'en-US',
    });

    assert.deepEqual(plan, {
        domains: ['trustpilot.com'],
        baseParams: {
            locale: 'en-US',
        },
    });
    assert.deepEqual(buildCompanyDetailsParams(plan, 'trustpilot.com'), {
        company_domain: 'trustpilot.com',
        locale: 'en-US',
    });
    assert.equal(describeTrustpilotCompanyDetailsRequest(plan), 'trustpilot.com');
});

test('accepts and deduplicates batch company domains', () => {
    const plan = buildTrustpilotCompanyDetailsPlan({
        company_domain: 'www.trustpilot.com',
        company_domains: [
            'https://www.amazon.com/reviews',
            'amazon.com',
            'example.co.uk?ref=trustpilot',
        ],
    });

    assert.deepEqual(plan.domains, ['trustpilot.com', 'amazon.com', 'example.co.uk']);
    assert.equal(describeTrustpilotCompanyDetailsRequest(plan), '3 company domains');
});

test('accepts comma and newline separated batch company domains', () => {
    const plan = buildTrustpilotCompanyDetailsPlan({
        company_domains: 'trustpilot.com, https://www.amazon.com/reviews\nexample.com',
    });

    assert.deepEqual(plan.domains, ['trustpilot.com', 'amazon.com', 'example.com']);
});

test('defaults locale and passes optional fields', () => {
    const plan = buildTrustpilotCompanyDetailsPlan({
        company_domain: 'trustpilot.com',
        fields: 'basic_info,ratings,metadata',
    });

    assert.deepEqual(plan.baseParams, {
        locale: 'en-US',
        fields: 'basic_info,ratings,metadata',
    });
});

test('rejects invalid input', () => {
    assert.throws(
        () => buildTrustpilotCompanyDetailsPlan({ company_domain: 'invalid' }),
        /company_domains must be a valid domain name/,
    );

    assert.throws(
        () => buildTrustpilotCompanyDetailsPlan({ company_domains: [123] }),
        /company_domains must be a string/,
    );

    assert.throws(
        () => buildTrustpilotCompanyDetailsPlan({}),
        /Provide company_domain or company_domains/,
    );

    assert.throws(
        () => buildTrustpilotCompanyDetailsPlan({ company_domain: 'trustpilot.com', locale: 'en' }),
        /locale must be one of/,
    );

    assert.throws(
        () => buildTrustpilotCompanyDetailsPlan({
            company_domains: Array.from({ length: 101 }, (_, index) => `example${index}.com`),
        }),
        /at most 100 domains/,
    );
});
