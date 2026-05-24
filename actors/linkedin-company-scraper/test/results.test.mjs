import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildLinkedInCompanyDatasetItem,
    buildLinkedInCompanyFailureItem,
    isRecoverableLinkedInCompanyError,
} from '../dist/results.js';
import { ScrappaApiError } from '../dist/shared/index.js';

test('buildLinkedInCompanyDatasetItem preserves response fields and adds batch metadata', () => {
    assert.deepEqual(
        buildLinkedInCompanyDatasetItem(
            {
                success: true,
                name: 'Microsoft',
                followers: 1,
            },
            'linkedin.com/company/microsoft',
            'https://www.linkedin.com/company/microsoft',
        ),
        {
            success: true,
            name: 'Microsoft',
            followers: 1,
            url: 'https://www.linkedin.com/company/microsoft',
            input_url: 'linkedin.com/company/microsoft',
            normalized_url: 'https://www.linkedin.com/company/microsoft',
        },
    );
});

test('buildLinkedInCompanyFailureItem emits per-item Scrappa error metadata', () => {
    assert.deepEqual(
        buildLinkedInCompanyFailureItem(
            new ScrappaApiError(404, 'Not found'),
            'linkedin.com/company/microsoft',
            'https://www.linkedin.com/company/microsoft',
        ),
        {
            success: false,
            input_url: 'linkedin.com/company/microsoft',
            normalized_url: 'https://www.linkedin.com/company/microsoft',
            url: 'https://www.linkedin.com/company/microsoft',
            error: 'Scrappa API error (404): Not found',
            error_type: 'scrappa_api_error',
            message: 'Company not found',
            status_code: 404,
        },
    );
});

test('isRecoverableLinkedInCompanyError only downgrades not-found API errors', () => {
    assert.equal(isRecoverableLinkedInCompanyError(new ScrappaApiError(404, 'Not found')), true);
    assert.equal(isRecoverableLinkedInCompanyError(new ScrappaApiError(401, 'Unauthorized')), false);
    assert.equal(isRecoverableLinkedInCompanyError(new ScrappaApiError(500, 'Unavailable')), false);
    assert.equal(isRecoverableLinkedInCompanyError(new Error('timeout')), false);
});
