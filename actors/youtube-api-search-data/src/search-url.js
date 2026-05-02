const API_BASE_URL = 'https://scrappa.co/api/youtube/search';

const SORT_PARAM_VALUES = {
    upload_date: 'date',
    view_count: 'viewCount',
};

const UPLOAD_DATE_FILTERS = {
    hour: (now) => new Date(now.getTime() - 60 * 60 * 1000),
    today: (now) => new Date(now.getTime() - 24 * 60 * 60 * 1000),
    week: (now) => new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000),
    month: (now) => subtractUtcMonths(now, 1),
    year: (now) => subtractUtcMonths(now, 12),
};

const MAX_LIMIT = 20;

function singleValue(value) {
    const rawValue = Array.isArray(value) ? value.find((item) => item !== undefined && item !== null) : value;

    if (typeof rawValue !== 'string') {
        return undefined;
    }

    const trimmedValue = rawValue.trim();
    return trimmedValue === '' ? undefined : trimmedValue;
}

function positiveInteger(value) {
    if (!Number.isInteger(value) || value <= 0) {
        return undefined;
    }

    return value;
}

function daysInUtcMonth(year, month) {
    return new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
}

function subtractUtcMonths(date, months) {
    const totalMonths = date.getUTCFullYear() * 12 + date.getUTCMonth() - months;
    const year = Math.floor(totalMonths / 12);
    const month = totalMonths % 12;
    const day = Math.min(date.getUTCDate(), daysInUtcMonth(year, month));

    return new Date(Date.UTC(
        year,
        month,
        day,
        date.getUTCHours(),
        date.getUTCMinutes(),
        date.getUTCSeconds(),
        date.getUTCMilliseconds()
    ));
}

function publishedAfterValue(uploadDate, now) {
    const filter = UPLOAD_DATE_FILTERS[uploadDate];
    if (!filter) {
        return undefined;
    }

    return filter(now).toISOString();
}

export function buildSearchRequest(input = {}, options = {}) {
    const actorInput = input && typeof input === 'object' ? input : {};
    const query = singleValue(actorInput.q);
    if (!query) {
        throw new Error('Search query "q" is required.');
    }

    const params = new URLSearchParams({ query });
    const sort = singleValue(actorInput.sort);
    const now = options.now instanceof Date ? options.now : new Date();

    const stringParams = {
        // Map known Apify values, pass through Scrappa-compatible values, default to relevance.
        order: SORT_PARAM_VALUES[sort] ?? sort ?? 'relevance',
        videoDuration: singleValue(actorInput.duration),
        publishedAfter: publishedAfterValue(singleValue(actorInput.upload_date), now),
        continuation: singleValue(actorInput.continuation),
        type: singleValue(actorInput.type),
    };

    for (const [key, value] of Object.entries(stringParams)) {
        if (value !== undefined) {
            params.set(key, value);
        }
    }

    const limit = positiveInteger(actorInput.limit);
    if (limit) {
        params.set('limit', String(Math.min(limit, MAX_LIMIT)));
    }

    return {
        url: `${API_BASE_URL}?${params.toString()}`,
        query,
    };
}
