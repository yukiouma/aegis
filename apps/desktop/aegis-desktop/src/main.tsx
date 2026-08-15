import React from "react";
import ReactDOM from "react-dom/client";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { routeTree } from "./routes/routeTree.gen";
import { QueryProvider } from "./data/client";
import { DocumentLangSync } from "./DocumentLangSync";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  PersistentThemeProvider,
  PersistentI18nProvider,
  SettingsSyncBridge,
} from "./SettingsSyncBridge";
import { shouldRedirectToBootstrap } from "./bootstrap-redirect";

// Set the app entry to /bootstrap so the health check and login
// status probe always run before the user reaches the login page
// or the authenticated home. The fallback below handles the case
// where `history.replaceState` is a no-op under the tauri://
// protocol — detected by reading the pathname after the call.
//
// Workspace windows open at /project/<code> and must skip the
// bootstrap probes entirely (see bootstrap-redirect.ts).
const initialPath = window.location.pathname;
if (shouldRedirectToBootstrap(initialPath)) {
  window.history.replaceState(null, "", "/bootstrap");
}
const router = createRouter({ routeTree });
if (shouldRedirectToBootstrap(window.location.pathname)) {
  // replaceState did not move us — fall back to a router navigate.
  void router.navigate({ to: "/bootstrap", replace: true });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

function App() {
  React.useEffect(() => {
    requestAnimationFrame(() => {
      requestAnimationFrame(async () => {
        try {
          const win = getCurrentWindow();
          await win.show();
          await win.setFocus();
        } catch (err) {
          console.error("[App] Failed to show window:", err);
        }
      });
    });
  });
  return (
    <React.StrictMode>
      <PersistentThemeProvider>
        <QueryProvider>
          <PersistentI18nProvider>
            <SettingsSyncBridge>
              <DocumentLangSync />
              <RouterProvider router={router} />
              {import.meta.env.DEV && (
                <TanStackRouterDevtools router={router} position="bottom-right" />
              )}
            </SettingsSyncBridge>
          </PersistentI18nProvider>
        </QueryProvider>
      </PersistentThemeProvider>
    </React.StrictMode>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />,
);
