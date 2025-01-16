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
  'txids' : SubmittedTxidType,
  'liquidity' : bigint,
  'amount0' : bigint,
  'amount1' : bigint,
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
export interface InitArgs {
  'commission_receiver' : [] | [Principal],
  'auth' : [] | [Principal],
  'bitcoin_network' : BitcoinNetwork,
}
export interface PairInfoQuery {
  'reserve0' : bigint,
  'reserve1' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
  'no_of_holder' : number,
}
export interface PositionDetail {
  'total_liquidity' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
  'amount0_owned' : bigint,
  'pool_id' : bigint,
  'liquidity_owned' : bigint,
  'amount1_owned' : bigint,
}
export interface RemoveLiquidityArgs {
  'amount1_min' : bigint,
  'fee_per_vbytes' : [] | [bigint],
  'liquidity' : bigint,
  'amount0_min' : bigint,
  'token0' : TokenType,
  'token1' : TokenType,
}
export interface RuneId { 'tx' : number, 'block' : bigint }
export type SubmittedTxidType = { 'Bitcoin' : { 'txid' : string } };
export interface SwapExactTokensForTokensArgs {
  'fee_per_vbytes' : [] | [bigint],
  'amount_out_min' : bigint,
  'token_in' : TokenType,
  'amount_in' : bigint,
  'token_out' : TokenType,
}
export interface SwapResult {
  'txids' : SubmittedTxidType,
  'amount_out' : bigint,
  'amount_in' : bigint,
}
export type TokenType = { 'Rune' : RuneId } |
  { 'Bitcoin' : null };
export interface User { 'deposit_addresses' : Addresses, 'slippage' : number }
export type WithdrawalType = {
    'Rune' : {
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
  'create_pair' : ActorMethod<[CreatePairArgs], bigint>,
  'get_amount_out' : ActorMethod<[bigint, TokenType, TokenType], bigint>,
  'get_balances' : ActorMethod<[], Array<[TokenType, bigint]>>,
  'get_bitcoin_address' : ActorMethod<[], string>,
  'get_pairs' : ActorMethod<[], Array<PairInfoQuery>>,
  'get_positions' : ActorMethod<[], Array<PositionDetail>>,
  'get_user_info' : ActorMethod<[], User>,
  'remove_liquidity' : ActorMethod<[RemoveLiquidityArgs], AddLiquidityResult>,
  'swap_exact_tokens_for_tokens' : ActorMethod<
    [SwapExactTokensForTokensArgs],
    SwapResult
  >,
  'withdraw' : ActorMethod<[WithdrawalType], SubmittedTxidType>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
