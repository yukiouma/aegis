import React from "react";
import ReactDOM from "react-dom/client";
import { AegisThemeProvider } from "@aegis/ui/theme";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AegisThemeProvider>
      <App />
    </AegisThemeProvider>
  </React.StrictMode>,
);
