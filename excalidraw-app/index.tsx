import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import ExcalidrawApp from "./App";
import "./index.scss";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Excalidraw root element was not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <ExcalidrawApp />
  </StrictMode>,
);
