import assert from 'node:assert/strict';
import test from 'node:test';
import { isTransientScrappaError, requestWithRetries } from '../src/retry.js';

test('treats wrapped Scrappa rate limit responses as transient', () => {
    assert.equal(isTransientScrappaError({
        response: {
            status: 500,
            data: { error: 'Rate limited (HTTP 429)' },
        },
    }), true);
});

test('treats HTTP 429 responses as transient', () => {
    assert.equal(isTransientScrappaError({
        response: {
            status: 429,
            data: { message: 'Too many requests' },
        },
    }), true);
});

test('does not retry validation failures', () => {
    assert.equal(isTransientScrappaError({
        response: {
            status: 400,
            data: { message: 'The URL must be a valid Instagram post URL.' },
        },
    }), false);
});

test('requestWithRetries retries transient failures and returns the first success', async () => {
    let calls = 0;
    const result = await requestWithRetries(async () => {
        calls++;
        if (calls < 2) {
            throw {
                response: {
                    status: 500,
                    data: { error: 'Rate limited (HTTP 429)' },
                },
            };
        }

        return { ok: true };
    }, {
        delaysMs: [1],
        wait: async () => {},
    });

    assert.deepEqual(result, { ok: true });
    assert.equal(calls, 2);
});
