/// <reference types="vite/client" />

interface Window {
  __ELIN_SHELL?: "toast" | "tray" | "main" | "workspace";
  __ELIN_WORKSPACE?: string;
}
