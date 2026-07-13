import test from 'node:test';
import assert from 'node:assert/strict';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const { buildVintedUserProfileDatasetItem, getVintedUserProfile } = await import(`../${sourceDirectory}/response-utils.js`);

const profile = {
    id: 255914028,
    login: 'agranier',
    country_code: 'DE',
    city: 'Wiesbaden',
    feedback_count: 46,
    feedback_reputation: 0.98,
    positive_feedback_count: 45,
    neutral_feedback_count: 0,
    negative_feedback_count: 1,
    bundle_discount: {
        enabled: true,
        discounts: [{ minimal_item_count: 2, fraction: '0.05' }],
    },
    item_count: 7,
    total_items_count: 19,
    followers_count: 0,
    following_count: 0,
    last_loged_on_ts: '2026-07-13T10:12:36+02:00',
    last_loged_on: 'heute 10:12 Uhr',
    business: false,
    is_on_holiday: false,
    is_account_banned: false,
    profile_url: 'https://www.vinted.de/member/255914028-agranier',
    verification: {
        email: { valid: true },
        facebook: { valid: false },
        google: { valid: false },
    },
};

test('maps the wrapped Scrappa user profile and request metadata', () => {
    const response = { success: true, data: { user: profile }, meta: { duration_ms: 2451.07, scraped_at: '2026-07-13T09:49:21Z' } };
    const resolved = getVintedUserProfile(response);
    const item = buildVintedUserProfileDatasetItem(resolved, {
        userId: '255914028',
        params: { user_id: '255914028', country: 'DE' },
        index: 0,
    }, response);

    assert.equal(item.login, 'agranier');
    assert.equal(item.feedback_reputation, 0.98);
    assert.equal(item.bundle_discount_enabled, true);
    assert.deepEqual(item.bundle_discounts, [{ minimal_item_count: 2, fraction: '0.05' }]);
    assert.equal(item.item_count, 7);
    assert.equal(item.total_items_count, 19);
    assert.equal(item.last_activity, '2026-07-13T10:12:36+02:00');
    assert.equal(item.is_email_verified, true);
    assert.equal(item.is_facebook_verified, false);
    assert.equal(item.profile_url, profile.profile_url);
    assert.equal(item.request_user_id, '255914028');
    assert.equal(item.request_country, 'DE');
    assert.equal(item.scrappa_duration_ms, 2451.07);
    assert.equal(item.request_success, true);
});

test('supports a direct profile envelope and rejects failed or unresolved profiles', () => {
    assert.equal(getVintedUserProfile({ success: true, user: profile }).login, 'agranier');
    assert.throws(() => getVintedUserProfile({ success: false, message: 'User not found', status_code: 404 }), /User not found/);
    assert.throws(() => getVintedUserProfile({ success: true, data: { user: { id: 42, can_view_profile: false } } }), /private or unavailable/);
    assert.throws(() => getVintedUserProfile({ success: true, data: { user: {} } }), /did not include/);
    assert.throws(() => getVintedUserProfile({ success: true, data: { user: { id: null, login: 'missing-id', profile_url: 'https://www.vinted.de/member/missing-id' } } }), /incomplete/);
    assert.throws(() => getVintedUserProfile({ success: true, data: { user: { id: 42 } } }), /incomplete/);
    assert.throws(() => getVintedUserProfile({ success: true, data: { user: { id: 42, login: 'sparse' } } }), /incomplete/);
});
