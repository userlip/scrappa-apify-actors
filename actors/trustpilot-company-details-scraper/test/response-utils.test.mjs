import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildTrustpilotCompanyDetailsDatasetItem,
} = await import(responseUtilsModule);

test('builds normalized Trustpilot company details dataset item', () => {
    const item = buildTrustpilotCompanyDetailsDatasetItem(
        {
            success: true,
            basic_info: {
                name: 'Trustpilot',
                domain: 'trustpilot.com',
                business_unit_id: 'abc123',
                website: 'www.trustpilot.com',
                profile_url: '/review/trustpilot.com',
                logo_url: '//example.com/logo.png',
                is_claimed: true,
                is_verified: false,
            },
            ratings: {
                trustscore: 4.4,
                stars: 4.5,
                count: 501587,
            },
            location: {
                country: 'United States',
                country_code: 'US',
                city: 'New York',
                address: 'New York, NY',
            },
            categories: [
                { displayName: 'Review Site', slug: 'review-site' },
                { name: 'Software Company', id: 'software-company' },
            ],
            contact: {
                email: 'support@trustpilot.com',
                phone: '+1 555 0100',
            },
            social_media: {
                linkedin: 'https://www.linkedin.com/company/trustpilot',
            },
            metadata: {
                source: 'trustpilot',
                scraped_at: '2026-06-06T05:03:22.364719Z',
            },
        },
        {
            companyDomain: 'trustpilot.com',
            params: {
                locale: 'en-US',
            },
        },
    );

    assert.equal(item.company_domain, 'trustpilot.com');
    assert.equal(item.requested_company_domain, 'trustpilot.com');
    assert.equal(item.company_name, 'Trustpilot');
    assert.equal(item.business_unit_id, 'abc123');
    assert.equal(item.website_url, 'https://www.trustpilot.com');
    assert.equal(item.profile_url, 'https://www.trustpilot.com/review/trustpilot.com');
    assert.equal(item.logo_url, 'https://example.com/logo.png');
    assert.equal(item.trust_score, 4.4);
    assert.equal(item.stars, 4.5);
    assert.equal(item.review_count, 501587);
    assert.equal(item.is_claimed, true);
    assert.equal(item.is_verified, false);
    assert.equal(item.country, 'United States');
    assert.equal(item.country_code, 'US');
    assert.equal(item.city, 'New York');
    assert.equal(item.email, 'support@trustpilot.com');
    assert.equal(item.phone, '+1 555 0100');
    assert.equal(item.category_names, 'Review Site, Software Company');
    assert.equal(item.category_slugs, 'review-site, software-company');
    assert.deepEqual(item.social_media, {
        linkedin: 'https://www.linkedin.com/company/trustpilot',
    });
    assert.equal(item.request_locale, 'en-US');
    assert.equal(item.response_source, 'trustpilot');
    assert.equal(item.scraped_at, '2026-06-06T05:03:22.364719Z');
});

test('tolerates sparse company details responses', () => {
    const item = buildTrustpilotCompanyDetailsDatasetItem(
        {
            ratings: {
                trust_score: 3.2,
                review_count: 10,
            },
        },
        {
            companyDomain: 'example.com',
            params: {},
        },
    );

    assert.equal(item.company_domain, 'example.com');
    assert.equal(item.company_name, null);
    assert.equal(item.profile_url, 'https://www.trustpilot.com/review/example.com');
    assert.equal(item.website_url, 'https://example.com');
    assert.equal(item.trust_score, 3.2);
    assert.equal(item.review_count, 10);
    assert.equal(item.category_names, '');
    assert.equal(item.category_slugs, '');
    assert.equal(item.social_media, null);
});

test('unwraps live Scrappa company details response shape', () => {
    const item = buildTrustpilotCompanyDetailsDatasetItem(
        {
            success: true,
            message: 'Company details retrieved successfully',
            data: {
                basic_info: {
                    name: 'Trustpilot',
                    domain: 'trustpilot.com',
                    logo: null,
                    website: null,
                    claimed: true,
                    verified: false,
                    profileUrl: 'https://www.trustpilot.com/review/trustpilot.com',
                },
                ratings: {
                    trustscore: 4.4,
                    stars: 4.5,
                    count: 503425,
                },
                location: {
                    country: null,
                    city: null,
                    address: null,
                },
                categories: [
                    { id: 'media_company', name: 'Media Company' },
                    { id: 'review_site', name: 'Review Site' },
                ],
                contact: {
                    email: null,
                    phone: null,
                },
                social_media: [],
            },
            meta: {
                scraped_at: '2026-06-10T19:52:39.578820Z',
                source: 'aws_waf_replay_primary',
            },
        },
        {
            companyDomain: 'trustpilot.com',
            params: {
                locale: 'en-US',
            },
        },
    );

    assert.equal(item.company_name, 'Trustpilot');
    assert.equal(item.company_domain, 'trustpilot.com');
    assert.equal(item.trust_score, 4.4);
    assert.equal(item.stars, 4.5);
    assert.equal(item.review_count, 503425);
    assert.equal(item.is_claimed, true);
    assert.equal(item.is_verified, false);
    assert.equal(item.category_names, 'Media Company, Review Site');
    assert.equal(item.category_slugs, 'media_company, review_site');
    assert.deepEqual(item.social_media, []);
    assert.equal(item.response_source, 'aws_waf_replay_primary');
    assert.equal(item.scraped_at, '2026-06-10T19:52:39.578820Z');
});

test('normalizes protocol-relative profile URLs and ignores empty response domains', () => {
    const item = buildTrustpilotCompanyDetailsDatasetItem(
        {
            basic_info: {
                domain: '',
                profile_url: '//www.trustpilot.com/review/example.com',
                website: '',
            },
        },
        {
            companyDomain: 'example.com',
            params: {},
        },
    );

    assert.equal(item.company_domain, 'example.com');
    assert.equal(item.profile_url, 'https://www.trustpilot.com/review/example.com');
    assert.equal(item.website_url, 'https://example.com');
});

test('filters malformed category entries before building aliases', () => {
    const item = buildTrustpilotCompanyDetailsDatasetItem(
        {
            categories: [
                null,
                'invalid',
                { name: 'Review Site', id: 'review_site' },
                42,
                { displayName: 'Media Company', slug: 'media-company' },
            ],
        },
        {
            companyDomain: 'example.com',
            params: {},
        },
    );

    assert.equal(item.category_names, 'Review Site, Media Company');
    assert.equal(item.category_slugs, 'review_site, media-company');
});
