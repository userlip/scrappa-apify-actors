export function describeError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    if (typeof error === 'string') {
        return error;
    }

    try {
        const json = JSON.stringify(error);
        if (json !== undefined) {
            return json;
        }
    } catch {
        return String(error);
    }

    return String(error);
}
