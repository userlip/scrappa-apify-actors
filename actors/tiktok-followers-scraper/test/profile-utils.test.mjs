import assert from 'node:assert/strict';
import test from 'node:test';

import { extractProfileUserId } from '../dist/profile-utils.js';

test('extractProfileUserId returns direct user_id values', () => {
    assert.equal(extractProfileUserId({ user_id: '1234567890' }), '1234567890');
    assert.equal(extractProfileUserId({ user_id: 1234567890 }), '1234567890');
});

test('extractProfileUserId returns nested user id values', () => {
    assert.equal(extractProfileUserId({ user: { id: '9876543210' } }), '9876543210');
    assert.equal(extractProfileUserId({ user: { id: 9876543210 } }), '9876543210');
});

test('extractProfileUserId reads the first profile from array responses', () => {
    assert.equal(extractProfileUserId([{ user_id: 'first' }, { user_id: 'second' }]), 'first');
});

test('extractProfileUserId returns null when no usable user id exists', () => {
    assert.equal(extractProfileUserId(null), null);
    assert.equal(extractProfileUserId(undefined), null);
    assert.equal(extractProfileUserId({}), null);
    assert.equal(extractProfileUserId({ user_id: '' }), null);
    assert.equal(extractProfileUserId([]), null);
});
