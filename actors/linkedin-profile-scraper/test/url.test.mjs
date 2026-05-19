import assert from 'node:assert/strict';
import test from 'node:test';

import { normalizeLinkedInProfileUrl } from '../dist/url.js';

test('normalizeLinkedInProfileUrl adds a protocol and strips query params', () => {
    assert.equal(
        normalizeLinkedInProfileUrl('linkedin.com/in/williamhgates/?trk=foo'),
        'https://www.linkedin.com/in/williamhgates',
    );
});

test('normalizeLinkedInProfileUrl normalizes regional and mobile LinkedIn hosts', () => {
    assert.equal(
        normalizeLinkedInProfileUrl('https://de.linkedin.com/in/satyanadella/details/experience/'),
        'https://www.linkedin.com/in/satyanadella',
    );
    assert.equal(
        normalizeLinkedInProfileUrl('https://m.linkedin.com/in/williamhgates/#about'),
        'https://www.linkedin.com/in/williamhgates',
    );
});

test('normalizeLinkedInProfileUrl accepts plain http LinkedIn URLs', () => {
    assert.equal(
        normalizeLinkedInProfileUrl('http://linkedin.com/in/williamhgates'),
        'http://www.linkedin.com/in/williamhgates',
    );
});

test('normalizeLinkedInProfileUrl rejects non-profile LinkedIn URLs', () => {
    assert.throws(
        () => normalizeLinkedInProfileUrl('https://www.linkedin.com/company/microsoft'),
        /Invalid LinkedIn profile URL/,
    );
});

test('normalizeLinkedInProfileUrl rejects non-LinkedIn domains', () => {
    assert.throws(
        () => normalizeLinkedInProfileUrl('https://example.com/in/williamhgates'),
        /Invalid LinkedIn profile URL/,
    );
});

test('normalizeLinkedInProfileUrl rejects non-HTTP schemes', () => {
    assert.throws(
        () => normalizeLinkedInProfileUrl('ftp://linkedin.com/in/williamhgates'),
        /Invalid LinkedIn profile URL/,
    );
});

test('normalizeLinkedInProfileUrl rejects LinkedIn URLs with credentials', () => {
    assert.throws(
        () => normalizeLinkedInProfileUrl('https://user:pass@linkedin.com/in/williamhgates'),
        /Invalid LinkedIn profile URL/,
    );
});

test('normalizeLinkedInProfileUrl rejects LinkedIn URLs with explicit ports', () => {
    assert.throws(
        () => normalizeLinkedInProfileUrl('https://linkedin.com:444/in/williamhgates'),
        /Invalid LinkedIn profile URL/,
    );
});
