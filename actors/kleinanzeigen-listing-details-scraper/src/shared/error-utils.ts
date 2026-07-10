const SENSITIVE_VALUE_PATTERN = /(?:x-api-key|api[_ -]?key|authorization|access[_ -]?token|token|secret)\s*["']?\s*[:=]\s*["']?[^\s,"'}\]]+/gi;

export function errorSummary(error: unknown, maxLength = 500): string {
    const message = error instanceof Error ? error.message : String(error);
    return message.replace(SENSITIVE_VALUE_PATTERN, '[redacted]').slice(0, maxLength);
}
