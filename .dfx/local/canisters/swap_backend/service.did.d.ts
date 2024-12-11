import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface Account {
  'owner' : Principal,
  'subaccount' : [] | [Uint8Array | number[]],
}
export interface AddLiquidityArgs {
  'amount1_min' : bigint,
  'amount0_desired' : bigint,
  'amount0_min' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
  'amount1_desired' : bigint,
}
export interface Addresses {
  'icrc1_string' : string,
  'account_identifier' : Uint8Array | number[],
  'icrc1' : Account,
  'bitcoin' : string,
  'account_identifier_string' : string,
}
export type BitcoinNetwork = { 'mainnet' : null } |
  { 'regtest' : null } |
  { 'testnet' : null };
export interface CreatePairArgs { 'token0' : TokenType, 'token1' : TokenType }
export interface PoolInfoQuery {
  'reserve0' : bigint,
  'reserve1' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
  'pool_id' : bigint,
  'deposit_addresses' : Addresses,
}
export interface RuneId { 'tx' : number, 'block' : bigint }
export type SubmittedTxidType = { 'Ic' : { 'txid' : bigint } } |
  { 'Icrc1' : { 'txid' : bigint } } |
  { 'Bitcoin' : { 'txid' : string } };
export interface SwapArgs {
  'amount_out_min' : bigint,
  'token_in' : TokenType,
  'amount_in' : bigint,
  'token_out' : TokenType,
}
export interface SwapResult {
  'txids' : Array<SubmittedTxidType>,
  'amount_out' : bigint,
}
export type TokenType = { 'Icp' : null } |
  { 'Runestone' : RuneId } |
  { 'Bitcoin' : null } |
  { 'CkBTC' : null };
export interface _SERVICE {
  'add_liquidity' : ActorMethod<
    [AddLiquidityArgs],
    [bigint, Array<SubmittedTxidType>]
  >,
  'create_pair' : ActorMethod<[CreatePairArgs], bigint>,
  'get_combined_balance' : ActorMethod<
    [string, RuneId],
    Array<[TokenType, bigint]>
  >,
  'get_deposit_addresses' : ActorMethod<[], Addresses>,
  'get_user_balance' : ActorMethod<[], Array<[TokenType, bigint]>>,
  'pools' : ActorMethod<[], Array<PoolInfoQuery>>,
  'swap' : ActorMethod<[SwapArgs], SwapResult>,
  'test_combined_withdrawal' : ActorMethod<
    [RuneId, bigint, bigint, string],
    SubmittedTxidType
  >,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
