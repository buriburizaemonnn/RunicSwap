import React, { useCallback, useEffect, useState } from "react";
import {
  formatTokenAmount,
  getTokenSymbol,
  getTokenName,
} from "../utils/tokenUtils";
import "../styles/Vault.css";
import type { TokenType } from "../declarations/backend/backend.did.d.ts";
import { HttpAgent } from "@dfinity/agent";
import { createActor, canisterId } from "../declarations/backend";
import { useIdentity } from "../contexts/IdentityContext.tsx";

const Vault: React.FC = () => {
  const [bitcoinAddress, setBitcoinAddress] = useState<string | null>(null);
  const [balances, setBalances] = useState<Array<[TokenType, bigint]>>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const { identity } = useIdentity();

  const createAgentAndActor = useCallback(async () => {
    if (!identity) {
      throw new Error("Identity not available");
    }
    const agent = await HttpAgent.create({ identity });
    const actor = createActor(canisterId, { agent });
    return { agent, actor };
  }, [identity]);

  const get_bitcoin_address = async () => {
    try {
      const { actor } = await createAgentAndActor();
      const bitcoin_address = await actor.get_bitcoin_address();
      setBitcoinAddress(bitcoin_address);
      setError(null);
    } catch (error) {
      console.error("Failed to get bitcoin address: ", error);
      setError("Failed to get bitcoin address. Please try again.");
    }
  };

  const get_balances = async () => {
    if (!identity) return;
    try {
      const { actor } = await createAgentAndActor();
      const balances = await actor.get_balances();
      setBalances(balances);
      setError(null);
    } catch (error) {
      console.error("Failed to get balances: ", error);
      setError("Failed to get balances. Please try again.");
    }
  };

  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        await Promise.all([get_bitcoin_address(), get_balances()]);
        /*
        setBitcoinAddress(address);
        setBalances(balancesData);
        setError(null);
	*/
      } catch (err) {
        console.error("Error fetching vault data:", err);
        setError("Failed to fetch vault data. Please try again later.");
      } finally {
        setLoading(false);
      }
    };

    if (identity) {
      fetchData();
    }
  }, [identity]);

  if (!identity) {
    return (
      <div className="vault-container">
        Please connect your wallet to view your vault.
      </div>
    );
  }

  if (loading) {
    return <div className="vault-container">Loading vault data...</div>;
  }

  if (error) {
    return <div className="vault-container">{error}</div>;
  }

  return (
    <div className="vault-container">
      <h1 className="vault-title">Your Vault</h1>
      <div className="vault-section">
        <h2 className="vault-subtitle">Bitcoin Deposit Address</h2>
        <div className="bitcoin-address">{bitcoinAddress}</div>
      </div>
      <div className="vault-section">
        <h2 className="vault-subtitle">Token Balances</h2>
        <ul className="balance-list">
          {balances.map(([tokenType, balance], index) => (
            <li key={index} className="balance-item">
              <span className="token-name">
                {getTokenName(tokenType)} {getTokenSymbol(tokenType)}
              </span>
              <span className="token-balance">
                {formatTokenAmount(balance, tokenType)}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
};

export default Vault;
