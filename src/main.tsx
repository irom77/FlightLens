import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyTheme, useTheme } from "./stores/theme";
import "./styles.css";
// Apply the stored theme before the first paint so the app never flashes.
applyTheme(useTheme.getState().theme);
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
