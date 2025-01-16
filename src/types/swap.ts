import { Rune } from "./rune";
import type { SwapResult } from "../declarations/backend/backend.did.d.ts";

export interface Token {
  type: "Bitcoin" | "Rune";
  data: null | Rune;
}

export interface SwapState {
  fromToken: Token | null;
  toToken: Token | null;
  fromAmount: string;
  toAmount: string;
  swapResult: SwapResult | null;
}
