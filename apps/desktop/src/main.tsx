import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./DesktopApp";
import "./desktop-redesign.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
