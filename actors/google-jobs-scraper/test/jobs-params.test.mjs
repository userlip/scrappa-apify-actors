import assert from 'node:assert/strict';
import test from 'node:test';

import { buildJobsParams } from '../dist/jobs-params.js';

test('forwards Google Jobs search parameters', () => {
    assert.deepEqual(
        buildJobsParams({
            q: 'software engineer',
            hl: 'en',
            gl: 'us',
            google_domain: 'google.com',
            uule: 'w+CAIQICIWTmV3IFlvcms',
            lrad: 25,
            uds: 'filter-token',
        }),
        {
            q: 'software engineer',
            hl: 'en',
            gl: 'us',
            google_domain: 'google.com',
            uule: 'w+CAIQICIWTmV3IFlvcms',
            lrad: 25,
            uds: 'filter-token',
        }
    );
});

test('allows next page token without query', () => {
    assert.deepEqual(buildJobsParams({ next_page_token: 'next-token' }), {
        next_page_token: 'next-token',
    });
});
