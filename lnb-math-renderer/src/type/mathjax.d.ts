declare module "mathjax" {
  interface MathJaxLoaderConfig {
    load: readonly string[];
  }

  interface MathJaxTexConfig {
    packages?: { "[+]": readonly string[] };
  }

  interface MathJaxSvgConfig {
    fontCache?: "local" | "global" | "none";
  }

  interface MathJaxStartupConfig {
    typeset?: boolean;
  }

  interface MathJaxInitConfig {
    loader?: MathJaxLoaderConfig;
    tex?: MathJaxTexConfig;
    svg?: MathJaxSvgConfig;
    startup?: MathJaxStartupConfig;
  }

  interface MathJaxAdaptor {
    innerHTML: (node: unknown) => string;
    outerHTML: (node: unknown) => string;
    serializeXML: (node: unknown) => string;
  }

  interface MathJaxStartup {
    adaptor: MathJaxAdaptor;
  }

  interface MathJaxInstance {
    tex2svgPromise: (latex: string, options: { display: boolean }) => Promise<unknown>;
    startup: MathJaxStartup;
  }

  interface MathJaxModule {
    init: (config: MathJaxInitConfig) => Promise<MathJaxInstance>;
  }

  const MathJax: MathJaxModule;
  export default MathJax;
}
