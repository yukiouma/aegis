import React from "react";
import ReactDOM from "react-dom/client";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { DocumentLangSync } from "./DocumentLangSync";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DocumentLangSync />
        <App />
      </AegisI18nProvider>
    </AegisThemeProvider>
  </React.StrictMode>,
);