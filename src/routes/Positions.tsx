import React, { useState, useEffect, useCallback } from "react";
import { runeStore } from "../store/runeStore";
import { Token } from "../types/swap";
import "./../styles/Position.css";
import { useIdentity } from "../contexts/IdentityContext";
import { canisterId, createActor } from "./../declarations/backend";
import { HttpAgent } from "@dfinity/agent";

import type {
  AddLiquidityResult,
  AddLiquidityArgs,
  RemoveLiquidityArgs,
  PositionDetail,
  CreatePairArgs,
  TokenType,
} from "../declarations/backend/backend.did.d.ts";

import {
  formatTokenAmount,
  parseTokenAmount,
  getTokenSymbol,
  getTokenName,
  getTokenType,
} from "../utils/tokenUtils";

const Positions: React.FC = () => {
  const { identity } = useIdentity();
  const [token0, setToken0] = useState<Token | null>(null);
  const [token1, setToken1] = useState<Token | null>(null);
  const [amount0, setAmount0] = useState("");
  const [amount1, setAmount1] = useState("");
  const [allTokens, setAllTokens] = useState<Token[]>([]);
  const [positions, setPositions] = useState<PositionDetail[]>([]);
  const [showMessage, setShowMessage] = useState(false);
  const [message, setMessage] = useState("");

  const createAgentAndActor = useCallback(async () => {
    if (!identity) {
      throw new Error("Identity not available");
    }
    const agent = await HttpAgent.create({ identity });
    const actor = createActor(canisterId, { agent });
    return { agent, actor };
  }, [identity]);

  useEffect(() => {
    const runes = runeStore.getAllRunes();
    const tokens: Token[] = [
      { type: "Bitcoin", data: null },
      ...runes.map((rune) => ({ type: "Rune" as const, data: rune })),
    ];
    setAllTokens(tokens);
    fetchPositions();
  }, [identity]);

  const fetchPositions = async () => {
    if (!identity) return;
    try {
      const { actor } = await createAgentAndActor();
      const fetchedPositions = await actor.get_positions();
      setPositions(fetchedPositions);
    } catch (error) {
      console.error("Failed to fetch positions:", error);
      setMessage("Failed to fetch positions. Please try again.");
      setShowMessage(true);
    }
  };

  const handleTokenSelect =
    (tokenSetter: React.Dispatch<React.SetStateAction<Token | null>>) =>
    (selectedToken: Token) => {
      tokenSetter(selectedToken);
    };

  const handleAmountChange =
    (amountSetter: React.Dispatch<React.SetStateAction<string>>) =>
    (e: React.ChangeEvent<HTMLInputElement>) => {
      amountSetter(e.target.value);
    };

  const handleAddLiquidity = async () => {
    if (!identity || !token0 || !token1 || !amount0 || !amount1) {
      alert("Please connect wallet, select tokens and enter amounts");
      return;
    }

    const token0Type = getTokenType(token0);
    const token1Type = getTokenType(token1);

    const args: AddLiquidityArgs = {
      amount1_min: BigInt(0),
      fee_per_vbytes: [],
      amount0_desired: parseTokenAmount(amount0, token0Type),
      amount0_min: BigInt(0),
      token0: token0Type,
      token1: token1Type,
      amount1_desired: parseTokenAmount(amount1, token1Type),
    };

    try {
      const { actor } = await createAgentAndActor();
      const result: AddLiquidityResult = await actor.add_liquidity(args);
      setMessage(
        `Liquidity added successfully. Liquidity: ${result.liquidity.toString()}`,
      );
      setShowMessage(true);
      setTimeout(() => setShowMessage(false), 5000);
      fetchPositions();
    } catch (error: any) {
      console.error("Add liquidity failed:", error);
      if (error.message.includes("ADD_LIQUIDITY_ERROR: Non-existing Pair")) {
        const createPair = window.confirm(
          "Pair doesn't exist. Do you want to create it?",
        );
        if (createPair) {
          await handleCreatePair(token0Type, token1Type);
        }
      } else {
        alert("Failed to add liquidity. Please try again.");
      }
    }
  };

  const handleCreatePair = async (token0: TokenType, token1: TokenType) => {
    try {
      const { actor } = await createAgentAndActor();
      const createPairArgs: CreatePairArgs = { token0, token1 };
      const poolId = await actor.create_pair(createPairArgs);
      setMessage(`Pair created successfully. Pool ID: ${poolId.toString()}`);
      setShowMessage(true);
      setTimeout(() => setShowMessage(false), 5000);
    } catch (error) {
      console.error("Create pair failed:", error);
      alert("Failed to create pair. Please try again.");
    }
  };

  const handleRemoveLiquidity = async (
    position: PositionDetail,
    liquidityToRemove: bigint,
  ) => {
    if (!identity) {
      alert("Please connect your wallet");
      return;
    }

    const args: RemoveLiquidityArgs = {
      amount1_min: BigInt(0),
      fee_per_vbytes: [],
      liquidity: liquidityToRemove,
      amount0_min: BigInt(0),
      token0: position.token0,
      token1: position.token1,
    };

    try {
      const { actor } = await createAgentAndActor();
      const result: AddLiquidityResult = await actor.remove_liquidity(args);
      const token0Name = getTokenName(position.token0);
      const token1Name = getTokenName(position.token1);
      setMessage(
        `${result.liquidity.toString()} liquidity was burned, ${formatTokenAmount(result.amount0, position.token0)} ${getTokenSymbol(position.token0)} ${token0Name} and ${formatTokenAmount(result.amount1, position.token1)} ${getTokenSymbol(position.token1)} ${token1Name} received.`,
      );
      setShowMessage(true);
      setTimeout(() => setShowMessage(false), 5000);
      fetchPositions();
    } catch (error) {
      console.error("Remove liquidity failed:", error);
      alert("Failed to remove liquidity. Please try again.");
    }
  };

  return (
    <div className="positions-container">
      <h2 className="section-title">Add Liquidity</h2>
      <div className="add-liquidity-box">
        <div className="liquidity-input">
          <input
            value={amount0}
            onChange={handleAmountChange(setAmount0)}
            placeholder="0.0"
          />
          <TokenSelector
            selectedToken={token0}
            onSelectToken={handleTokenSelect(setToken0)}
            tokens={allTokens}
          />
        </div>
        <div className="liquidity-input">
          <input
            value={amount1}
            onChange={handleAmountChange(setAmount1)}
            placeholder="0.0"
          />
          <TokenSelector
            selectedToken={token1}
            onSelectToken={handleTokenSelect(setToken1)}
            tokens={allTokens}
          />
        </div>
        <button className="add-liquidity-button" onClick={handleAddLiquidity}>
          Add Liquidity
        </button>
      </div>
      <h2 className="section-title">Your Liquidity Positions</h2>
      <div className="positions-list">
        {positions.length > 0 ? (
          positions.map((position, index) => (
            <PositionItem
              key={index}
              position={position}
              onRemoveLiquidity={handleRemoveLiquidity}
            />
          ))
        ) : (
          <p className="no-positions">No positions. Add liquidity to appear!</p>
        )}
      </div>
      {showMessage && (
        <div className="message-box">
          <p>{message}</p>
        </div>
      )}
    </div>
  );
};

