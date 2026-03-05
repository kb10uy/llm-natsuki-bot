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
    const filteredSvg = svg.replace(/data-latex="[^"]*"/g, "");
    return sharp(Buffer.from(filteredSvg), {
        density: RASTERIZE_DENSITY * scale,
    })
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

    let maxFormulaWidth = 0;
    let totalFormulaHeight = 0;
    let index = 1;
    let y = 0;
    const compositeInputs: sharp.OverlayOptions[] = [];
    for (const formula of formulae) {
        // Formula
        let svg;
        let formulaImage;
        try {
            const svgNode = await mj.tex2svgPromise(formula.trim(), {
                display: true,
                em: RENDERING_EX_SIZE * 2,
                ex: RENDERING_EX_SIZE,
            });
            svg = mj.startup.adaptor.innerHTML(svgNode);
            formulaImage = await renderSvgToPng(svg, scale, background);
        } catch (e) {
            console.error("rendering error: ", e);
            console.error("rendering error: ", formula);
            console.error("rendering error: ", svg);
            continue;
        }

        const dimensions = await sharp(formulaImage).metadata();
        const formulaWidth = dimensions.width ?? 0;
        const formulaHeight = dimensions.height ?? 0;
        maxFormulaWidth = Math.max(maxFormulaWidth, formulaWidth);
        totalFormulaHeight += formulaHeight;

        compositeInputs.push({ input: formulaImage, top: y, left: 0 });

        // Label
        const labelPng = await sharp({
            text: {
                text: `(${index})`,
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
            left: formulaWidth + LABEL_MARGIN,
        });

        // Linefeed
        y += formulaHeight + FORMULA_GAP;
        ++index;
    }

    let compositeImage = await sharp({
        create: {
            width: maxFormulaWidth + LABEL_MARGIN + LABEL_WIDTH,
            height: totalFormulaHeight + FORMULA_GAP * (formulae.length - 1),
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
