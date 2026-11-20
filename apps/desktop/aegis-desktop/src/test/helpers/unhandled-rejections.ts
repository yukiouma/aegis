/**
 * Capture the promise rejections that escape every handler, so a test can
 * assert that a failure was *handled* rather than left unhandled.
 *
 * Vitest reports an unhandled rejection without failing the file, which
 * means "the test passed" and "the rejection was handled" are not the
 * same claim. This closes that gap.
 */
export function captureUnhandledRejections() {
  type ProcessLike = {
    on(
      event: "unhandledRejection",
      listener: (reason: unknown) => void,
    ): void;
    off(
      event: "unhandledRejection",
      listener: (reason: unknown) => void,
    ): void;
  };

  const rejections: unknown[] = [];
  const capture = (reason: unknown) => {
    rejections.push(reason);
  };
  // `process` is a Node global that the browser-targeted tsconfig does
  // not know about; the cast keeps the package free of @types/node.
  const proc = (globalThis as unknown as { process?: ProcessLike }).process;
  proc?.on("unhandledRejection", capture);

  return {
    rejections,
    release: () => proc?.off("unhandledRejection", capture),
  };
}
