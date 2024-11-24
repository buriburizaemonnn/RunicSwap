import { BrowserRouter, Route, Routes } from "react-router-dom";
import Navbar from "./components/Navbar";
import { Swap } from "./routes/Swap";
import { Pools } from "./routes/Pools";
import { Positions } from "./routes/Positions";
import { initializeStore } from "./store/RuneStore";
import { useEffect } from "react";

function App() {
  useEffect(() => {
    // initializeStore().catch(console.error);
  }, []);
  return (
    <>
      <BrowserRouter>
        <Navbar />
        <Routes>
          <Route path="/" element={<Swap />} />
          <Route path="/pools" element={<Pools />} />
          <Route path="/positions" element={<Positions />} />
        </Routes>
      </BrowserRouter>
    </>
  );
}

export default App;
