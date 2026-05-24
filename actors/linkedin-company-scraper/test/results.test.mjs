import assert from 'node:assert/strict';
import test from 'node:test';

import { buildLinkedInCompanyDatasetItem, buildLinkedInCompanyFailureItem } from '../dist/results.js';
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
            new ScrappaApiError(422, 'maximum_cache_age must be at least 1'),
            'linkedin.com/company/microsoft',
            'https://www.linkedin.com/company/microsoft',
        ),
        {
            success: false,
            input_url: 'linkedin.com/company/microsoft',
            normalized_url: 'https://www.linkedin.com/company/microsoft',
            url: 'https://www.linkedin.com/company/microsoft',
            error: 'Scrappa API error (422): maximum_cache_age must be at least 1',
            error_type: 'scrappa_api_error',
            message: 'Scrappa API error (422): maximum_cache_age must be at least 1',
            status_code: 422,
        },
    );
});
