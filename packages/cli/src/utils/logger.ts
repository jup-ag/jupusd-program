import { format } from "node:util";
import pino, {
  type Logger as PinoLogger,
  type TransportSingleOptions,
} from "pino";

export interface Logger {
  info(message: unknown, ...args: unknown[]): void;
  success(message: unknown, ...args: unknown[]): void;
  warn(message: unknown, ...args: unknown[]): void;
  error(message: unknown, ...args: unknown[]): void;
  debug(message: unknown, ...args: unknown[]): void;
  flush(): Promise<void>;
}

const shouldPrettyPrint =
  process.env.NODE_ENV !== "production" &&
  process.env.CLI_DISABLE_PRETTY_LOGS !== "1";

let baseLogger: PinoLogger;

if (shouldPrettyPrint) {
  try {
    // Use pino-pretty but configure it to avoid exit listeners
    const pinoPretty = require("pino-pretty");
    const prettyStream = pinoPretty({
      colorize: true,
      singleLine: true,
      ignore: "pid,hostname,scope",
      // These options help avoid exit listeners
      sync: true,
      mkdir: false,
    });
    baseLogger = pino(
      {
        level: process.env.LOG_LEVEL ?? (process.env.DEBUG ? "debug" : "info"),
        base: undefined,
      },
      prettyStream,
    );
  } catch (error) {
    // eslint-disable-next-line no-console
    console.warn(
      `Failed to initialize pretty logger: ${(error as Error).message}. Falling back to JSON output.`,
    );
    baseLogger = pino({
      level: process.env.LOG_LEVEL ?? (process.env.DEBUG ? "debug" : "info"),
      base: undefined,
    });
  }
} else {
  baseLogger = pino({
    level: process.env.LOG_LEVEL ?? (process.env.DEBUG ? "debug" : "info"),
    base: undefined,
  });
}

const formatMessage = (message: unknown, args: unknown[]): string => {
  if (typeof message === "string") {
    return args.length ? format(message, ...args) : message;
  }

  if (args.length > 0) {
    return format(message as never, ...(args as never[]));
  }

  if (typeof message === "object") {
    try {
      return JSON.stringify(message);
    } catch {
      return String(message);
    }
  }

  return String(message);
};

const wrapLogger = (logger: PinoLogger): Logger => ({
  info(message: unknown, ...args: unknown[]) {
    logger.info(formatMessage(message, args));
  },
  success(message: unknown, ...args: unknown[]) {
    logger.info({ status: "success" }, formatMessage(message, args));
  },
  warn(message: unknown, ...args: unknown[]) {
    logger.warn(formatMessage(message, args));
  },
  error(message: unknown, ...args: unknown[]) {
    logger.error(formatMessage(message, args));
  },
  debug(message: unknown, ...args: unknown[]) {
    logger.debug(formatMessage(message, args));
  },
  async flush() {
    const flushFn = (
      logger as PinoLogger & {
        flush?: (callback?: (err?: Error) => void) => void;
      }
    ).flush;

    if (typeof flushFn !== "function") {
      return;
    }

    await new Promise<void>((resolve, reject) => {
      try {
        flushFn.call(logger, (error?: Error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      } catch (error) {
        reject(error as Error);
      }
    });
  },
});

export function getLogger(): Logger {
  return wrapLogger(baseLogger);
}
