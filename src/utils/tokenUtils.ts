import { runeStore } from "../store/runeStore";

import type { TokenType } from "../declarations/backend/backend.did.d.ts";
import { Token } from "../types/swap.ts";

export function formatTokenAmount(
  amount: bigint,
  tokenType: TokenType,
): string {
  const divisibility = getTokenDivisibility(tokenType);
  const floatAmount = Number(amount) / Math.pow(10, divisibility);
  return floatAmount.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: divisibility,
  });
}

export function parseTokenAmount(amount: string, tokenType: TokenType): bigint {
  const divisibility = getTokenDivisibility(tokenType);
  const floatAmount = parseFloat(amount);
  return BigInt(Math.round(floatAmount * Math.pow(10, divisibility)));
}

export function getTokenSymbol(tokenType: TokenType): string {
  if ("Bitcoin" in tokenType) {
    return "₿";
  } else if ("Rune" in tokenType) {
    const rune = runeStore.getRune(tokenType.Rune);
    return rune?.symbol || "¤";
  }
  return "¤";
}

function getTokenDivisibility(tokenType: TokenType): number {
  if ("Bitcoin" in tokenType) {
    return 8;
  } else if ("Rune" in tokenType) {
    const rune = runeStore.getRune(tokenType.Rune);
    return rune?.divisibility || 0;
  }
  return 0;
}

export function getTokenName(tokenType: TokenType): string {
  if ("Bitcoin" in tokenType) {
    return "Bitcoin";
  } else if ("Rune" in tokenType) {
    const rune = runeStore.getRune(tokenType.Rune);
    return rune ? rune.name : "Unknown Rune";
  }
  return "Unknown Token";
}

export function getTokenType(token: Token): TokenType {
  if (token.type === "Bitcoin") {
    return { Bitcoin: null };
  } else if (token.data) {
    return { Rune: token.data.runeid };
  }
  throw new Error("Invalid token");
}
