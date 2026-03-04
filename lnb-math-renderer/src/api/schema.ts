export interface RenderRequestBody {
    latex: string;
    display: boolean;
}

export interface ErrorResponse {
    error: string;
}

export function validateRenderRequest(
    request: unknown,
): request is RenderRequestBody {
    if (typeof request !== "object" || request === null) {
        return false;
    }

    if (!("latex" in request) || typeof request.latex !== "string") {
        return false;
    }

    if (!("display" in request) || typeof request.display !== "boolean") {
        return false;
    }

    return true;
}
