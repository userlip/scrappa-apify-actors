import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildKununuJobsSearchPlan,
    describeKununuJobsRequest,
} from '../dist/kununu-jobs-params.js';

test('builds default Kununu jobs search parameters', () => {
    assert.deepEqual(buildKununuJobsSearchPlan(undefined), {
        params: {
            query: 'Software Engineer',
            location: 'Berlin',
            country: 'de',
            page: 1,
        },
        startPage: 1,
        maxPages: 1,
        includeRawJob: false,
    });
});

test('normalizes and forwards Kununu jobs search filters', () => {
    assert.deepEqual(buildKununuJobsSearchPlan({
        query: '  Product Manager ',
        location: ' Munich ',
        country: ' DE ',
        page: '2',
        max_pages: '3',
        radius: 100,
        sort: 'kununuScore',
        workplace: ['full_remote', 'PARTLY_REMOTE', 'FULL_REMOTE'],
        employment_types: ['full_time', 'part_time'],
        career_level: [3, '4'],
        kununu_score: ['4-5', '3-4'],
        industry: [12, '13'],
        discipline: ['1001', 1002],
        benefits: ['flexWorkingHours', 'pensionPlan'],
        is_top_company: true,
        include_raw_job: true,
    }), {
        params: {
            query: 'Product Manager',
            location: 'Munich',
            country: 'de',
            page: 2,
            radius: 100,
            sort: 'kununuScore',
            workplace: ['FULL_REMOTE', 'PARTLY_REMOTE'],
            employment_types: ['FULL_TIME', 'PART_TIME'],
            career_level: ['3', '4'],
            kununu_score: ['4-5', '3-4'],
            industry: [12, 13],
            discipline: [1001, 1002],
            benefits: ['flexWorkingHours', 'pensionPlan'],
            is_top_company: true,
        },
        startPage: 2,
        maxPages: 3,
        includeRawJob: true,
    });
});

test('treats whitespace-only integer inputs as omitted', () => {
    const plan = buildKununuJobsSearchPlan({ page: '   ', max_pages: ' ', radius: '' });

    assert.equal(plan.startPage, 1);
    assert.equal(plan.maxPages, 1);
    assert.equal(plan.params.radius, undefined);
});

test('keeps false Top Company filter explicit', () => {
    assert.equal(buildKununuJobsSearchPlan({ is_top_company: false }).params.is_top_company, false);
});

test('rejects unsupported enum and range values', () => {
    assert.throws(() => buildKununuJobsSearchPlan({ country: 'us' }), /country must be one of/);
    assert.throws(() => buildKununuJobsSearchPlan({ radius: 25 }), /radius must be one of/);
    assert.throws(() => buildKununuJobsSearchPlan({ max_pages: 11 }), /max_pages must be between 1 and 10/);
    assert.throws(() => buildKununuJobsSearchPlan({ workplace: ['REMOTE'] }), /workplace must be one of/);
    assert.throws(() => buildKununuJobsSearchPlan({ industry: [45] }), /industry must be between 1 and 44/);
    assert.throws(() => buildKununuJobsSearchPlan({ discipline: [999] }), /discipline must be between 1001 and 1022/);
    assert.throws(() => buildKununuJobsSearchPlan({ is_top_company: 'true' }), /is_top_company must be a boolean/);
});

test('describes Kununu jobs requests for logs', () => {
    assert.equal(
        describeKununuJobsRequest({ query: 'Data Analyst', location: 'Vienna', country: 'at' }),
        '"Data Analyst" in Vienna, AT'
    );
});
