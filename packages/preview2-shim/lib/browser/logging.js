const levels = ["trace", "debug", "info", "warn", "error"];

let logLevel = levels.indexOf("warn");

export function setLevel(level) {
  logLevel = levels.indexOf(level);
}

export function log(level, context, msg) {
  if (logLevel > levels.indexOf(level)) return;
  console[level](`(${context}) ${msg}\n`);
}
