import { defineConfig, type Options } from "tsup";

const entryPoints: string[] = ["src/index.ts"];

const baseOptions: Partial<Options> = {
  entry: entryPoints,
  sourcemap: true,
  treeshake: true,
  minify: false,
  splitting: false,
};

export default defineConfig([
  {
    ...baseOptions,
    clean: true,
    format: ["cjs"],
    platform: "node",
    dts: {
      entry: {
        index: "src/index.ts",
      },
      resolve: true,
    },
    outDir: "dist",
    outExtension() {
      return {
        js: ".node.cjs",
      };
    },
  },
  {
    ...baseOptions,
    clean: false,
    format: ["esm"],
    platform: "node",
    dts: false,
    outDir: "dist",
    outExtension() {
      return {
        js: ".node.mjs",
      };
    },
  },
  {
    ...baseOptions,
    clean: false,
    format: ["cjs"],
    platform: "browser",
    dts: false,
    outDir: "dist",
    outExtension() {
      return {
        js: ".browser.cjs",
      };
    },
  },
  {
    ...baseOptions,
    clean: false,
    format: ["esm"],
    platform: "browser",
    dts: false,
    outDir: "dist",
    outExtension() {
      return {
        js: ".browser.mjs",
      };
    },
  },
  {
    ...baseOptions,
    clean: false,
    format: ["esm"],
    platform: "neutral",
    dts: false,
    outDir: "dist",
    outExtension() {
      return {
        js: ".react-native.mjs",
      };
    },
  },
]);
