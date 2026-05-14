import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildPlaylistDetailsUrl } from '../src/playlist-url.js';

describe('buildPlaylistDetailsUrl', () => {
    it('builds a playlist details URL with the playlist ID', () => {
        const url = new URL(buildPlaylistDetailsUrl({ id: 'PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf' }));

        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/playlists');
        assert.equal(url.searchParams.get('id'), 'PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf');
    });

    it('encodes playlist IDs in the query string', () => {
        const url = new URL(buildPlaylistDetailsUrl({ id: 'playlist id/with spaces' }));

        assert.equal(url.searchParams.get('id'), 'playlist id/with spaces');
        assert.match(url.toString(), /id=playlist\+id%2Fwith\+spaces/);
    });

    it('requires id', () => {
        assert.throws(() => buildPlaylistDetailsUrl({}), /id/);
        assert.throws(() => buildPlaylistDetailsUrl({ id: '   ' }), /id/);
        assert.throws(() => buildPlaylistDetailsUrl({ id: 123 }), /id/);
        assert.throws(() => buildPlaylistDetailsUrl(null), /id/);
    });
});
