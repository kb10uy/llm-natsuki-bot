export interface RenderRequestBody {
    formula: string;
    display: boolean;
    scale: number;
}

export interface RenderMultipleRequestBody {
    formulae: string[];
    scale: number;
}

export interface ErrorResponse {
    error: string;
}

export function validateRenderMultipleRequest(
    request: unknown,
): request is RenderMultipleRequestBody {
    if (typeof request !== "object" || request === null) {
        return false;
    }

    if (
        !("formulae" in request) ||
        !Array.isArray(request.formulae) ||
        request.formulae.length === 0 ||
        !request.formulae.every((f) => typeof f === "string")
    ) {
        return false;
    }

    return true;
}

export function validateRenderRequest(
    request: unknown,
): request is RenderRequestBody {
    if (typeof request !== "object" || request === null) {
        return false;
    }

    if (!("formula" in request) || typeof request.formula !== "string") {
        return false;
    }

    if (!("display" in request) || typeof request.display !== "boolean") {
        return false;
    }

    return true;
}
