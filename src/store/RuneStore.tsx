import { RuneId, RuneEntry } from "../types";
import { CandidRuneEntry } from "../declarations/ord_indexer/ord_indexer.did";
import { ord_indexer } from "../declarations/ord_indexer";

class RuneStore {
  private static instance: RuneStore;
  private runesMap: Map<string, RuneEntry>;
  private initialized: boolean = false;

  private constructor() {
    this.runesMap = new Map<string, RuneEntry>();
  }

  public static getInstance(): RuneStore {
    if (!RuneStore.instance) {
      RuneStore.instance = new RuneStore();
    }
    return RuneStore.instance;
  }

  private convertCandidToRuneEntry(
    candidEntry: CandidRuneEntry,
  ): [string, RuneEntry] {
    const runeId: RuneId = {
      tx: candidEntry.runeid.tx,
      block: candidEntry.runeid.block,
    };

    const runeEntry: RuneEntry = {
      runeId,
      decimal: candidEntry.divisibility,
      symbol: candidEntry.symbol,
    };

    return [candidEntry.runename, runeEntry];
  }

  public async initialize(): Promise<void> {
    if (this.initialized) return;

    try {
      const response: CandidRuneEntry[] =
        await this.fetchRunesFromSmartContract();

      response.forEach((candidEntry) => {
        const [runename, runeEntry] =
          this.convertCandidToRuneEntry(candidEntry);
        this.runesMap.set(runename, runeEntry);
      });

      this.initialized = true;
    } catch (error) {
      console.error("Failed to initialize rune store:", error);
      throw error;
    }
  }

  private async fetchRunesFromSmartContract(): Promise<CandidRuneEntry[]> {
    return ord_indexer.get_50_rune_entries();
  }

  public getRunes(): Map<string, RuneEntry> {
    return this.runesMap;
  }

  public getRune(runename: string): RuneEntry | undefined {
    return this.runesMap.get(runename);
  }
}

// Initialize store when the app starts
export const runeStore = RuneStore.getInstance();

export async function initializeStore(): Promise<void> {
  await runeStore.initialize();
}
