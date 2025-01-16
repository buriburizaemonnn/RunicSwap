import { useEffect, useState } from "react";
import { UNISAT, useLaserEyes, WIZZ, XVERSE } from "@omnisat/lasereyes";
import { useIdentity } from "../contexts/IdentityContext";
import "./../styles/ConnectDialog.css";
import { useSiwbIdentity } from "ic-siwb-lasereyes-connector";

interface ConnectDialogProps {
  isOpen: boolean;
  setIsOpen: (isOpen: boolean) => void;
}

export default function ConnectDialog({
  isOpen,
  setIsOpen,
}: ConnectDialogProps) {
  const p = useLaserEyes();
  const { loginII, loginSIWB, identity } = useIdentity();
  const { setLaserEyes } = useSiwbIdentity();

  const [loading, setLoading] = useState<boolean>(false);

  useEffect(() => {
    if (identity) {
      setIsOpen(false);
    }
  }, [identity, setIsOpen]);

  const handleWalletClick = async (
    wallet: typeof WIZZ | typeof UNISAT | typeof XVERSE,
  ) => {
    setLoading(true);
    try {
      await setLaserEyes(p, wallet);
      await loginSIWB();
    } catch (error) {
      console.error("SIWB login error:", error);
    } finally {
      setLoading(false);
    }
  };

  const handleInternetIdentityClick = async () => {
    setLoading(true);
    try {
      await loginII();
    } catch (error) {
      console.error("Internet Identity login error:", error);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="connect-dialog-overlay" onClick={() => setIsOpen(false)}>
      <div className="connect-dialog" onClick={(e) => e.stopPropagation()}>
        <h2 className="connect-dialog-title">Select Wallet</h2>
        <div className="connect-dialog-buttons">
          <button
            className="connect-dialog-button"
            onClick={() => handleWalletClick(WIZZ)}
            disabled={loading}
          >
            Wizz Wallet
          </button>
          <button
            className="connect-dialog-button"
            onClick={() => handleWalletClick(UNISAT)}
            disabled={loading}
          >
            Unisat Wallet
          </button>
          <button
            className="connect-dialog-button"
            onClick={() => handleWalletClick(XVERSE)}
            disabled={loading}
          >
            Xverse Wallet
          </button>
          <button
            className="connect-dialog-button"
            onClick={handleInternetIdentityClick}
            disabled={loading}
          >
            Internet Identity
          </button>
        </div>
        {loading && (
          <div className="connect-dialog-loading">
            <div className="connect-dialog-spinner"></div>
            <p>Connecting...</p>
          </div>
        )}
      </div>
    </div>
  );
}
