import assert from 'node:assert/strict';
import test from 'node:test';

const resultsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/results.ts'
    : '../dist/results.js';
const sharedModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';
const {
    buildTranslationDatasetItem,
    buildTranslationFailureItem,
    extractTranslatedText,
} = await import(resultsModule);
const { ScrappaHttpError } = await import(sharedModule);

const request = {
    index: 1,
    text: 'Good morning',
    source: 'en',
    target: 'de',
    params: {
        text: 'Good morning',
        source: 'en',
        target: 'de',
    },
};

test('extracts translated text from Scrappa response shapes', () => {
    assert.equal(extractTranslatedText({ translated_text: 'Guten Morgen' }), 'Guten Morgen');
    assert.equal(extractTranslatedText({ data: { translated_text: 'Buenos dias' } }), 'Buenos dias');
    assert.equal(extractTranslatedText({ translation: 'Bonjour' }), 'Bonjour');
    assert.equal(extractTranslatedText({ result: 'Ciao' }), 'Ciao');
    assert.equal(extractTranslatedText('Hallo'), 'Hallo');
});

test('extracts translated text using documented field precedence', () => {
    assert.equal(
        extractTranslatedText({
            translated_text: 'Guten Morgen',
            translation: 'Bonjour',
            result: 'Ciao',
            data: { translated_text: 'Hola' },
        }),
        'Guten Morgen',
    );
    assert.equal(
        extractTranslatedText({
            translation: 'Bonjour',
            result: 'Ciao',
            data: { translated_text: 'Guten Morgen' },
        }),
        'Bonjour',
    );
    assert.equal(
        extractTranslatedText({
            result: 'Ciao',
            data: { translated_text: 'Guten Morgen' },
        }),
        'Ciao',
    );
});

test('throws when translation response has no translated text', () => {
    assert.throws(
        () => extractTranslatedText({ data: {} }),
        /did not include translated_text/,
    );
    assert.throws(
        () => extractTranslatedText(''),
        /did not include translated_text/,
    );
    assert.throws(
        () => extractTranslatedText({ translated_text: '   ' }),
        /did not include translated_text/,
    );
});

test('builds successful translation dataset item', () => {
    assert.deepEqual(
        buildTranslationDatasetItem(request, { translated_text: 'Guten Morgen' }),
        {
            success: true,
            index: 1,
            text: 'Good morning',
            translated_text: 'Guten Morgen',
            source: 'en',
            target: 'de',
            error: null,
            status_code: null,
        },
    );
});

test('builds failed translation dataset item with status metadata', () => {
    assert.deepEqual(
        buildTranslationFailureItem(request, new ScrappaHttpError(503, 'Translation service temporarily unavailable. Please retry.')),
        {
            success: false,
            index: 1,
            text: 'Good morning',
            translated_text: null,
            source: 'en',
            target: 'de',
            error: 'Scrappa API error (503): Translation service temporarily unavailable. Please retry.',
            status_code: 503,
        },
    );
});
