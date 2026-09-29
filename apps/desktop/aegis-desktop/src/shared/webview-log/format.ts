/**
 * Stringify the args of a `console.*` call into a single line. Used
 * by the webview log forwarder before sending the message over the
 * Tauri IPC. Designed to never throw — the forwarder sits on a hot
 * path so any error here must not break the calling app.
 *
 * Behaviour:
 * - `string` / `number` / `boolean` / `null` / `undefined` →
 *   `String(arg)`.
 * - `Error` → `"<name>: <message>\n<stack>"`.
 * - plain object / array → `JSON.stringify(arg)` (falls back to
 *   `String(arg)` when serialisation throws, e.g. on circular refs).
 * - multiple args are joined with a single space.
 */
export function formatConsoleArgs(args: readonly unknown[]): string {
  return args.map(formatOne).join(" ");
}

function formatOne(arg: unknown): string {
  if (arg === null) return "null";
  if (arg === undefined) return "undefined";
  if (typeof arg === "string") return arg;
  if (typeof arg === "number" || typeof arg === "boolean" || typeof arg === "bigint") {
    return String(arg);
  }
  if (arg instanceof Error) {
    const stack = arg.stack ?? "";
    return stack ? `${arg.name}: ${arg.message}\n${stack}` : `${arg.name}: ${arg.message}`;
  }
  if (typeof arg === "object") {
    try {
      return JSON.stringify(arg);
    } catch {
      return String(arg);
    }
  }
  return String(arg);
}