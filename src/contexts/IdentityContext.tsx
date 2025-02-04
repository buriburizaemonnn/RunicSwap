import React, { createContext, useContext, useState, useEffect } from "react";
import { AuthClient } from "@dfinity/auth-client";
import { Identity } from "@dfinity/agent";
import { useSiwbIdentity } from "ic-siwb-lasereyes-connector";

type IdentityType = "II" | "SIWB" | null;

interface IdentityContextType {
  identity: Identity | null;
  identityType: IdentityType;
  setIdentity: (identity: Identity | null, type: IdentityType) => void;
  isAuthenticated: boolean;
  loginII: (identityProvider?: string) => Promise<void>;
  loginSIWB: () => Promise<void>;
  logout: () => Promise<void>;
}

const IdentityContext = createContext<IdentityContextType | undefined>(
  undefined,
);

export const useIdentity = () => {
  const context = useContext(IdentityContext);
  if (!context) {
    throw new Error("useIdentity must be used within an IdentityProvider");
  }
  return context;
};

export const IdentityProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [identity, setIdentityState] = useState<Identity | null>(null);
  const [identityType, setIdentityType] = useState<IdentityType>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  const {
    login: siwbLogin,
    identity: siwbIdentity,
    clear: siwbClear,
  } = useSiwbIdentity();

  useEffect(() => {
    const checkAuth = async () => {
      const authClient = await AuthClient.create();
      const isIIAuthenticated = await authClient.isAuthenticated();
      if (isIIAuthenticated) {
        setIdentityState(authClient.getIdentity());
        setIdentityType("II");
        setIsAuthenticated(true);
      } else if (siwbIdentity) {
        setIdentityState(siwbIdentity);
        setIdentityType("SIWB");
        setIsAuthenticated(true);
      }
    };
    checkAuth();
  }, [siwbIdentity]);

  const setIdentity = (newIdentity: Identity | null, type: IdentityType) => {
    setIdentityState(newIdentity);
    setIdentityType(type);
    setIsAuthenticated(!!newIdentity);
  };

  const loginII = async (identityProvider?: string) => {
    const authClient = await AuthClient.create();
    const iiUrl =
      identityProvider ||
      `http://${process.env.CANISTER_ID_INTERNET_IDENTITY}.localhost:4943/`;
    await new Promise<void>((resolve, reject) => {
      authClient.login({
        identityProvider: iiUrl,
        onSuccess: () => {
          setIdentity(authClient.getIdentity(), "II");
          resolve();
        },
        onError: reject,
      });
    });
  };

  const loginSIWB = async () => {
    const result = await siwbLogin();
    if (result && siwbIdentity) {
      setIdentity(siwbIdentity, "SIWB");
    } else {
      console.error("SIWB login failed or identity is undefined");
    }
  };

  const logout = async () => {
    if (identityType === "II") {
      const authClient = await AuthClient.create();
      await authClient.logout();
    } else if (identityType === "SIWB") {
      siwbClear();
    }
    setIdentity(null, null);
  };

  return (
    <IdentityContext.Provider
      value={{
        identity,
        identityType,
        setIdentity,
        isAuthenticated,
        loginII,
        loginSIWB,
        logout,
      }}
    >
      {children}
    </IdentityContext.Provider>
  );
};
