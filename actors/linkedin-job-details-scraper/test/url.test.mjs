import assert from 'node:assert/strict';
import test from 'node:test';

import { normalizeLinkedInJobUrl } from '../dist/url.js';

test('normalizeLinkedInJobUrl accepts common LinkedIn jobs view URLs', () => {
    assert.equal(
        normalizeLinkedInJobUrl('linkedin.com/jobs/view/1234567890/?trk=public_jobs_topcard-title'),
        'https://www.linkedin.com/jobs/view/1234567890',
    );
    assert.equal(
        normalizeLinkedInJobUrl('https://de.linkedin.com/jobs/view/software-engineer-at-example-1234567890/'),
        'https://www.linkedin.com/jobs/view/software-engineer-at-example-1234567890',
    );
    assert.equal(
        normalizeLinkedInJobUrl('https://m.linkedin.com/jobs/view/1234567890?refId=abc#main-content'),
        'https://www.linkedin.com/jobs/view/1234567890',
    );
});

test('normalizeLinkedInJobUrl rejects non-job URLs', () => {
    assert.throws(
        () => normalizeLinkedInJobUrl('https://www.linkedin.com/in/example'),
        /Invalid LinkedIn job URL/,
    );
    assert.throws(
        () => normalizeLinkedInJobUrl('https://example.com/jobs/view/123'),
        /Invalid LinkedIn job URL/,
    );
});
