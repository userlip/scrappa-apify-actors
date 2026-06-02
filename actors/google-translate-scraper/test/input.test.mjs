import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const inputModule = process.env.TEST_SOURCE === 'src'
    ? '../src/input.ts'
    : '../dist/input.js';
const {
    buildTranslationRequests,
    describeTranslationRequests,
} = await import(inputModule);

test('builds a single translation request from top-level fields', () => {
    assert.deepEqual(
        buildTranslationRequests({ text: ' Good morning ', source: 'EN', target: 'de' }),
        [{
            index: 0,
            text: 'Good morning',
            source: 'en',
            target: 'de',
            params: {
                text: 'Good morning',
                source: 'en',
                target: 'de',
            },
        }],
    );
});

test('builds batch translation requests and prefers items over top-level fields', () => {
    assert.deepEqual(
        buildTranslationRequests({
            text: 'Ignored',
            source: 'en',
            target: 'de',
            items: [
                { text: 'Hello', source: 'en', target: 'es' },
                { text: 'Welcome', source: 'pt-br', target: 'ZH-cn' },
                { text: 'Script code', source: 'MS-arab', target: 'MNI-mtei' },
                { text: 'Latin America', source: 'en', target: 'ES-419' },
            ],
        }),
        [
            {
                index: 0,
                text: 'Hello',
                source: 'en',
                target: 'es',
                params: { text: 'Hello', source: 'en', target: 'es' },
            },
            {
                index: 1,
                text: 'Welcome',
                source: 'pt-BR',
                target: 'zh-CN',
                params: { text: 'Welcome', source: 'pt-BR', target: 'zh-CN' },
            },
            {
                index: 2,
                text: 'Script code',
                source: 'ms-Arab',
                target: 'mni-Mtei',
                params: { text: 'Script code', source: 'ms-Arab', target: 'mni-Mtei' },
            },
            {
                index: 3,
                text: 'Latin America',
                source: 'en',
                target: 'es-419',
                params: { text: 'Latin America', source: 'en', target: 'es-419' },
            },
        ],
    );
});

test('validates translation input', () => {
    assert.throws(
        () => buildTranslationRequests({}),
        /text must be a string/,
    );
    assert.throws(
        () => buildTranslationRequests({ text: '', source: 'en', target: 'de' }),
        /text is required/,
    );
    assert.throws(
        () => buildTranslationRequests({ text: 'Hello', source: 'en', target: 'en' }),
        /target must be different from source/,
    );
    assert.throws(
        () => buildTranslationRequests({ items: [] }),
        /items must include at least one translation item/,
    );
    assert.throws(
        () => buildTranslationRequests({ items: ['Hello'] }),
        /items\[0\] must be an object/,
    );
    assert.throws(
        () => buildTranslationRequests({ items: [{ text: 'Hello', source: 'english', target: 'de' }] }),
        /items\[0\]\.source must be a language code/,
    );
});

test('describes translation requests', () => {
    assert.equal(
        describeTranslationRequests(buildTranslationRequests({
            items: [
                { text: 'Hello', source: 'en', target: 'de' },
                { text: 'Hola', source: 'es', target: 'fr' },
                { text: 'Bonjour', source: 'fr', target: 'it' },
                { text: 'Ciao', source: 'it', target: 'en' },
            ],
        })),
        '4 translation request(s): en->de, es->fr, fr->it and 1 more',
    );
});

test('input schema exposes batch items and single-item compatibility fields', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.properties.items.type, 'array');
    assert.equal(schema.properties.items.maxItems, 100);
    assert.deepEqual(schema.properties.items.items.required, ['text', 'source', 'target']);
    assert.equal(schema.properties.text.type, 'string');
    assert.equal(schema.properties.source.default, 'en');
    assert.equal(schema.properties.target.default, 'de');
});
