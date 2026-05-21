import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildSimilarwebTrafficRequests,
    describeSimilarwebTrafficRequests,
} from '../dist/request-params.js';

test('builds normalized unique requests from single and batch inputs', () => {
    assert.deepEqual(
        buildSimilarwebTrafficRequests({
            domain: ' https://www.Google.com/search ',
            domains: ['google.com', 'GITHUB.com/features', 'www.shopify.com'],
        }),
        [
            { domain: 'google.com', inputDomain: 'https://www.Google.com/search' },
            { domain: 'github.com', inputDomain: 'GITHUB.com/features' },
            { domain: 'shopify.com', inputDomain: 'www.shopify.com' },
        ],
    );
});

test('rejects missing or malformed domains', () => {
    assert.throws(
        () => buildSimilarwebTrafficRequests({}),
        /At least one domain is required/,
    );
    assert.throws(
        () => buildSimilarwebTrafficRequests({ domains: 'google.com' }),
        /domains must be an array/,
    );
    assert.throws(
        () => buildSimilarwebTrafficRequests({ domain: 'not-a-valid-domain!!!' }),
        /must be a valid domain name/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeSimilarwebTrafficRequests([{ domain: 'google.com', inputDomain: 'google.com' }]),
        'google.com',
    );
    assert.equal(
        describeSimilarwebTrafficRequests([
            { domain: 'google.com', inputDomain: 'google.com' },
            { domain: 'github.com', inputDomain: 'github.com' },
            { domain: 'shopify.com', inputDomain: 'shopify.com' },
            { domain: 'stripe.com', inputDomain: 'stripe.com' },
        ]),
        '4 domains (google.com, github.com, shopify.com, ...)',
    );
});
