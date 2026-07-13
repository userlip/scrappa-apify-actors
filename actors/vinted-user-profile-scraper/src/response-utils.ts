import type { VintedUserProfileRequest } from './request-params.js';

export interface VintedVerificationRecord {
    valid?: boolean;
    available?: boolean;
    verified_at?: string | null;
    [key: string]: unknown;
}

export interface VintedBundleDiscount {
    id?: string | number;
    user_id?: string | number;
    enabled?: boolean;
    minimal_item_count?: number;
    fraction?: string | number;
    discounts?: Array<Record<string, unknown>>;
    [key: string]: unknown;
}

export interface VintedUserProfile {
    id?: string | number;
    login?: string;
    country_code?: string;
    country_iso_code?: string;
    country_title?: string;
    country_title_local?: string;
    city?: string;
    feedback_count?: number;
    feedback_reputation?: number;
    positive_feedback_count?: number;
    neutral_feedback_count?: number;
    negative_feedback_count?: number;
    bundle_discount?: VintedBundleDiscount | null;
    last_loged_on_ts?: string | null;
    last_loged_on?: string | null;
    item_count?: number;
    total_items_count?: number;
    followers_count?: number;
    following_count?: number;
    business?: boolean;
    business_account_id?: string | number | null;
    is_on_holiday?: boolean;
    is_account_banned?: boolean;
    can_view_profile?: boolean;
    profile_url?: string;
    share_profile_url?: string;
    path?: string;
    verification?: {
        email?: VintedVerificationRecord;
        facebook?: VintedVerificationRecord;
        google?: VintedVerificationRecord;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export interface VintedUserProfileResponse {
    success?: boolean;
    data?: Record<string, unknown> | VintedUserProfile;
    user?: VintedUserProfile;
    message?: string;
    status_code?: number | string;
    meta?: Record<string, unknown>;
    [key: string]: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isProfileCandidate(value: unknown): value is VintedUserProfile {
    if (!isRecord(value)) {
        return false;
    }

    return ['id', 'login', 'feedback_count', 'profile_url', 'can_view_profile'].some((field) => value[field] !== undefined);
}

function nonEmptyString(value: unknown): string | null {
    if (typeof value !== 'string') {
        return null;
    }

    const normalized = value.trim();
    return normalized === '' ? null : normalized;
}

function hasValidProfileIdentity(value: unknown): boolean {
    if (typeof value === 'number') {
        return Number.isSafeInteger(value) && value > 0;
    }

    const normalized = nonEmptyString(value);
    return normalized !== null && /^[1-9]\d*$/.test(normalized);
}

function isResolvedProfile(profile: VintedUserProfile): boolean {
    const hasPublicMarker = nonEmptyString(profile.profile_url) ?? nonEmptyString(profile.path);

    return hasValidProfileIdentity(profile.id)
        && nonEmptyString(profile.login) !== null
        && hasPublicMarker !== null;
}

function responseFailureMessage(response: VintedUserProfileResponse): string {
    const dataMessage = isRecord(response.data) ? nonEmptyString(response.data.message) : null;
    const message = nonEmptyString(response.message) ?? dataMessage;
    const statusCode = response.status_code === undefined ? '' : ` (status_code: ${response.status_code})`;

    return `${message ?? 'Scrappa response reported failure'}${statusCode}`;
}

export function getVintedUserProfile(response: VintedUserProfileResponse): VintedUserProfile {
    if (response.success === false) {
        throw new Error(responseFailureMessage(response));
    }

    const data = isRecord(response.data) ? response.data : undefined;
    const profile = (isProfileCandidate(response.user) ? response.user : undefined)
        ?? (isProfileCandidate(data?.user) ? data.user : undefined)
        ?? (isProfileCandidate(data) ? data : undefined)
        ?? (isProfileCandidate(response) ? response : undefined);

    if (!profile) {
        throw new Error('Scrappa response did not include Vinted user profile details');
    }

    if (profile.can_view_profile === false) {
        throw new Error('Vinted user profile is private or unavailable');
    }

    if (profile.is_account_banned === true) {
        throw new Error('Vinted user profile is banned or unavailable');
    }

    if (!isResolvedProfile(profile)) {
        throw new Error('Scrappa response included an incomplete Vinted user profile');
    }

    return profile;
}

function verificationValid(value: unknown): boolean | null {
    if (!isRecord(value) || typeof value.valid !== 'boolean') {
        return null;
    }

    return value.valid;
}

export function buildVintedUserProfileDatasetItem(
    profile: VintedUserProfile,
    request: VintedUserProfileRequest,
    response: VintedUserProfileResponse,
): Record<string, unknown> {
    const bundleDiscount = isRecord(profile.bundle_discount) ? profile.bundle_discount : null;
    const verification = isRecord(profile.verification) ? profile.verification : {};
    const meta = isRecord(response.meta) ? response.meta : {};
    const countryCode = profile.country_code ?? profile.country_iso_code ?? null;
    const lastActivity = profile.last_loged_on_ts ?? profile.last_loged_on ?? null;

    return {
        // Preserve raw public Scrappa profile fields, then provide stable analysis fields.
        ...profile,
        id: profile.id ?? null,
        login: profile.login ?? null,
        country_code: countryCode,
        country_iso_code: profile.country_iso_code ?? countryCode,
        country_title: profile.country_title ?? profile.country_title_local ?? null,
        city: profile.city ?? null,
        feedback_count: profile.feedback_count ?? null,
        feedback_reputation: profile.feedback_reputation ?? null,
        positive_feedback_count: profile.positive_feedback_count ?? null,
        neutral_feedback_count: profile.neutral_feedback_count ?? null,
        negative_feedback_count: profile.negative_feedback_count ?? null,
        bundle_discount: bundleDiscount,
        bundle_discount_enabled: bundleDiscount?.enabled ?? null,
        bundle_discounts: bundleDiscount?.discounts ?? [],
        item_count: profile.item_count ?? null,
        total_items_count: profile.total_items_count ?? null,
        followers_count: profile.followers_count ?? null,
        following_count: profile.following_count ?? null,
        last_activity: lastActivity,
        last_activity_at: profile.last_loged_on_ts ?? null,
        last_activity_localized: profile.last_loged_on ?? null,
        verification,
        is_email_verified: verificationValid(verification.email),
        is_facebook_verified: verificationValid(verification.facebook),
        is_google_verified: verificationValid(verification.google),
        business: profile.business ?? null,
        business_account_id: profile.business_account_id ?? null,
        is_on_holiday: profile.is_on_holiday ?? null,
        is_account_banned: profile.is_account_banned ?? false,
        profile_url: profile.profile_url ?? null,
        share_profile_url: profile.share_profile_url ?? profile.profile_url ?? null,
        path: profile.path ?? null,
        request_user_id: request.userId,
        request_country: request.params.country,
        request_index: request.index,
        request_success: true,
        scrappa_duration_ms: meta.duration_ms ?? null,
        scrappa_scraped_at: meta.scraped_at ?? null,
    };
}
