import MathJax from "mathjax";
import sharp from "sharp";

const RENDERING_EX_SIZE = 32;
const RASTERIZE_DENSITY = 96;
const FORMULA_GAP = 24;
const LABEL_MARGIN = 16;
const LABEL_WIDTH = 48;

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

async function renderSvgToPng(
    svg: string,
    scale: number,
    background: string,
): Promise<Buffer> {
    return sharp(Buffer.from(svg), { density: RASTERIZE_DENSITY * scale })
        .extend({
            top: 5,
            bottom: 5,
            left: 5,
            right: 5,
            background,
        })
        .flatten({ background })
        .png()
        .toBuffer();
}

export async function renderMultipleToPng(
    formulae: string[],
    options?: RenderOptions,
): Promise<Uint8Array> {
    const mj = await getMathJax();
    const scale = options?.scale ?? 1.0;
    const padding = options?.padding ?? 0;
    const background = options?.backgroundColor ?? "white";

    const formulaPngs = await Promise.all(
        formulae.map(async (latex) => {
            const svgNode = await mj.tex2svgPromise(latex.trim(), {
                display: true,
                em: RENDERING_EX_SIZE * 2,
                ex: RENDERING_EX_SIZE,
            });
            const svg = mj.startup.adaptor.innerHTML(svgNode);
            return renderSvgToPng(svg, scale, background);
        }),
    );
    const dimensions = await Promise.all(
        formulaPngs.map((png) => sharp(png).metadata()),
    );

    const maxFormulaWidth = Math.max(...dimensions.map((d) => d.width ?? 0));
    const totalWidth = maxFormulaWidth + LABEL_MARGIN + LABEL_WIDTH;
    const totalHeight =
        dimensions.reduce((sum, d) => sum + (d.height ?? 0), 0) +
        FORMULA_GAP * (formulae.length - 1);

    const compositeInputs: sharp.OverlayOptions[] = [];
    let y = 0;
    for (let i = 0; i < formulaPngs.length; i++) {
        const formulaHeight = dimensions[i]!.height ?? 0;

        compositeInputs.push({ input: formulaPngs[i]!, top: y, left: 0 });

        const labelPng = await sharp({
            text: {
                text: `(${i + 1})`,
                font: "sans",
                dpi: Math.round(72 * scale),
            },
        })
            .flatten({ background })
            .png()
            .toBuffer();

        const labelDimensions = await sharp(labelPng).metadata();
        const labelHeight = labelDimensions.height ?? 0;
        const labelTop = y + Math.round((formulaHeight - labelHeight) / 2);

        compositeInputs.push({
            input: labelPng,
            top: labelTop,
            left: maxFormulaWidth + LABEL_MARGIN,
        });

        y += formulaHeight + FORMULA_GAP;
    }

    let compositeImage = await sharp({
        create: {
            width: totalWidth,
            height: totalHeight,
            channels: 3,
            background,
        },
    })
        .png()
        .composite(compositeInputs)
        .toBuffer();

    if (padding > 0) {
        compositeImage = await sharp(compositeImage)
            .png()
            .extend({
                top: padding,
                bottom: padding,
                left: padding,
                right: padding,
                background,
            })
            .toBuffer();
    }

    return new Uint8Array(compositeImage);
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

    let image = await renderSvgToPng(svg, scale, background);
    if (padding > 0) {
        image = await sharp(image)
            .extend({
                top: padding,
                bottom: padding,
                left: padding,
                right: padding,
                background,
            })
            .png()
            .toBuffer();
    }

    return new Uint8Array(image);
}