interface PositionItemProps {
  position: PositionDetail;
  onRemoveLiquidity: (
    position: PositionDetail,
    liquidityToRemove: bigint,
  ) => void;
}

const PositionItem: React.FC<PositionItemProps> = ({
  position,
  onRemoveLiquidity,
}) => {
  const [liquidityToRemove, setLiquidityToRemove] = useState("0");

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const percentage = parseInt(e.target.value);
    const newLiquidity =
      (BigInt(position.liquidity_owned) * BigInt(percentage)) / BigInt(100);
    setLiquidityToRemove(newLiquidity.toString());
  };

  const percentageOwned = (
    (Number(position.liquidity_owned) / Number(position.total_liquidity)) *
    100
  ).toFixed(2);

  return (
    <div className="position-item">
      <div className="position-header">
        <div className="token-pair">
          <span className="token">
            {getTokenName(position.token0)} {getTokenSymbol(position.token0)}
          </span>
          <span className="token-separator">-</span>
          <span className="token">
            {getTokenName(position.token1)} {getTokenSymbol(position.token1)}
          </span>
        </div>
        <div className="pool-id">Pool ID: {position.pool_id.toString()}</div>
      </div>
      <div className="position-details">
        <div className="detail-row">
          <span className="detail-label">Amount Owned:</span>
          <span className="detail-value">
            {formatTokenAmount(position.amount0_owned, position.token0)}{" "}
            {getTokenSymbol(position.token0)}
          </span>
        </div>
        <div className="detail-row">
          <span className="detail-label">Amount Owned:</span>
          <span className="detail-value">
            {formatTokenAmount(position.amount1_owned, position.token1)}{" "}
            {getTokenSymbol(position.token1)}
          </span>
        </div>
        <div className="detail-row">
          <span className="detail-label">Liquidity Owned:</span>
          <span className="detail-value">
            {position.liquidity_owned.toString()} ({percentageOwned}%)
          </span>
        </div>
      </div>
      <div className="remove-liquidity-controls">
        <input
          type="range"
          min="0"
          max="100"
          value={(
            (Number(liquidityToRemove) / Number(position.liquidity_owned)) *
            100
          ).toString()}
          onChange={handleSliderChange}
        />
        <div className="remove-liquidity-info">
          <span className="remove-liquidity-amount">{liquidityToRemove}</span>
          <span className="remove-liquidity-percentage">
            (
            {(
              (Number(liquidityToRemove) / Number(position.liquidity_owned)) *
              100
            ).toFixed(2)}
            %)
          </span>
        </div>
        <button
          onClick={() => onRemoveLiquidity(position, BigInt(liquidityToRemove))}
        >
          Remove Liquidity
        </button>
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
          <span className="token-name-selected">
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

export default Positions;
