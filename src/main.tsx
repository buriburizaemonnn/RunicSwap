import "./index.css";
import App from "./App.tsx";

import ReactDOM from "react-dom/client";
import { LaserEyesProvider, TESTNET4 } from "@omnisat/lasereyes";
import { SiwbIdentityProvider } from "ic-siwb-lasereyes-connector";

import { idlFactory as siwbIdl } from "./../ic_siwb/ic_siwb_provider.idl.ts";
import type { _SERVICE as siwbService } from "./../ic_siwb/ic_siwb_provider.d.ts";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <LaserEyesProvider config={{ network: TESTNET4 }}>
    <SiwbIdentityProvider<siwbService>
      canisterId={"mwm4a-eiaaa-aaaah-aebnq-cai"}
      idlFactory={siwbIdl}
      httpAgentOptions={{
        host:
          process.env.DFX_NETWORK === "ic"
            ? "https://icp0.io"
            : "http://127.0.0.1:8080",
      }}
    >
      <App />
    </SiwbIdentityProvider>
  </LaserEyesProvider>,
);
