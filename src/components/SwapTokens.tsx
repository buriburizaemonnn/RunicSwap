import React, { useState, useEffect, useCallback } from "react";
import { runeStore } from "../store/runeStore";
import { SwapState, Token } from "../types/swap";
import {
  formatTokenAmount,
  parseTokenAmount,
  getTokenSymbol,
  getTokenType,
} from "../utils/tokenUtils";

import type {
  SwapExactTokensForTokensArgs,
  SwapResult,
} from "../declarations/backend/backend.did.d.ts";

import {
  canisterId,
  createActor,
  backend as swapBackend,
} from "./../declarations/backend";

import "./../styles/Swap.css";
import { useIdentity } from "../contexts/IdentityContext";
import { HttpAgent } from "@dfinity/agent";

const SwapTokens: React.FC = () => {
  const { identity } = useIdentity();
  const [swapState, setSwapState] = useState<SwapState>({
    fromToken: null,
    toToken: null,
    fromAmount: "",
    toAmount: "",
    swapResult: null,
  });
  const [showMessage, setShowMessage] = useState(false);
  const [allTokens, setAllTokens] = useState<Token[]>([]);

  useEffect(() => {
    const fetchRunes = async () => {
      try {
        const runes = runeStore.getAllRunes();
        const tokens: Token[] = [
          { type: "Bitcoin", data: null },
          ...runes.map((rune) => ({ type: "Rune" as const, data: rune })),
        ];
        setAllTokens(tokens);
      } catch (error) {
        console.error("Error fetching runes:", error);
      }
    };

    fetchRunes();
  }, []);

  const handleTokenSelect =
    (tokenType: "from" | "to") => (selectedToken: Token) => {
      setSwapState((prev) => {
        const otherTokenType = tokenType === "from" ? "toToken" : "fromToken";
        if (
          prev[otherTokenType] &&
          prev[otherTokenType]?.type === selectedToken.type &&
          (selectedToken.type === "Bitcoin" ||
            (selectedToken.type === "Rune" &&
              prev[otherTokenType]?.data?.runeid.tx ===
                selectedToken.data?.runeid.tx))
        ) {
          return prev;
        }
        return {
          ...prev,
          [tokenType === "from" ? "fromToken" : "toToken"]: selectedToken,
          fromAmount: "",
          toAmount: "",
        };
      });
    };

  const handleAmountChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const newAmount = e.target.value;
    setSwapState((prev) => ({ ...prev, fromAmount: newAmount }));

    if (swapState.fromToken && swapState.toToken && newAmount) {
      try {
        const fromTokenType = getTokenType(swapState.fromToken);
        const toTokenType = getTokenType(swapState.toToken);
        const amountIn = parseTokenAmount(newAmount, fromTokenType);
        const amountOut = await swapBackend.get_amount_out(
          amountIn,
          fromTokenType,
          toTokenType,
        );
        setSwapState((prev) => ({
          ...prev,
          toAmount: formatTokenAmount(amountOut, toTokenType),
        }));
      } catch (error) {
        console.error("Error calculating swap amount:", error);
      }
    }
  };

  const handleSwapTokens = async () => {
    setSwapState((prev) => {
      const newState = {
        ...prev,
        fromToken: prev.toToken,
        toToken: prev.fromToken,
        fromAmount: prev.toAmount,
        toAmount: "",
      };
      return newState;
    });

    // Recalculate the amount after swapping tokens
    if (swapState.toToken && swapState.fromToken && swapState.toAmount) {
      try {
        const fromTokenType = getTokenType(swapState.toToken);
        const toTokenType = getTokenType(swapState.fromToken);
        const amountIn = parseTokenAmount(swapState.toAmount, fromTokenType);
        const amountOut = await swapBackend.get_amount_out(
          amountIn,
          fromTokenType,
          toTokenType,
        );
        setSwapState((prev) => ({
          ...prev,
          toAmount: formatTokenAmount(amountOut, toTokenType),
        }));
      } catch (error) {
        console.error("Error calculating swap amount after token swap:", error);
      }
    }
  };

  const executeSwap = useCallback(
    async (args: SwapExactTokensForTokensArgs): Promise<SwapResult> => {
      if (!identity) {
        throw new Error("Please connect your wallet first");
      }
      if (!canisterId) {
        throw new Error("Canister ID is not defined");
      }

      const agent = await HttpAgent.create({ identity });
      const actor = createActor(canisterId, {
        agent,
      });
      return await actor.swap_exact_tokens_for_tokens(args);
    },
    [identity],
  );

  const handleSwap = async () => {
    if (!identity) {
      alert("Please connect your wallet first");
      return;
    }

    if (!swapState.fromToken || !swapState.toToken || !swapState.fromAmount) {
      alert("Please select tokens and enter an amount");
      return;
    }

    const fromTokenType = getTokenType(swapState.fromToken);
    const toTokenType = getTokenType(swapState.toToken);

    const args: SwapExactTokensForTokensArgs = {
      fee_per_vbytes: [],
      amount_out_min: BigInt(0),
      token_in: fromTokenType,
      amount_in: parseTokenAmount(swapState.fromAmount, fromTokenType),
      token_out: toTokenType,
    };

    try {
      const result = await executeSwap(args);
      setSwapState((prev) => ({ ...prev, swapResult: result }));
      setShowMessage(true);
      setTimeout(() => setShowMessage(false), 5000);
    } catch (error) {
      console.error("Swap failed:", error);
      alert("Swap failed. Please try again.");
    }
  };

  return (
    <div className="swap-container">
      <h2>Swap</h2>
      <div className="swap-box">
        <div className="swap-input">
          <input
            value={swapState.fromAmount}
            onChange={handleAmountChange}
            placeholder="0.0"
          />
          <TokenSelector
            selectedToken={swapState.fromToken}
            onSelectToken={handleTokenSelect("from")}
            tokens={allTokens}
          />
        </div>
        <div className="swap-arrow-container" onClick={handleSwapTokens}>
          <div className="swap-arrow">↓</div>
        </div>
        <div className="swap-input">
          <input value={swapState.toAmount} readOnly placeholder="0.0" />
          <TokenSelector
            selectedToken={swapState.toToken}
            onSelectToken={handleTokenSelect("to")}
            tokens={allTokens}
          />
        </div>
        <button
          className="swap-button"
          onClick={handleSwap}
          disabled={!identity}
        >
          {identity ? "Swap" : "Connect Wallet to Swap"}
        </button>
        {showMessage && swapState.swapResult && (
          <div className="message-box">
            <p>
              Swap successful.{" "}
              {formatTokenAmount(
                swapState.swapResult.amount_out,
                getTokenType(swapState.toToken!),
              )}{" "}
              {getTokenSymbol(getTokenType(swapState.toToken!))} received.
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

interface TokenSelectorProps {
  selectedToken: Token | null;
  onSelectToken: (token: Token) => void;
  tokens: Token[];
}

const TokenSelector: React.FC<TokenSelectorProps> = ({
  selectedToken,
  onSelectToken,
  tokens,
}) => {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div className="token-selector">
      <div className="selected-token" onClick={() => setIsOpen(!isOpen)}>
        {selectedToken ? (
          <span className="token-name">
            {selectedToken.type === "Bitcoin"
              ? "Bitcoin"
              : selectedToken.data?.name}{" "}
            {getTokenSymbol(getTokenType(selectedToken))}
          </span>
        ) : (
          "Select a token"
        )}
      </div>
      {isOpen && (
        <div className="token-list">
          {tokens.map((token, index) => (
            <div
              key={index}
              className="token-option"
              onClick={() => {
                onSelectToken(token);
                setIsOpen(false);
              }}
            >
              <span className="token-name">
                {token.type === "Bitcoin" ? "Bitcoin" : token.data?.name}{" "}
                {getTokenSymbol(getTokenType(token))}
              </span>
              <span className="token-tag" data-type={token.type}>
                {token.type}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SwapTokens;
