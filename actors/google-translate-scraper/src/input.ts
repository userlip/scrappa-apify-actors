export interface TranslationItemInput {
    text?: unknown;
    source?: unknown;
    target?: unknown;
}

export interface GoogleTranslateInput extends TranslationItemInput {
    items?: unknown;
}

export interface TranslationRequest {
    index: number;
    text: string;
    source: string;
    target: string;
    params: Record<string, unknown>;
}

const MAX_ITEMS_PER_RUN = 100;
const MAX_TEXT_LENGTH = 5000;
const LANGUAGE_CODE_PATTERN = /^[a-z]{2,3}(?:-(?:[A-Z][a-z]{3}|[A-Z]{2}|[0-9]{3}))?$/;

function normalizeLanguageCode(value: string): string {
    if (!value.includes('-')) {
        return value.toLowerCase();
    }

    const [language, region] = value.split('-', 2);
    const normalizedRegion = region.length === 2
        ? region.toUpperCase()
        : `${region.slice(0, 1).toUpperCase()}${region.slice(1).toLowerCase()}`;

    return `${language.toLowerCase()}-${normalizedRegion}`;
}

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (trimmed === '') {
        throw new Error(`${field} is required`);
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanLanguage(value: unknown, field: string): string {
    const language = normalizeLanguageCode(cleanRequiredString(value, field, 10));
    if (!LANGUAGE_CODE_PATTERN.test(language)) {
        throw new Error(`${field} must be a language code like en, de, es-419, zh-CN, pt-BR, or mni-Mtei`);
    }

    return language;
}

function getRawItems(input: GoogleTranslateInput): TranslationItemInput[] {
    if (input.items !== undefined && input.items !== null) {
        if (!Array.isArray(input.items)) {
            throw new Error('items must be an array');
        }

        if (input.items.length === 0) {
            throw new Error('items must include at least one translation item');
        }

        if (input.items.length > MAX_ITEMS_PER_RUN) {
            throw new Error(`items cannot include more than ${MAX_ITEMS_PER_RUN} translations`);
        }

        return input.items.map((item, index) => {
            if (!item || typeof item !== 'object' || Array.isArray(item)) {
                throw new Error(`items[${index}] must be an object`);
            }

            return item as TranslationItemInput;
        });
    }

    return [input];
}

export function buildTranslationRequests(input: GoogleTranslateInput): TranslationRequest[] {
    const rawItems = getRawItems(input);

    return rawItems.map((item, index) => {
        const prefix = input.items === undefined || input.items === null ? '' : `items[${index}].`;
        const text = cleanRequiredString(item.text, `${prefix}text`, MAX_TEXT_LENGTH);
        const source = cleanLanguage(item.source, `${prefix}source`);
        const target = cleanLanguage(item.target, `${prefix}target`);

        if (source === target) {
            throw new Error(`${prefix}target must be different from ${prefix}source`);
        }

        return {
            index,
            text,
            source,
            target,
            params: { text, source, target },
        };
    });
}

export function describeTranslationRequests(requests: TranslationRequest[]): string {
    const sample = requests
        .slice(0, 3)
        .map((request) => `${request.source}->${request.target}`);
    const suffix = requests.length > sample.length ? ` and ${requests.length - sample.length} more` : '';

    return `${requests.length} translation request(s): ${sample.join(', ')}${suffix}`;
}
