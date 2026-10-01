import assert from 'node:assert/strict';
import test from 'node:test';
import { diffMetadata } from './apply-store-metadata.mjs';

test('diffMetadata reports changed metadata fields only', () => {
    const entry = {
        title: 'Google Maps Reviews Scraper',
        description: 'Collect public review data.',
        seoTitle: 'Google Maps Reviews Scraper',
        seoDescription: 'Collect Google Maps review data for local research.',
        categories: ['LEAD_GENERATION', 'MARKETING'],
    };
    const live = {
        title: 'Google Maps Reviews Scraper',
        description: 'Collect public reviews.',
        seoTitle: 'Google Maps Reviews Scraper',
        seoDescription: 'Collect Google Maps review data for local research.',
        categories: ['BUSINESS'],
        name: 'google-maps-reviews-scraper',
    };

    assert.deepEqual(diffMetadata(entry, live), {
        description: { before: 'Collect public reviews.', after: 'Collect public review data.' },
        categories: { before: ['BUSINESS'], after: ['LEAD_GENERATION', 'MARKETING'] },
    });
});

test('diffMetadata treats missing live values as changes', () => {
    const entry = {
        title: 'YouTube Playlist Search Scraper',
        description: 'Search public YouTube playlists.',
        seoTitle: 'YouTube Playlist Search Scraper',
        seoDescription: 'Search YouTube playlists and export results through Apify.',
        categories: ['VIDEOS'],
    };

    assert.deepEqual(diffMetadata(entry, {}), {
        title: { before: null, after: entry.title },
        description: { before: null, after: entry.description },
        seoTitle: { before: null, after: entry.seoTitle },
        seoDescription: { before: null, after: entry.seoDescription },
        categories: { before: null, after: entry.categories },
    });
});
