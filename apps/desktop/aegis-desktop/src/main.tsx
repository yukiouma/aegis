import React from "react";
import ReactDOM from "react-dom/client";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { routeTree } from "./routes/routeTree.gen";
import { DocumentLangSync } from "./DocumentLangSync";
import { getCurrentWindow } from "@tauri-apps/api/window";

const router = createRouter({ routeTree });

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
      <AegisThemeProvider>
        <AegisI18nProvider>
          <DocumentLangSync />
          <RouterProvider router={router} />
          {import.meta.env.DEV && (
            <TanStackRouterDevtools router={router} position="bottom-right" />
          )}
        </AegisI18nProvider>
      </AegisThemeProvider>
    </React.StrictMode>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />,
);
