import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeLinkedInCompanyUrl } from './url.js';

test('normalizeLinkedInCompanyUrl adds a protocol and strips query params', () => {
    assert.equal(
        normalizeLinkedInCompanyUrl('linkedin.com/company/microsoft/?trk=foo'),
        'https://www.linkedin.com/company/microsoft',
    );
});

test('normalizeLinkedInCompanyUrl normalizes regional and mobile LinkedIn hosts', () => {
    assert.equal(
        normalizeLinkedInCompanyUrl('https://de.linkedin.com/company/j.p.morgan/about/'),
        'https://www.linkedin.com/company/j.p.morgan',
    );
    assert.equal(
        normalizeLinkedInCompanyUrl('https://m.linkedin.com/company/microsoft/#about'),
        'https://www.linkedin.com/company/microsoft',
    );
});

test('normalizeLinkedInCompanyUrl rejects non-company LinkedIn URLs', () => {
    assert.throws(
        () => normalizeLinkedInCompanyUrl('https://www.linkedin.com/school/harvard'),
        /Invalid LinkedIn company URL/,
    );
});

test('normalizeLinkedInCompanyUrl rejects non-LinkedIn domains', () => {
    assert.throws(
        () => normalizeLinkedInCompanyUrl('https://example.com/company/acme'),
        /Invalid LinkedIn company URL/,
    );
});

test('normalizeLinkedInCompanyUrl rejects non-HTTP schemes', () => {
    assert.throws(
        () => normalizeLinkedInCompanyUrl('ftp://linkedin.com/company/acme'),
        /Invalid LinkedIn company URL/,
    );
});
