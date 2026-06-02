import assert from 'node:assert/strict';
import test from 'node:test';

const inputModule = process.env.TEST_SOURCE === 'src'
    ? '../src/input.ts'
    : '../dist/input.js';
const runnerModule = process.env.TEST_SOURCE === 'src'
    ? '../src/run-translations.ts'
    : '../dist/run-translations.js';
const sharedModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';
const { buildTranslationRequests } = await import(inputModule);
const { runTranslations } = await import(runnerModule);
const { ScrappaHttpError } = await import(sharedModule);

test('continues a batch after one translation item fails', async () => {
    const requests = buildTranslationRequests({
        items: [
            { text: 'Good morning', source: 'en', target: 'de' },
            { text: 'How are you?', source: 'en', target: 'es' },
            { text: 'Thank you', source: 'en', target: 'fr' },
        ],
    });
    const calls = [];
    const pushed = [];

    const summary = await runTranslations(
        requests,
        {
            async get(endpoint, params, options) {
                calls.push({ endpoint, params, options });
                if (params.text === 'How are you?') {
                    throw new ScrappaHttpError(503, 'Translation service temporarily unavailable. Please retry.');
                }

                return { translated_text: `${params.text} translated` };
            },
        },
        {
            async push(item) {
                pushed.push(item);
                return { saved: true, statusMessage: null };
            },
        },
        () => null,
    );

    assert.equal(calls.length, 3);
    assert.deepEqual(calls[0], {
        endpoint: '/google-translate',
        params: { text: 'Good morning', source: 'en', target: 'de' },
        options: { attempts: 2 },
    });
    assert.deepEqual(
        summary,
        {
            requested: 3,
            succeeded: 2,
            failed: 1,
            saved: 3,
            statusMessage: null,
            firstItem: pushed[0],
        },
    );
    assert.deepEqual(
        pushed.map((item) => ({ success: item.success, translated_text: item.translated_text, status_code: item.status_code })),
        [
            { success: true, translated_text: 'Good morning translated', status_code: null },
            { success: false, translated_text: null, status_code: 503 },
            { success: true, translated_text: 'Thank you translated', status_code: null },
        ],
    );
});

test('stops before fetching when charge limit status is returned', async () => {
    const requests = buildTranslationRequests({
        items: [
            { text: 'Good morning', source: 'en', target: 'de' },
            { text: 'How are you?', source: 'en', target: 'es' },
        ],
    });
    let calls = 0;

    const summary = await runTranslations(
        requests,
        {
            async get() {
                calls += 1;
                return { translated_text: 'unused' };
            },
        },
        {
            async push() {
                return { saved: true, statusMessage: null };
            },
        },
        () => 'Charge limit reached before fetching.',
    );

    assert.equal(calls, 0);
    assert.equal(summary.statusMessage, 'Charge limit reached before fetching.');
    assert.equal(summary.saved, 0);
});
