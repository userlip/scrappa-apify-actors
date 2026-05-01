import assert from 'node:assert/strict';
import test from 'node:test';
import { buildLinkedInPostParams } from '../dist/search-params.js';

const url = 'https://www.linkedin.com/posts/example-activity-123';

test('forwards cache age when it is at least one second', () => {
    const warnings = [];
    const params = buildLinkedInPostParams(
        { url, use_cache: true, maximum_cache_age: 1 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        url,
        use_cache: true,
        maximum_cache_age: 1,
    });
    assert.deepEqual(warnings, []);
});

test('omits use_cache when it is explicitly false', () => {
    const warnings = [];
    const params = buildLinkedInPostParams(
        { url, use_cache: false, maximum_cache_age: 1 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        url,
    });
    assert.deepEqual(warnings, []);
});

test('does not forward cache age when it is zero', () => {
    const warnings = [];
    const params = buildLinkedInPostParams(
        { url, use_cache: true, maximum_cache_age: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        url,
        use_cache: true,
    });
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /maximum_cache_age must be at least 1, got 0/);
});

test('does not forward cache age when it is negative', () => {
    const warnings = [];
    const params = buildLinkedInPostParams(
        { url, use_cache: true, maximum_cache_age: -1 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        url,
        use_cache: true,
    });
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /maximum_cache_age must be at least 1, got -1/);
});

test('does not forward cache age when it is not an integer number', () => {
    for (const maximum_cache_age of [1.5, '3600', null, Number.NaN]) {
        const warnings = [];
        const params = buildLinkedInPostParams(
            { url, use_cache: true, maximum_cache_age },
            (message) => warnings.push(message),
        );

        assert.deepEqual(params, {
            url,
            use_cache: true,
        });
        assert.equal(warnings.length, 1);
    }
});

test('omits cache age when it is undefined', () => {
    const warnings = [];
    const params = buildLinkedInPostParams(
        { url, use_cache: true },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        url,
        use_cache: true,
    });
    assert.deepEqual(warnings, []);
});
