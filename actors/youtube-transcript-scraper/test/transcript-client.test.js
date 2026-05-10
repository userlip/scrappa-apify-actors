import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildTranscriptRequest, fetchTranscript } from '../src/transcript-client.js';

describe('buildTranscriptRequest', () => {
    it('adds Scrappa API headers', () => {
        const { requestOptions } = buildTranscriptRequest({ id: 'dQw4w9WgXcQ' }, 'secret-key');

        assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
        assert.equal(requestOptions.headers.Accept, 'application/json');
    });
});

describe('fetchTranscript', () => {
    it('returns parsed JSON from Scrappa', async () => {
        const result = await fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
            apiKey: 'secret-key',
            fetchFn: async () => ({
                ok: true,
                json: async () => ({ videoId: 'dQw4w9WgXcQ', transcript: [] }),
            }),
        });

        assert.equal(result.data.videoId, 'dQw4w9WgXcQ');
    });

    it('throws on non-2xx responses', async () => {
        let calls = 0;

        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                fetchFn: async () => {
                    calls++;
                    return {
                        ok: false,
                        status: 401,
                        statusText: 'Unauthorized',
                    };
                },
            }),
            /401 Unauthorized/,
        );

        assert.equal(calls, 1);
    });

    it('retries transient Scrappa responses before returning parsed JSON', async () => {
        let calls = 0;
        const delays = [];

        const result = await fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
            apiKey: 'secret-key',
            retryBaseDelayMs: 5,
            randomFn: () => 0,
            sleepFn: async (ms) => delays.push(ms),
            fetchFn: async () => {
                calls++;

                if (calls < 3) {
                    return {
                        ok: false,
                        status: 522,
                        statusText: '',
                    };
                }

                return {
                    ok: true,
                    json: async () => ({ videoId: 'dQw4w9WgXcQ', transcript: [{ text: 'hello' }] }),
                };
            },
        });

        assert.equal(calls, 3);
        assert.deepEqual(delays, [5, 10]);
        assert.equal(result.data.transcript.length, 1);
    });

    it('fails after retry attempts are exhausted', async () => {
        let calls = 0;

        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                maxAttempts: 2,
                retryBaseDelayMs: 5,
                randomFn: () => 0,
                sleepFn: async () => {},
                fetchFn: async () => {
                    calls++;
                    return {
                        ok: false,
                        status: 522,
                        statusText: '',
                    };
                },
            }),
            /522 <none>/,
        );

        assert.equal(calls, 2);
    });

    it('retries transient network errors', async () => {
        let calls = 0;

        const result = await fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
            apiKey: 'secret-key',
            retryBaseDelayMs: 5,
            randomFn: () => 0,
            sleepFn: async () => {},
            fetchFn: async () => {
                calls++;

                if (calls === 1) {
                    throw new TypeError('fetch failed');
                }

                return {
                    ok: true,
                    json: async () => ({ videoId: 'dQw4w9WgXcQ', transcript: [] }),
                };
            },
        });

        assert.equal(calls, 2);
        assert.equal(result.data.videoId, 'dQw4w9WgXcQ');
    });

    it('passes the api URL and attempt number to the request callback', async () => {
        const requests = [];

        await fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
            apiKey: 'secret-key',
            retryBaseDelayMs: 5,
            randomFn: () => 0,
            sleepFn: async () => {},
            onRequest: (apiUrl, attempt) => requests.push({ apiUrl, attempt }),
            fetchFn: async () => {
                if (requests.length === 1) {
                    return {
                        ok: false,
                        status: 522,
                        statusText: '',
                    };
                }

                return {
                    ok: true,
                    json: async () => ({ videoId: 'dQw4w9WgXcQ', transcript: [] }),
                };
            },
        });

        assert.equal(requests.length, 2);
        assert.equal(requests[0].attempt, 1);
        assert.equal(requests[1].attempt, 2);
        assert.match(requests[0].apiUrl, /\/api\/youtube\/transcript\?video_id=dQw4w9WgXcQ/);
        assert.equal(requests[1].apiUrl, requests[0].apiUrl);
    });

    it('adds Scrappa context to exhausted network errors', async () => {
        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                maxAttempts: 1,
                fetchFn: async () => {
                    throw new TypeError('fetch failed');
                },
            }),
            /Scrappa API request failed before receiving a response: fetch failed/,
        );
    });

    it('adds jitter to retry delays', async () => {
        const delays = [];

        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                maxAttempts: 2,
                retryBaseDelayMs: 100,
                retryJitterRatio: 0.2,
                randomFn: () => 0.5,
                sleepFn: async (ms) => delays.push(ms),
                fetchFn: async () => ({
                    ok: false,
                    status: 522,
                    statusText: '',
                }),
            }),
            /522 <none>/,
        );

        assert.deepEqual(delays, [110]);
    });

    it('honors Retry-After delta seconds on retryable responses', async () => {
        let calls = 0;
        const delays = [];

        const result = await fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
            apiKey: 'secret-key',
            retryBaseDelayMs: 100,
            randomFn: () => 0,
            sleepFn: async (ms) => delays.push(ms),
            fetchFn: async () => {
                calls++;

                if (calls === 1) {
                    return {
                        ok: false,
                        status: 429,
                        statusText: 'Too Many Requests',
                        headers: new Headers({ 'retry-after': '3' }),
                    };
                }

                return {
                    ok: true,
                    json: async () => ({ videoId: 'dQw4w9WgXcQ', transcript: [] }),
                };
            },
        });

        assert.equal(result.data.videoId, 'dQw4w9WgXcQ');
        assert.deepEqual(delays, [3000]);
    });

    it('honors Retry-After HTTP dates on retryable responses', async () => {
        const delays = [];
        const nowMs = Date.UTC(2026, 4, 10, 12, 0, 0);
        const retryAt = new Date(nowMs + 3000).toUTCString();

        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                maxAttempts: 2,
                retryBaseDelayMs: 100,
                randomFn: () => 0,
                nowFn: () => nowMs,
                sleepFn: async (ms) => delays.push(ms),
                fetchFn: async () => ({
                    ok: false,
                    status: 503,
                    statusText: 'Service Unavailable',
                    headers: new Headers({ 'retry-after': retryAt }),
                }),
            }),
            /503 Service Unavailable/,
        );

        assert.deepEqual(delays, [3000]);
    });

    it('adds context to invalid JSON responses', async () => {
        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                fetchFn: async () => ({
                    ok: true,
                    json: async () => {
                        throw new SyntaxError('Unexpected token <');
                    },
                }),
            }),
            /Scrappa API returned an invalid JSON response.*Unexpected token </,
        );
    });
});
