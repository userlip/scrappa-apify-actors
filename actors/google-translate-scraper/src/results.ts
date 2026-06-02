import type { TranslationRequest } from './input.js';
import { describeScrappaError, ScrappaHttpError } from './shared/index.js';

export interface GoogleTranslateResponse {
    translated_text?: unknown;
    translation?: unknown;
    result?: unknown;
    data?: unknown;
    [key: string]: unknown;
}

export interface TranslationDatasetItem {
    success: boolean;
    index: number;
    text: string;
    translated_text: string | null;
    source: string;
    target: string;
    error: string | null;
    status_code: number | null;
}

function readTranslationField(value: unknown): string | null {
    if (typeof value !== 'string') {
        return null;
    }

    const trimmed = value.trim();
    return trimmed === '' ? null : trimmed;
}

export function extractTranslatedText(response: unknown): string {
    if (typeof response === 'string') {
        const translated = readTranslationField(response);
        if (translated) {
            return translated;
        }
    }

    if (response && typeof response === 'object') {
        const payload = response as GoogleTranslateResponse;
        const translated = readTranslationField(payload.translated_text)
            ?? readTranslationField(payload.translation)
            ?? readTranslationField(payload.result);

        if (translated) {
            return translated;
        }

        if (payload.data !== undefined) {
            return extractTranslatedText(payload.data);
        }
    }

    throw new Error('Scrappa Google Translate response did not include translated_text');
}

export function buildTranslationDatasetItem(
    request: TranslationRequest,
    response: unknown,
): TranslationDatasetItem {
    return {
        success: true,
        index: request.index,
        text: request.text,
        translated_text: extractTranslatedText(response),
        source: request.source,
        target: request.target,
        error: null,
        status_code: null,
    };
}

export function buildTranslationFailureItem(
    request: TranslationRequest,
    error: unknown,
): TranslationDatasetItem {
    return {
        success: false,
        index: request.index,
        text: request.text,
        translated_text: null,
        source: request.source,
        target: request.target,
        error: describeScrappaError(error),
        status_code: error instanceof ScrappaHttpError ? error.status : null,
    };
}
