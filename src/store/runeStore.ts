import type {
  RuneId,
  TokenType,
} from "./../declarations/backend/backend.did.d.ts";

import { Rune } from "../types/rune.ts";

import { ord_indexer } from "../declarations/ord_indexer";

import type { CandidRuneEntry } from "./../declarations/ord_indexer/ord_indexer.did.d.ts";

class RuneStore {
  private runes: Rune[] = [];
  private initialized: boolean = false;

  constructor() {
    this.initializeRunes();
  }

  private async initializeRunes() {
    if (this.initialized) return;

    try {
      const runeEntries: CandidRuneEntry[] =
        await ord_indexer.get_50_rune_entries();

      this.runes = runeEntries.map((entry) =>
        this.convertCandidRuneToRune(entry),
      );
      this.initialized = true;
    } catch (error) {
      console.error("Failed to initialize runes:", error);
    }
  }

  private convertCandidRuneToRune(candidRune: CandidRuneEntry): Rune {
    return {
      name: candidRune.runename,
      runeid: {
        tx: candidRune.runeid.tx,
        block: candidRune.runeid.block,
      },
      divisibility: candidRune.divisibility,
      symbol:
        candidRune.symbol.length > 0 && typeof candidRune.symbol[0] === "number"
          ? String.fromCodePoint(candidRune.symbol[0])
          : undefined,
    };
  }

  getAllRunes(): Rune[] {
    return this.runes;
  }

  getRune(runeid: RuneId): Rune | undefined {
    return this.runes.find(
      (rune) =>
        rune.runeid.tx === runeid.tx && rune.runeid.block === runeid.block,
    );
  }

  getTokenType(rune: Rune): TokenType {
    return { Rune: rune.runeid };
  }

  getBitcoinTokenType(): TokenType {
    return { Bitcoin: null };
  }
}

export const runeStore = new RuneStore();
