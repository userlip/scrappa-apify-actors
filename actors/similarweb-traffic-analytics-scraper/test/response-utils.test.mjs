import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildSimilarwebDatasetItem,
    hasSimilarwebTrafficData,
} from '../dist/response-utils.js';

test('builds a normalized dataset item from a Similarweb response', () => {
    const item = buildSimilarwebDatasetItem({
        domain: 'google.com',
        site_name: 'google.com',
        title: 'Google',
        category: 'search_engines',
        global_rank: { Rank: 1 },
        country_rank: { Country: 840, CountryCode: 'US', Rank: '1' },
        category_rank: { Rank: '2', Category: 'Search' },
        engagement: {
            visits: '84172772881',
            time_on_site: '592.7',
            page_per_visit: '8.35',
            bounce_rate: '0.28',
            month: '12',
            year: '2025',
        },
        traffic_sources: {
            direct: 0.86,
            search: 0.07,
            social: 0.01,
            referrals: 0.04,
            mail: 0.002,
            paid_referrals: 0.003,
        },
        estimated_monthly_visits: {
            '2025-10-01': 85326206196,
            '2025-12-01': '84172772881',
        },
        top_countries: [{ country_code: 'US', share: 0.24 }],
        top_keywords: [{ keyword: 'gmail', volume: 114101130 }],
        screenshot: 'https://example.com/screenshot.png',
    }, {
        domain: 'google.com',
        inputDomain: 'https://www.google.com/search',
    });

    assert.equal(item.success, true);
    assert.equal(item.domain, 'google.com');
    assert.equal(item.global_rank_value, 1);
    assert.equal(item.country_rank_value, 1);
    assert.equal(item.category_rank_value, 2);
    assert.equal(item.visits, 84172772881);
    assert.equal(item.time_on_site, 592.7);
    assert.equal(item.traffic_direct, 0.86);
    assert.equal(item.latest_month, '2025-12-01');
    assert.equal(item.latest_month_visits, 84172772881);
    assert.deepEqual(item.result_counts, {
        top_countries: 1,
        top_keywords: 1,
        estimated_monthly_visit_months: 2,
    });
    assert.equal(item.input_domain, 'https://www.google.com/search');
});

test('detects whether a response has chargeable traffic data', () => {
    assert.equal(hasSimilarwebTrafficData({ global_rank: { Rank: 123 } }), true);
    assert.equal(hasSimilarwebTrafficData({ engagement: { visits: '1000' } }), true);
    assert.equal(hasSimilarwebTrafficData({ engagement: { visits: null } }), false);
});
