import React from "react";
import ReactDOM from "react-dom/client";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { routeTree } from "./routes/routeTree.gen";
import { DocumentLangSync } from "./DocumentLangSync";

const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DocumentLangSync />
        <RouterProvider router={router} />
      </AegisI18nProvider>
    </AegisThemeProvider>
  </React.StrictMode>,
);
