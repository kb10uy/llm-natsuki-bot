import type { Context } from "hono";
import type { BlankEnv, BlankInput } from "hono/types";
import type { ContentfulStatusCode } from "hono/utils/http-status";
import { validateRenderRequest } from "./schema.js";
import { renderToPng } from "../rendering/latex.js";

function error<P extends string>(
    c: Context<BlankEnv, P, BlankInput>,
    status: ContentfulStatusCode,
    message: string,
) {
    return c.json({ error: message }, status);
}

export async function health(c: Context<BlankEnv, "/health", BlankInput>) {
    return c.json({ status: "ok" });
}

export async function renderMath(
    c: Context<BlankEnv, "/renderMath", BlankInput>,
) {
    let request: unknown = await c.req.json();
    if (!validateRenderRequest(request)) {
        return error(c, 400, "invalid request");
    }

    try {
        const png = await renderToPng(request.latex.trim(), request.display, {
            scale: request.scale ?? 1.0,
            padding: 20,
        });
        return new Response(png, {
            status: 200,
            headers: {
                "Content-Type": "image/png",
                "Content-Length": png.length.toString(),
            },
        });
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        console.error(`[render error] ${message}`);
        return error(c, 500, message);
    }
}
