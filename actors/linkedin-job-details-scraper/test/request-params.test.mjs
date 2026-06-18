import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.ACTOR_TEST_TARGET === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const { buildLinkedInJobDetailsParams } = await import(requestParamsModule);

test('includes cache age only when cache is enabled and age is at least 1', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsParams({
            url: 'https://www.linkedin.com/jobs/view/1234567890',
            use_cache: true,
            maximum_cache_age: 1,
        }),
        {
            url: 'https://www.linkedin.com/jobs/view/1234567890',
            use_cache: 1,
            maximum_cache_age: 1,
        },
    );
});

test('omits zero and invalid cache ages to avoid Scrappa validation failures', () => {
    for (const maximum_cache_age of [0, -1, 1.5, '3600', null, Number.NaN]) {
        assert.deepEqual(
            buildLinkedInJobDetailsParams({
                url: 'https://www.linkedin.com/jobs/view/1234567890',
                use_cache: true,
                maximum_cache_age,
            }),
            {
                url: 'https://www.linkedin.com/jobs/view/1234567890',
                use_cache: 1,
            },
        );
    }
});

test('omits cache controls when cache is disabled', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsParams({
            url: 'https://www.linkedin.com/jobs/view/1234567890',
            use_cache: false,
            maximum_cache_age: 3600,
        }),
        {
            url: 'https://www.linkedin.com/jobs/view/1234567890',
        },
    );
});
