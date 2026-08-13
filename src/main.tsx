import React from "react";
import ReactDOM from "react-dom/client";
import { detectLocale } from "./i18n";
import App from "./App";
import "./index.css";

function bootstrapShell() {
  const params = new URLSearchParams(window.location.search);
  const hash = new URLSearchParams(window.location.hash.replace(/^#/, ""));
  const shell = params.get("shell") ?? hash.get("shell");
  const workspace = params.get("workspace") ?? hash.get("workspace");
  if (shell === "toast" || shell === "tray" || shell === "workspace" || shell === "main") {
    window.__ELIN_SHELL = shell;
  }
  if (workspace) {
    window.__ELIN_SHELL = "workspace";
    window.__ELIN_WORKSPACE = workspace;
  }
  const kind = window.__ELIN_SHELL;
  if (kind === "toast" || kind === "tray" || kind === "workspace") {
    document.documentElement.dataset.shell = kind;
  }
  if (kind === "toast" || kind === "tray") {
    document.documentElement.style.background = "transparent";
  } else {
    document.documentElement.style.background = "#0b0a12";
  }
  document.documentElement.lang = detectLocale();
}

bootstrapShell();

document.addEventListener("contextmenu", (event) => event.preventDefault());
document.addEventListener("keydown", (event) => {
  if (event.key === "F12") event.preventDefault();
  if (event.ctrlKey && event.shiftKey && ["I", "J", "C"].includes(event.key)) {
    event.preventDefault();
  }
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
