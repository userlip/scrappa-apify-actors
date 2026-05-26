import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildFailureDatasetItem,
    buildJsonDatasetItem,
    buildMarkdownDatasetItem,
} from '../dist/response-utils.js';
import { ScrappaWebScraperHttpError } from '../dist/web-scraper-client.js';

const request = {
    input_url: 'https://example.com',
    request_url: 'https://example.com',
};

test('normalizes json response fields while preserving Scrappa response', () => {
    const item = buildJsonDatasetItem(
        {
            success: true,
            site_status_code: 200,
            url: 'https://example.com',
            final_url: 'https://example.com/',
            data: {
                title: 'Example Domain',
                description: 'Example description',
                body_text: 'Example body',
                links: ['https://www.iana.org/domains/example'],
                emails: [],
                phone_numbers: ['+1 555 1000'],
                images: [{ src: 'https://example.com/image.png' }],
                languages_detected: ['en'],
            },
        },
        request,
        {
            url: 'https://example.com',
            include_html: true,
            response_type: 'json',
        },
    );

    assert.equal(item.success, true);
    assert.equal(item.input_url, 'https://example.com');
    assert.equal(item.response_type, 'json');
    assert.equal(item.include_html, true);
    assert.equal(item.site_status_code, 200);
    assert.equal(item.title, 'Example Domain');
    assert.equal(item.links_count, 1);
    assert.equal(item.emails_count, 0);
    assert.equal(item.phone_numbers_count, 1);
    assert.equal(item.images_count, 1);
    assert.deepEqual(item.languages_detected, ['en']);
});

test('wraps markdown response in a dataset item', () => {
    const item = buildMarkdownDatasetItem(
        '# Example Domain\n\nContent',
        request,
        {
            url: 'https://example.com',
            response_type: 'markdown',
        },
    );

    assert.equal(item.success, true);
    assert.equal(item.response_type, 'markdown');
    assert.equal(item.include_html, false);
    assert.equal(item.markdown, '# Example Domain\n\nContent');
    assert.equal(item.markdown_length, 25);
});

test('builds per-url failure items from Scrappa errors', () => {
    const item = buildFailureDatasetItem(
        new ScrappaWebScraperHttpError(400, 'Invalid URL format.', { error_code: 'INVALID_URL' }),
        request,
        {
            url: 'bad-url',
            include_html: false,
            response_type: 'json',
        },
    );

    assert.equal(item.success, false);
    assert.equal(item.status_code, 400);
    assert.equal(item.error_type, 'scrappa_api_error');
    assert.equal(item.error_code, 'INVALID_URL');
    assert.match(String(item.error), /Invalid URL format/);
});
