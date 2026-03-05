import MathJax from "mathjax";
import sharp from "sharp";

const RENDERING_EX_SIZE = 32;
const RASTERIZE_DENSITY = 96;

interface MathJaxInstance {
    tex2svgPromise: (
        latex: string,
        options: {
            display: boolean;
            em: number;
            ex: number;
        },
    ) => Promise<unknown>;
    startup: {
        adaptor: {
            innerHTML: (node: unknown) => string;
        };
    };
}

let mjx: MathJaxInstance | null = null;

async function getMathJax(): Promise<MathJaxInstance> {
    if (mjx !== null) return mjx;

    const initialized = await MathJax.init({
        loader: { load: ["input/tex", "output/svg"] },
        tex: {
            packages: { "[+]": ["ams"] },
        },
        svg: {
            fontCache: "local",
        },
        startup: {
            typeset: false,
        },
    });

    mjx = initialized as MathJaxInstance;
    return mjx;
}

export interface RenderOptions {
    readonly scale?: number;
    readonly padding?: number;
    readonly backgroundColor?: string;
}

export async function renderToPng(
    latexFormula: string,
    display: boolean,
    options?: RenderOptions,
): Promise<Uint8Array> {
    const mj = await getMathJax();

    const svgNode = await mj.tex2svgPromise(latexFormula, {
        display,
        em: RENDERING_EX_SIZE * 2,
        ex: RENDERING_EX_SIZE,
    });
    const svg = mj.startup.adaptor.innerHTML(svgNode);

    const scale = options?.scale ?? 1.0;
    const padding = options?.padding ?? 0;
    const background = options?.backgroundColor ?? "white";

    const image = sharp(Buffer.from(svg), {
        density: RASTERIZE_DENSITY * scale,
    });

    const png = await (
        padding > 0
            ? image.extend({
                  top: padding,
                  bottom: padding,
                  left: padding,
                  right: padding,
                  background,
              })
            : image
    )
        .flatten({ background })
        .png()
        .toBuffer();

    return new Uint8Array(png);
}
