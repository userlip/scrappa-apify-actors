import assert from 'node:assert/strict';
import test from 'node:test';

import { buildWebScraperParams, describeWebScraperRequest } from '../dist/request-params.js';

const request = {
    input_url: 'https://example.com',
    request_url: 'https://example.com',
};

test('builds json params with include_html only when true', () => {
    assert.deepEqual(
        buildWebScraperParams(request, { include_html: true }, 'json'),
        {
            url: 'https://example.com',
            include_html: true,
            response_type: 'json',
        },
    );

    assert.deepEqual(
        buildWebScraperParams(request, { include_html: false }, 'json'),
        {
            url: 'https://example.com',
            include_html: false,
            response_type: 'json',
        },
    );
});

test('omits include_html for markdown params', () => {
    assert.deepEqual(
        buildWebScraperParams(request, { include_html: true }, 'markdown'),
        {
            url: 'https://example.com',
            include_html: undefined,
            response_type: 'markdown',
        },
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeWebScraperRequest({
            url: 'https://example.com',
            include_html: true,
            response_type: 'json',
        }),
        'url=https://example.com, response_type=json, include_html=true',
    );

    assert.equal(
        describeWebScraperRequest({
            url: 'https://example.com',
            response_type: 'markdown',
        }),
        'url=https://example.com, response_type=markdown',
    );
});
