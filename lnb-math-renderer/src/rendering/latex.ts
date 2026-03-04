import MathJax from "mathjax";
import { Resvg, type ResvgRenderOptions } from "@resvg/resvg-js";

const RENDERING_EX_SIZE = 32;

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

interface SvgView {
    width: number;
    height: number;
    viewBox: [number, number, number, number];
}

function addPadding(svg: string, padding: number): string {
    const viewBoxText = svg.match(/viewBox="([^"]+)"/)?.[1];
    const widthText = svg.match(/width="([\d\.]+)/)?.[1];
    const heightText = svg.match(/height="([\d\.]+)/)?.[1];
    if (!viewBoxText || !widthText || !heightText) return svg;

    const viewBox = viewBoxText.split(" ").map(Number);
    const width = Number(widthText);
    const height = Number(heightText);
    if (!width || !height || viewBox.length !== 4 || viewBox.some(isNaN))
        return svg;

    const newView = adjustPadding(
        { width, height, viewBox: viewBox as [number, number, number, number] },
        padding,
    );

    return svg
        .replace(/height="[^\d\.]+/, `viewBox="${newView.height}`)
        .replace(/width="[^\d\.]+/, `viewBox="${newView.width}`)
        .replace(/viewBox="[^"]+"/, `viewBox="${newView.viewBox.join(" ")}"`);
}

function adjustPadding(svgView: SvgView, padding: number): SvgView {
    const newViewBox = [
        svgView.viewBox[0] - padding,
        svgView.viewBox[1] - padding,
        svgView.viewBox[2] + padding * 2,
        svgView.viewBox[3] + padding * 2,
    ] satisfies [number, number, number, number];
    // height 側を保持する
    const newRatio = newViewBox[2] / newViewBox[3];
    const newWidth = svgView.height * newRatio;
    return {
        width: newWidth,
        height: svgView.height,
        viewBox: newViewBox,
    };
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
    let svg = mj.startup.adaptor.innerHTML(svgNode);

    if (options?.padding ?? 0 > 0) {
        svg = addPadding(svg, options!.padding!);
    }

    const resvgOptions: ResvgRenderOptions = {
        fitTo: { mode: "zoom", value: options?.scale ?? 1.0 },
        background: options?.backgroundColor ?? "white",
    };

    const resvg = new Resvg(svg, resvgOptions);
    const rendered = resvg.render();
    return rendered.asPng();
}
