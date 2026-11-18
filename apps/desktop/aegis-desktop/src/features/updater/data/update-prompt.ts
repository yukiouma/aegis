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
      const version = await readUpdateDeferral();
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
    await writeUpdateDeferral(ready.version);
  }, [ready]);

  return { ready, shouldPrompt, defer, status };
}
