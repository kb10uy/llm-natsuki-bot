import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { cors } from "hono/cors";
import { logger } from "hono/logger";
import { health, renderMath } from "./api/endpoints.js";

const PORT = Number.parseInt(process.env["PORT"] ?? "3000", 10);

const app = new Hono();

app.use("*", logger());
app.use("*", cors());
app.onError((err, c) => {
    return c.json({ error: err.toString() }, 500);
});

app.get("/health", health);
app.post("/renderMath", renderMath);

console.log(`lnb-math-renderer listening on 0.0.0.0:${PORT}`);
serve({
    fetch: app.fetch,
    port: PORT,
});
