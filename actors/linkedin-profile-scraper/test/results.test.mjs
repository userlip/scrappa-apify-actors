import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildLinkedInProfileDatasetItem,
    buildLinkedInProfileFailureItem,
    isRecoverableLinkedInProfileError,
} from '../dist/results.js';
import { ScrappaApiError } from '../dist/shared/index.js';

test('buildLinkedInProfileDatasetItem preserves response fields and adds batch metadata', () => {
    assert.deepEqual(
        buildLinkedInProfileDatasetItem(
            {
                success: true,
                name: 'Bill Gates',
                followers: 1,
            },
            'linkedin.com/in/williamhgates',
            'https://www.linkedin.com/in/williamhgates',
        ),
        {
            success: true,
            name: 'Bill Gates',
            followers: 1,
            url: 'https://www.linkedin.com/in/williamhgates',
            input_url: 'linkedin.com/in/williamhgates',
            normalized_url: 'https://www.linkedin.com/in/williamhgates',
        },
    );
});

test('buildLinkedInProfileFailureItem emits per-item Scrappa error metadata', () => {
    assert.deepEqual(
        buildLinkedInProfileFailureItem(
            new ScrappaApiError(404, 'Not found'),
            'linkedin.com/in/missing-profile',
            'https://www.linkedin.com/in/missing-profile',
        ),
        {
            success: false,
            input_url: 'linkedin.com/in/missing-profile',
            normalized_url: 'https://www.linkedin.com/in/missing-profile',
            url: 'https://www.linkedin.com/in/missing-profile',
            error: 'Scrappa API error (404): Not found',
            error_type: 'scrappa_api_error',
            message: 'Profile not found',
            status_code: 404,
        },
    );
});

test('isRecoverableLinkedInProfileError only downgrades not-found API errors', () => {
    assert.equal(isRecoverableLinkedInProfileError(new ScrappaApiError(404, 'Not found')), true);
    assert.equal(isRecoverableLinkedInProfileError(new ScrappaApiError(401, 'Unauthorized')), false);
    assert.equal(isRecoverableLinkedInProfileError(new ScrappaApiError(500, 'Unavailable')), false);
    assert.equal(isRecoverableLinkedInProfileError(new Error('timeout')), false);
});
