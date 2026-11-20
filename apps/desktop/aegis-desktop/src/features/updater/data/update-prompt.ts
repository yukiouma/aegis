import { useCallback, useEffect, useState } from "react";

import { readUpdateDeferral, writeUpdateDeferral } from "../../settings";
import { useUpdateStatus } from "./update-status";

/**
 * Whether the update dialog should be open.
 *
 * The prompt is derived from three facts: the status says a release is
 * ready, the deferral has actually been read from the store, and the
 * ready version is not the deferred one. The deferral must be loaded
 * first — otherwise the dialog flashes open on every launch for a
 * release the user already deferred.
 */
export function useUpdatePrompt() {
  const status = useUpdateStatus();
  const [deferredVersion, setDeferredVersion] = useState<string | null>(null);
  const [deferralLoaded, setDeferralLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      let version: string | null;
      try {
        version = await readUpdateDeferral();
      } catch {
        // A settings store that cannot be read leaves the deferral
        // unknown, and an unknown deferral must not become a prompt.
        // Leaving it unloaded keeps the dialog closed; the update is
        // still visible in the Settings row, which does not depend on
        // the deferral.
        return;
      }
      if (cancelled) return;
      setDeferredVersion(version);
      setDeferralLoaded(true);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const ready =
    status.data?.status.kind === "ready" ? status.data.status : null;
  const shouldPrompt =
    deferralLoaded && ready !== null && ready.version !== deferredVersion;

  const defer = useCallback(async () => {
    if (!ready) return;
    setDeferredVersion(ready.version);
    try {
      await writeUpdateDeferral(ready.version);
    } catch {
      // The deferral still applies to this session — the state above is
      // already set. Only the cross-launch persistence is lost.
    }
  }, [ready]);

  return { ready, shouldPrompt, defer, status };
}
