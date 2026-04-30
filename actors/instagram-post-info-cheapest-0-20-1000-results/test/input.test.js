import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveInstagramPostInput } from '../src/input.js';

test('uses a non-empty URL when provided', () => {
    assert.deepEqual(resolveInstagramPostInput({
        url: ' https://www.instagram.com/natgeo/p/DXHKcyvEWfr/ ',
        shortcode: 'SHOULD_NOT_BE_USED',
    }), {
        identifier: 'https://www.instagram.com/natgeo/p/DXHKcyvEWfr/',
        params: { url: 'https://www.instagram.com/natgeo/p/DXHKcyvEWfr/' },
    });
});

test('uses a bare domain URL as URL input', () => {
    assert.deepEqual(resolveInstagramPostInput({
        url: ' instagram.com/natgeo/p/DXHKcyvEWfr/ ',
        shortcode: 'SHOULD_NOT_BE_USED',
    }), {
        identifier: 'instagram.com/natgeo/p/DXHKcyvEWfr/',
        params: { url: 'instagram.com/natgeo/p/DXHKcyvEWfr/' },
    });
});

test('falls back to shortcode when URL is empty', () => {
    assert.deepEqual(resolveInstagramPostInput({
        url: '',
        shortcode: 'DXHKcyvEWfr',
    }), {
        identifier: 'DXHKcyvEWfr',
        params: { shortcode: 'DXHKcyvEWfr' },
    });
});

test('uses URL field as shortcode when it is not a URL', () => {
    assert.deepEqual(resolveInstagramPostInput({
        url: ' DXHKcyvEWfr ',
    }), {
        identifier: 'DXHKcyvEWfr',
        params: { shortcode: 'DXHKcyvEWfr' },
    });
});

test('falls back to legacy media_id when URL and shortcode are empty', () => {
    assert.deepEqual(resolveInstagramPostInput({
        url: '   ',
        shortcode: '',
        media_id: 'DXHKcyvEWfr',
    }), {
        identifier: 'DXHKcyvEWfr',
        params: { shortcode: 'DXHKcyvEWfr' },
    });
});

test('rejects missing URL and shortcode input', () => {
    assert.throws(
        () => resolveInstagramPostInput({ url: '', shortcode: ' ', media_id: null }),
        /Instagram post URL or shortcode is required/,
    );
});

test('rejects null input with the expected validation error', () => {
    assert.throws(
        () => resolveInstagramPostInput(null),
        /Instagram post URL or shortcode is required/,
    );
});
