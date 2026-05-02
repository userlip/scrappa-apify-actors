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
            next_page_token: undefined,
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
        q: undefined,
        next_page_token: 'next-token',
        hl: undefined,
        gl: undefined,
        google_domain: undefined,
        uule: undefined,
        lrad: undefined,
        uds: undefined,
    });
});
