import pino from "pino";

export const logger = pino({
  level: "trace",
});

// Ensure unhandled errors are logged
process.on("unhandledRejection", (reason: any, promise) => {
  logger.error({ promise }, "Unhandled Rejection at:", reason.stack || reason);
});

// Ensure uncaught exceptions are logged
process.on("uncaughtException", (err) => {
  logger.fatal(err, "Uncaught Exception thrown");
  process.exit(1);
});