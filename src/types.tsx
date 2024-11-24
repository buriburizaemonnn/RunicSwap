export interface RuneId {
  tx: number;
  block: bigint;
}

export interface RuneEntry {
  decimal: number;
  runeId: RuneId;
  symbol: [] | [number];
}
