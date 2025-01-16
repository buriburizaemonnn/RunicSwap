import type { TokenType } from "./../declarations/backend/backend.did.d.ts";

export interface LiquidityPosition {
  id: string;
  token0: TokenType;
  token1: TokenType;
  amount0: bigint;
  amount1: bigint;
  liquidity: bigint;
}
