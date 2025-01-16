import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface CandidRuneEntry {
  'id' : bigint,
  'runeid' : CandidRuneId,
  'divisibility' : number,
  'block' : bigint,
  'runename' : string,
  'symbol' : [] | [number],
}
export interface CandidRuneId { 'tx' : number, 'block' : bigint }
export type MintError = { 'Cap' : bigint } |
  { 'End' : bigint } |
  { 'Start' : bigint } |
  { 'Unmintable' : null };
export type OrdError = { 'Rpc' : RpcError } |
  { 'Overflow' : null } |
  { 'Params' : string } |
  { 'NotEnoughConfirmations' : null } |
  { 'Index' : MintError } |
  { 'Unrecoverable' : null } |
  { 'BlockVerification' : number } |
  { 'OutPointNotFound' : null } |
  { 'Recoverable' : { 'height' : number, 'depth' : number } };
export type Result = { 'Ok' : null } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : [number, string] } |
  { 'Err' : OrdError };
export type Result_2 = { 'Ok' : Array<RuneBalance> } |
  { 'Err' : OrdError };
export type RpcError = { 'Io' : [string, string, string] } |
  { 'Endpoint' : [string, string, string] } |
  { 'Decode' : [string, string, string] };
export interface RuneBalance { 'id' : RuneId, 'balance' : bigint }
export interface RuneId { 'tx' : number, 'block' : bigint }
export interface _SERVICE {
  'admin_set_url' : ActorMethod<[string], Result>,
  'get_50_rune_entries' : ActorMethod<[], Array<CandidRuneEntry>>,
  'get_height' : ActorMethod<[], Result_1>,
  'get_rune_entry_by_runeid' : ActorMethod<
    [CandidRuneId],
    [] | [CandidRuneEntry]
  >,
  'get_runes_by_utxo' : ActorMethod<[string, number], Result_2>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
