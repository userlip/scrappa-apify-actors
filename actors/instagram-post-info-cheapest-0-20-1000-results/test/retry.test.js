import assert from 'node:assert/strict';
import test from 'node:test';
import {
    DEFAULT_RETRY_DELAYS_MS,
    isCooldownAuthScrappaError,
    isRateLimitScrappaError,
    isTransientScrappaError,
    requestWithRetries,
} from '../src/retry.js';

test('uses extended cooldown delays for Instagram rate limits', () => {
    assert.deepEqual(DEFAULT_RETRY_DELAYS_MS, [30000, 90000, 180000, 300000, 600000, 900000]);
});

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

test('detects wrapped Scrappa rate limit messages', () => {
    assert.equal(isRateLimitScrappaError({
        response: {
            status: 500,
            data: { error: 'Rate limited (HTTP 429)' },
        },
    }), true);
});

test('detects cooldown authentication responses without treating them as generally transient', () => {
    const error = {
        response: {
            status: 401,
            data: { error: 'Authentication required (HTTP 401)' },
        },
    };

    assert.equal(isCooldownAuthScrappaError(error), true);
    assert.equal(isTransientScrappaError(error), false);
});

test('treats gateway and unavailable responses as transient', () => {
    for (const status of [502, 503, 504]) {
        assert.equal(isTransientScrappaError({
            response: {
                status,
                data: { message: 'Temporary upstream failure' },
            },
        }), true);
    }
});

test('treats timeout errors without response objects as transient', () => {
    assert.equal(isTransientScrappaError(new Error('Request timeout after 60000ms')), true);
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

test('requestWithRetries throws after exhausting transient retries', async () => {
    let calls = 0;
    const lastError = {
        response: {
            status: 503,
            data: { error: 'Temporarily unavailable' },
        },
    };

    await assert.rejects(
        requestWithRetries(async () => {
            calls++;
            throw lastError;
        }, {
            delaysMs: [1, 1],
            wait: async () => {},
        }),
        lastError,
    );

    assert.equal(calls, 3);
});

test('requestWithRetries does not retry non-transient failures', async () => {
    let calls = 0;
    const validationError = {
        response: {
            status: 400,
            data: { message: 'The URL must be a valid Instagram post URL.' },
        },
    };

    await assert.rejects(
        requestWithRetries(async () => {
            calls++;
            throw validationError;
        }, {
            delaysMs: [1, 1],
            wait: async () => {},
        }),
        validationError,
    );

    assert.equal(calls, 1);
});

test('requestWithRetries can retry a cooldown auth response after a rate limit', async () => {
    let calls = 0;
    let sawRateLimit = false;
    const delays = [];

    const result = await requestWithRetries(async () => {
        calls++;
        if (calls === 1) {
            throw {
                response: {
                    status: 500,
                    data: { error: 'Rate limited (HTTP 429)' },
                },
            };
        }

        if (calls === 2) {
            throw {
                response: {
                    status: 401,
                    data: { error: 'Authentication required (HTTP 401)' },
                },
            };
        }

        return { ok: true };
    }, {
        delaysMs: [1, 1],
        shouldRetry: (error) => {
            if (isTransientScrappaError(error)) {
                sawRateLimit = sawRateLimit || isRateLimitScrappaError(error);
                return true;
            }

            return sawRateLimit && isCooldownAuthScrappaError(error);
        },
        wait: async (delayMs) => {
            delays.push(delayMs);
        },
    });

    assert.deepEqual(result, { ok: true });
    assert.deepEqual(delays, [1, 1]);
    assert.equal(calls, 3);
});

test('requestWithRetries does not retry a direct authentication failure', async () => {
    let calls = 0;
    const authError = {
        response: {
            status: 401,
            data: { error: 'Authentication required (HTTP 401)' },
        },
    };

    await assert.rejects(
        requestWithRetries(async () => {
            calls++;
            throw authError;
        }, {
            delaysMs: [1, 1],
            shouldRetry: (error) => {
                if (isTransientScrappaError(error)) {
                    return true;
                }

                return false;
            },
            wait: async () => {},
        }),
        authError,
    );

    assert.equal(calls, 1);
});

test('requestWithRetries with empty delays runs once without retrying', async () => {
    let calls = 0;
    const transientError = {
        response: {
            status: 503,
            data: { error: 'Temporarily unavailable' },
        },
    };

    await assert.rejects(
        requestWithRetries(async () => {
            calls++;
            throw transientError;
        }, {
            delaysMs: [],
            wait: async () => {},
        }),
        transientError,
    );

    assert.equal(calls, 1);
});
