import assert from 'node:assert/strict';
import test from 'node:test';

const resultsModule = process.env.ACTOR_TEST_TARGET === 'src'
    ? '../src/results.ts'
    : '../dist/results.js';
const sharedModule = process.env.ACTOR_TEST_TARGET === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';

const {
    buildLinkedInJobDetailsDatasetItem,
    buildLinkedInJobDetailsFailureItem,
    buildLinkedInJobDetailsOutput,
    isRecoverableLinkedInJobDetailsError,
} = await import(resultsModule);
const { ScrappaApiError } = await import(sharedModule);

test('buildLinkedInJobDetailsDatasetItem preserves response fields and adds batch metadata', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsDatasetItem(
            {
                success: true,
                job_title: 'Software Engineer',
                company_name: 'Example Corp',
                date_posted: '2026-06-01',
                applicant_count: '23 applicants',
                application_url: 'https://example.com/apply',
            },
            'linkedin.com/jobs/view/1234567890',
            'https://www.linkedin.com/jobs/view/1234567890',
        ),
        {
            success: true,
            job_title: 'Software Engineer',
            company_name: 'Example Corp',
            date_posted: '2026-06-01',
            applicant_count: '23 applicants',
            application_url: 'https://example.com/apply',
            title: 'Software Engineer',
            company: 'Example Corp',
            posted_date: '2026-06-01',
            applicants: '23 applicants',
            apply_url: 'https://example.com/apply',
            url: 'https://www.linkedin.com/jobs/view/1234567890',
            input_url: 'linkedin.com/jobs/view/1234567890',
            normalized_url: 'https://www.linkedin.com/jobs/view/1234567890',
        },
    );
});

test('buildLinkedInJobDetailsDatasetItem keeps canonical fields over aliases', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsDatasetItem(
            {
                title: 'Senior Software Engineer',
                job_title: 'Software Engineer',
                company: 'Canonical Corp',
                company_name: 'Alias Corp',
                url: 'https://www.linkedin.com/jobs/view/1234567890/',
            },
            'https://www.linkedin.com/jobs/view/1234567890/?trk=foo',
            'https://www.linkedin.com/jobs/view/1234567890',
        ),
        {
            success: true,
            title: 'Senior Software Engineer',
            job_title: 'Software Engineer',
            company: 'Canonical Corp',
            company_name: 'Alias Corp',
            url: 'https://www.linkedin.com/jobs/view/1234567890/',
            input_url: 'https://www.linkedin.com/jobs/view/1234567890/?trk=foo',
            normalized_url: 'https://www.linkedin.com/jobs/view/1234567890',
        },
    );
});

test('buildLinkedInJobDetailsDatasetItem treats empty canonical strings as missing', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsDatasetItem(
            {
                title: '',
                job_title: 'Software Engineer',
                company: '   ',
                company_name: 'Example Corp',
                posted_date: '',
                date_posted: '2026-06-01',
                apply_url: '',
                application_url: 'https://example.com/apply',
            },
            'linkedin.com/jobs/view/1234567890',
            'https://www.linkedin.com/jobs/view/1234567890',
        ),
        {
            success: true,
            title: 'Software Engineer',
            job_title: 'Software Engineer',
            company: 'Example Corp',
            company_name: 'Example Corp',
            posted_date: '2026-06-01',
            date_posted: '2026-06-01',
            apply_url: 'https://example.com/apply',
            application_url: 'https://example.com/apply',
            url: 'https://www.linkedin.com/jobs/view/1234567890',
            input_url: 'linkedin.com/jobs/view/1234567890',
            normalized_url: 'https://www.linkedin.com/jobs/view/1234567890',
        },
    );
});

test('buildLinkedInJobDetailsFailureItem emits per-item Scrappa error metadata', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsFailureItem(
            new ScrappaApiError(404, 'Not found'),
            'linkedin.com/jobs/view/missing-job',
            'https://www.linkedin.com/jobs/view/missing-job',
        ),
        {
            success: false,
            input_url: 'linkedin.com/jobs/view/missing-job',
            normalized_url: 'https://www.linkedin.com/jobs/view/missing-job',
            url: 'https://www.linkedin.com/jobs/view/missing-job',
            error: 'Scrappa API error (404): Not found',
            error_type: 'scrappa_api_error',
            message: 'Job not found',
            status_code: 404,
        },
    );
});

test('buildLinkedInJobDetailsOutput strips wrapper metadata for single-run compatibility', () => {
    assert.deepEqual(
        buildLinkedInJobDetailsOutput({
            success: true,
            title: 'Software Engineer',
            url: 'https://www.linkedin.com/jobs/view/1234567890',
            input_url: 'linkedin.com/jobs/view/1234567890',
            normalized_url: 'https://www.linkedin.com/jobs/view/1234567890',
            error: 'ignored wrapper error',
            error_type: 'ignored_wrapper_error',
        }),
        {
            success: true,
            title: 'Software Engineer',
        },
    );
});

test('isRecoverableLinkedInJobDetailsError only downgrades not-found API errors', () => {
    assert.equal(isRecoverableLinkedInJobDetailsError(new ScrappaApiError(404, 'Not found')), true);
    assert.equal(isRecoverableLinkedInJobDetailsError(new ScrappaApiError(401, 'Unauthorized')), false);
    assert.equal(isRecoverableLinkedInJobDetailsError(new ScrappaApiError(500, 'Unavailable')), false);
    assert.equal(isRecoverableLinkedInJobDetailsError(new Error('timeout')), false);
});
