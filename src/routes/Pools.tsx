import React, { useState, useEffect } from "react";
import "./../styles/Pools.css";

import {
  getTokenName,
  getTokenSymbol,
  formatTokenAmount,
} from "../utils/tokenUtils";

import type { PairInfoQuery } from "../declarations/backend/backend.did";
import { backend } from "../declarations/backend";

const Pools: React.FC = () => {
  const [pairs, setPairs] = useState<PairInfoQuery[]>([]);

  async function getPairs(): Promise<PairInfoQuery[]> {
    return await backend.get_pairs();
  }

  useEffect(() => {
    const fetchPairs = async () => {
      try {
        const pairsData = await getPairs();
        setPairs(pairsData);
      } catch (error) {
        console.error("Failed to fetch pairs:", error);
      }
    };

    fetchPairs();
  }, []);

  return (
    <div className="pools-container">
      <h1 className="pools-title">Liquidity Pools</h1>
      <div className="pools-list">
        {pairs.map((pair, index) => (
          <PoolCard key={index} pair={pair} />
        ))}
      </div>
    </div>
  );
};

interface PoolCardProps {
  pair: PairInfoQuery;
}

const PoolCard: React.FC<PoolCardProps> = ({ pair }) => {
  const token0Name = getTokenName(pair.token0);
  const token1Name = getTokenName(pair.token1);

  return (
    <div className="pool-card">
      <div className="pool-card-header">
        <div className="token-pair">
          <span className="token">
            {token0Name} {getTokenSymbol(pair.token0)}
          </span>
          <span className="token-separator">/</span>
          <span className="token">
            {token1Name} {getTokenSymbol(pair.token1)}
          </span>
        </div>
      </div>
      <div className="pool-card-body">
        <div className="pool-info">
          <div className="info-column">
            <div className="info-item">
              <span className="info-label">Reserve {token0Name}:</span>
              <span className="info-value">
                {formatTokenAmount(pair.reserve0, pair.token0)}{" "}
                {getTokenSymbol(pair.token0)}
              </span>
            </div>
            <div className="info-item">
              <span className="info-label">Reserve {token1Name}:</span>
              <span className="info-value">
                {formatTokenAmount(pair.reserve1, pair.token1)}{" "}
                {getTokenSymbol(pair.token1)}
              </span>
            </div>
          </div>
          <div className="info-column">
            <div className="info-item">
              <span className="info-label">Number of Holders:</span>
              <span className="info-value">{pair.no_of_holder}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Pools;
