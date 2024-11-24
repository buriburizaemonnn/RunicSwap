// import { StrictMode } from 'react'
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { SiwbIdentityProvider } from "ic-use-siwb-identity";

createRoot(document.getElementById("root")!).render(
  // <StrictMode>
  <SiwbIdentityProvider
  // TODO
  >
    <App />
  </SiwbIdentityProvider>,
  // </StrictMode>,
);
