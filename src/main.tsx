import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import Navbar from "./components/Navbar.tsx";

const handleConnectWallet = () => {
  console.log("Connecting wallet...");
  // Implement wallet connection logic here
};
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Navbar onConnectWallet={handleConnectWallet} />
    <App />
  </StrictMode>,
);
