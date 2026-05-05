import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildTranscriptUrl } from '../src/transcript-url.js';

describe('buildTranscriptUrl', () => {
    it('builds a transcript URL with language and locale params', () => {
        const url = new URL(buildTranscriptUrl({
            id: 'dQw4w9WgXcQ',
            language: 'es',
            hl: 'EN',
            gl: 'us',
        }));

        assert.equal(url.origin + url.pathname, 'https://scrappa.co/api/youtube/transcript');
        assert.equal(url.searchParams.get('video_id'), 'dQw4w9WgXcQ');
        assert.equal(url.searchParams.get('id'), null);
        assert.equal(url.searchParams.get('language'), 'es');
        assert.equal(url.searchParams.get('hl'), 'en');
        assert.equal(url.searchParams.get('gl'), 'US');
    });

    it('accepts Apify select-array language input and optional lang alias', () => {
        const url = new URL(buildTranscriptUrl({
            id: 'dQw4w9WgXcQ',
            language: ['de'],
            lang: 'fr',
        }));

        assert.equal(url.searchParams.get('language'), 'de');
        assert.equal(url.searchParams.get('lang'), 'fr');
    });

    it('only sends debug when enabled', () => {
        const withoutDebug = new URL(buildTranscriptUrl({ id: 'dQw4w9WgXcQ', debug: false }));
        const withDebug = new URL(buildTranscriptUrl({ id: 'dQw4w9WgXcQ', debug: 'true' }));

        assert.equal(withoutDebug.searchParams.get('debug'), null);
        assert.equal(withDebug.searchParams.get('debug'), '1');
    });

    it('requires id', () => {
        assert.throws(() => buildTranscriptUrl({}), /id/);
        assert.throws(() => buildTranscriptUrl({ id: '   ' }), /id/);
    });
});
