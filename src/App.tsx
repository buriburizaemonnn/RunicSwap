import "./App.css";
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
} from "react-router-dom";
import Swap from "./routes/Swap";
import Pools from "./routes/Pools";
import Positions from "./routes/Positions";
import Vault from "./routes/Vault";
import Navbar from "./components/Navbar";
import { IdentityProvider } from "./contexts/IdentityContext";
import { useEffect, useState } from "react";
import { runeStore } from "./store/runeStore";

function App() {
  const [isRuneStoreInitialized, setIsRuneStoreInitialized] = useState(false);

  useEffect(() => {
    const initializeRuneStore = async () => {
      runeStore.getAllRunes();
      setIsRuneStoreInitialized(true);
    };

    initializeRuneStore();
  }, []);

  if (!isRuneStoreInitialized) {
    return <div>Loading...</div>;
  }
  return (
    <>
      <IdentityProvider>
        <Router>
          <div>
            <Navbar />
            <Routes>
              <Route path="/swap" element={<Swap />} />
              <Route path="/pools" element={<Pools />} />
              <Route path="/positions" element={<Positions />} />
              <Route path="/vault" element={<Vault />} />
              <Route path="/" element={<Navigate replace to="/swap" />} />
              {/* Catch-all route */}
              <Route path="*" element={<Navigate replace to="/swap" />} />
            </Routes>
          </div>
        </Router>
      </IdentityProvider>
    </>
  );
}

export default App;
