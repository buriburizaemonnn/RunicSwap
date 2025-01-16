import type { RuneId } from "../declarations/backend/backend.did.d.ts";

export interface Rune {
  name: string;
  runeid: RuneId;
  divisibility: number;
  symbol?: string;
}
