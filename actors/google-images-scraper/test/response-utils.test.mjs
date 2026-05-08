import assert from 'node:assert/strict';
import test from 'node:test';

import { enrichResult, extractImageResults } from '../dist/response-utils.js';

test('extracts image results from array and data wrapper responses', () => {
    const result = { position: 1, title: 'Coffee' };

    assert.deepEqual(extractImageResults([result]), [result]);
    assert.deepEqual(extractImageResults({ data: [result] }), [result]);
});

test('returns an empty result set for unexpected response shapes', () => {
    const originalWarn = console.warn;
    const warnings = [];
    console.warn = (message) => warnings.push(message);

    try {
        assert.deepEqual(extractImageResults({ results: [{ position: 1 }] }), []);
        assert.deepEqual(extractImageResults(null), []);
        assert.deepEqual(extractImageResults('unexpected'), []);
        assert.deepEqual(warnings, [
            'Scrappa Google Images response did not include an image result array',
            'Scrappa Google Images response did not include an image result array',
            'Scrappa Google Images response did not include an image result array',
        ]);
    } finally {
        console.warn = originalWarn;
    }
});

test('enriches image results with dataset aliases and default values', () => {
    assert.deepEqual(
        enrichResult(
            {
                title: 'Coffee',
                original: 'https://example.com/coffee.jpg',
                thumbnail: 'https://example.com/thumb.jpg',
                link: 'https://example.com/source',
            },
            {
                q: 'coffee',
                page: 1,
                hl: 'en',
                gl: 'us',
                imgsz: 'large',
                safe: 'active',
            },
        ),
        {
            title: 'Coffee',
            original: 'https://example.com/coffee.jpg',
            thumbnail: 'https://example.com/thumb.jpg',
            link: 'https://example.com/source',
            position: null,
            source: null,
            image_url: 'https://example.com/coffee.jpg',
            thumbnail_url: 'https://example.com/thumb.jpg',
            source_url: 'https://example.com/source',
            width: null,
            height: null,
            is_product: false,
            request_q: 'coffee',
            request_page: 1,
            request_hl: 'en',
            request_gl: 'us',
            request_imgsz: 'large',
            request_imgtype: null,
            request_imgcolor: null,
            request_imgar: null,
            request_tbs: null,
            request_safe: 'active',
        },
    );
});
