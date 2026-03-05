export interface RenderRequestBody {
    formula: string;
    display: boolean;
    scale: number;
}

export interface RenderMultipleRequestBody {
    formulae: string[];
    scale: number;
}

export interface RenderCommonOption {
    scale: number;
    preserveAlpha: boolean;
}

export interface ErrorResponse {
    error: string;
}

export function validateRenderMultipleRequest(
    payload: unknown,
): payload is RenderMultipleRequestBody & RenderCommonOption {
    if (typeof payload !== "object" || payload === null) {
        return false;
    }

    if (
        !("formulae" in payload) ||
        !Array.isArray(payload.formulae) ||
        payload.formulae.length === 0 ||
        !payload.formulae.every((f) => typeof f === "string")
    ) {
        return false;
    }

    return validateCommonOption(payload);
}

export function validateRenderRequest(
    payload: unknown,
): payload is RenderRequestBody & RenderCommonOption {
    if (typeof payload !== "object" || payload === null) {
        return false;
    }

    if (!("formula" in payload) || typeof payload.formula !== "string") {
        return false;
    }

    if (!("display" in payload) || typeof payload.display !== "boolean") {
        return false;
    }

    return validateCommonOption(payload);
}

function validateCommonOption(payload: unknown): payload is RenderCommonOption {
    if (typeof payload !== "object" || payload === null) {
        return false;
    }

    if (!("scale" in payload) || typeof payload.scale !== "number") {
        return false;
    }

    if (
        !("preserveAlpha" in payload) ||
        typeof payload.preserveAlpha !== "boolean"
    ) {
        return false;
    }

    return true;
}
