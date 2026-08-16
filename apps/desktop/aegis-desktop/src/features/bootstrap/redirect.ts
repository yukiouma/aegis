/**
 * Decides whether the app should redirect to `/bootstrap` (which fires
 * the health check + login-status probes) based on the current
 * pathname. Workspace windows open at `/project/<code>` and must
 * skip the probes entirely: their route file's `beforeLoad` runs the
 * auth check, and the bootstrap probes would race against it and
 * overwrite the workspace URL on success (BootstrapPage navigates
 * to `/` on logged-in).
 *
 * Returns `true` only for the literal entry points — the path the
 * main window lands on before any URL manipulation. Returns `false`
 * for every other path, including workspace URLs.
 */
export function shouldRedirectToBootstrap(pathname: string): boolean {
  return pathname === "/" || pathname === "/index.html";
}