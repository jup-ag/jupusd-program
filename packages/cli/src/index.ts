#!/usr/bin/env node

import { Errors, flush, run } from "@oclif/core";
import { basename, join } from "node:path";

const isDevMode = basename(__dirname) === "src";
const commandsPath = join(__dirname, "commands");

export async function main(argv = process.argv.slice(2)) {
  try {
    await run(argv, {
      root: __dirname,
      pjson: {
        ...require(
          join(__dirname, isDevMode ? "../package.json" : "../package.json"),
        ),
        oclif: {
          ...require(
            join(__dirname, isDevMode ? "../package.json" : "../package.json"),
          ).oclif,
          commands: commandsPath,
        },
      },
    });
    await flush();
  } catch (error) {
    await Errors.handle(error as Error);
  }
}

if (require.main === module) {
  void main();
}
