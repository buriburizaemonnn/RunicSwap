import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface Account {
  'owner' : Principal,
  'subaccount' : [] | [Uint8Array | number[]],
}
export interface AddLiquidityArgs {
  'amount1_min' : bigint,
  'fee_per_vbytes' : [] | [bigint],
  'amount0_desired' : bigint,
  'amount0_min' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
  'amount1_desired' : bigint,
}
export interface AddLiquidityResult {
  'txids' : Array<SubmittedTransactionIdType>,
  'liquidity' : bigint,
}
export type BitcoinNetwork = { 'mainnet' : null } |
  { 'regtest' : null } |
  { 'testnet' : null };
export interface DepositAddresses {
  'account_identifier' : Uint8Array | number[],
  'account' : Account,
  'bitcoin' : string,
  'account_identifier_string' : string,
  'account_string' : string,
}
export interface InitArgs {
  'auth' : [] | [Principal],
  'commission_receiver_btc' : string,
  'commission_receiver_icp' : [] | [Uint8Array | number[]],
  'commission_receiver_principal' : [] | [Principal],
  'bitcoin_network' : BitcoinNetwork,
  'ord_canister' : Principal,
}
export interface RemoveLiquidityArgs {
  'amount1_min' : bigint,
  'amount0_min' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
}
export interface RemoveLiquidityResult {
  'liquidity_burned' : bigint,
  'txids' : Array<SubmittedTransactionIdType>,
}
export interface RuneId { 'tx' : number, 'block' : bigint }
export type SubmittedTransactionIdType = { 'Icp' : { 'txid' : bigint } } |
  { 'Bitcoin' : { 'txid' : string } };
export interface SwapArgs {
  'to' : [] | [Principal],
  'amount_out_min' : bigint,
  'token_in' : TokenType,
  'amount_in' : bigint,
  'token_out' : TokenType,
}
export interface SwapResult {
  'txids' : Array<SubmittedTransactionIdType>,
  'amount_received' : bigint,
}
export type TokenType = { 'Icp' : null } |
  { 'Runestone' : RuneId } |
  { 'Bitcoin' : null };
export type WithdrawalType = { 'Icp' : { 'to' : string, 'amount' : bigint } } |
  {
    'Runestone' : {
      'to' : string,
      'fee_per_vbytes' : [] | [bigint],
      'runeid' : RuneId,
      'amount' : bigint,
    }
  } |
  {
    'Bitcoin' : {
      'to' : string,
      'fee_per_vbytes' : [] | [bigint],
      'amount' : bigint,
    }
  };
export interface _SERVICE {
  'add_liquidity' : ActorMethod<[AddLiquidityArgs], AddLiquidityResult>,
  'get_balance_of_rune' : ActorMethod<[RuneId], bigint>,
  'get_deposit_addresses' : ActorMethod<[], DepositAddresses>,
  'remove_liquidity' : ActorMethod<
    [RemoveLiquidityArgs],
    RemoveLiquidityResult
  >,
  'swap_exact_tokens_for_tokens' : ActorMethod<[SwapArgs], SwapResult>,
  'test_rune_withdrawal_from_other_user' : ActorMethod<
    [Principal, RuneId, bigint],
    SubmittedTransactionIdType
  >,
  'withdraw' : ActorMethod<[WithdrawalType], SubmittedTransactionIdType>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
