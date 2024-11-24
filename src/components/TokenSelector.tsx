import "./../styles/TokenSelector.css";
import { runeStore } from "../store/RuneStore";
import { RuneId } from "../types";
import React, { useState, useEffect, useRef } from "react";

export type TokenType =
  | { Icp: null }
  | { Runestone: RuneId }
  | { Bitcoin: null }
  | { CkBTC: null };

interface TokenOption {
  name: string;
  type: TokenType;
  tag?: string;
}

interface TokenSelectorProps {
  onSelect: (token: TokenType) => void;
  otherSelectedToken?: TokenType; // To prevent selecting the same token
}

export const TokenSelector: React.FC<TokenSelectorProps> = ({
  onSelect,
  otherSelectedToken,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [selectedToken, setSelectedToken] = useState<TokenOption | null>(null);
  const [allTokens, setAllTokens] = useState<TokenOption[]>([]);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Initialize base tokens
    const baseTokens: TokenOption[] = [
      { name: "Bitcoin", type: { Bitcoin: null } },
      { name: "ICP", type: { Icp: null } },
      { name: "CKBTC", type: { CkBTC: null }, tag: "icrc1" },
    ];

    // Add Runestone tokens from store
    const runesMap = runeStore.getRunes();
    const runeTokens: TokenOption[] = Array.from(runesMap.entries()).map(
      ([name, entry]) => ({
        name,
        type: { Runestone: entry.runeId },
        tag: "rune",
      }),
    );

    setAllTokens([...baseTokens, ...runeTokens]);
  }, []);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const formatRuneName = (name: string): string => {
    return name.replace(/[•]/g, "").toLowerCase();
  };

  const filterTokens = () => {
    if (!search) return allTokens;

    return allTokens.filter((token) => {
      const searchTerm = search.toLowerCase();
      const tokenName = formatRuneName(token.name);
      return tokenName.includes(searchTerm);
    });
  };

  const handleSelect = (token: TokenOption) => {
    setSelectedToken(token);
    onSelect(token.type);
    setIsOpen(false);
    setSearch("");
  };

  const isTokenDisabled = (token: TokenOption) => {
    if (!otherSelectedToken) return false;

    const tokenType = Object.keys(token.type)[0];
    const otherTokenType = Object.keys(otherSelectedToken)[0];

    if (tokenType !== otherTokenType) return false;

    if (tokenType === "Runestone") {
      const runeId = (token.type as { Runestone: RuneId }).Runestone;
      const otherRuneId = (otherSelectedToken as { Runestone: RuneId })
        .Runestone;
      return runeId.tx === otherRuneId.tx && runeId.block === otherRuneId.block;
    }

    return tokenType === otherTokenType;
  };

  return (
    <div className="token-input-container" ref={dropdownRef}>
      <button className="token-button" onClick={() => setIsOpen(!isOpen)}>
        {selectedToken ? selectedToken.name : "Select Token"}
      </button>

      {isOpen && (
        <div className="token-dropdown">
          <input
            className="dropdown-search"
            type="text"
            placeholder="Search tokens..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            autoFocus
          />
          <div>
            {filterTokens().map((token) => (
              <div
                key={token.name}
                className="token-option"
                onClick={() => !isTokenDisabled(token) && handleSelect(token)}
                style={{
                  opacity: isTokenDisabled(token) ? 0.5 : 1,
                  cursor: isTokenDisabled(token) ? "not-allowed" : "pointer",
                }}
              >
                <span>{token.name}</span>
                {token.tag && <span className="token-tag">{token.tag}</span>}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
