export const idlFactory = ({ IDL }) => {
  const BitcoinNetwork = IDL.Variant({
    'mainnet' : IDL.Null,
    'regtest' : IDL.Null,
    'testnet' : IDL.Null,
  });
  const InitArgs = IDL.Record({
    'commission_receiver' : IDL.Opt(IDL.Principal),
    'auth' : IDL.Opt(IDL.Principal),
    'bitcoin_network' : BitcoinNetwork,
  });
  const RuneId = IDL.Record({ 'tx' : IDL.Nat32, 'block' : IDL.Nat64 });
  const TokenType = IDL.Variant({ 'Rune' : RuneId, 'Bitcoin' : IDL.Null });
  const AddLiquidityArgs = IDL.Record({
    'amount1_min' : IDL.Nat64,
    'fee_per_vbytes' : IDL.Opt(IDL.Nat64),
    'amount0_desired' : IDL.Nat64,
    'amount0_min' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
    'amount1_desired' : IDL.Nat64,
  });
  const SubmittedTxidType = IDL.Variant({
    'Bitcoin' : IDL.Record({ 'txid' : IDL.Text }),
  });
  const AddLiquidityResult = IDL.Record({
    'txids' : SubmittedTxidType,
    'liquidity' : IDL.Nat64,
    'amount0' : IDL.Nat64,
    'amount1' : IDL.Nat64,
  });
  const CreatePairArgs = IDL.Record({
    'token0' : TokenType,
    'token1' : TokenType,
  });
  const PairInfoQuery = IDL.Record({
    'reserve0' : IDL.Nat64,
    'reserve1' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
    'no_of_holder' : IDL.Nat32,
  });
  const PositionDetail = IDL.Record({
    'total_liquidity' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
    'amount0_owned' : IDL.Nat64,
    'pool_id' : IDL.Nat,
    'liquidity_owned' : IDL.Nat64,
    'amount1_owned' : IDL.Nat64,
  });
  const Account = IDL.Record({
    'owner' : IDL.Principal,
    'subaccount' : IDL.Opt(IDL.Vec(IDL.Nat8)),
  });
  const Addresses = IDL.Record({
    'icrc1_string' : IDL.Text,
    'account_identifier' : IDL.Vec(IDL.Nat8),
    'icrc1' : Account,
    'bitcoin' : IDL.Text,
    'account_identifier_string' : IDL.Text,
  });
  const User = IDL.Record({
    'deposit_addresses' : Addresses,
    'slippage' : IDL.Nat8,
  });
  const RemoveLiquidityArgs = IDL.Record({
    'amount1_min' : IDL.Nat64,
    'fee_per_vbytes' : IDL.Opt(IDL.Nat64),
    'liquidity' : IDL.Nat64,
    'amount0_min' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
  });
  const SwapExactTokensForTokensArgs = IDL.Record({
    'fee_per_vbytes' : IDL.Opt(IDL.Nat64),
    'amount_out_min' : IDL.Nat64,
    'token_in' : TokenType,
    'amount_in' : IDL.Nat64,
    'token_out' : TokenType,
  });
  const SwapResult = IDL.Record({
    'txids' : SubmittedTxidType,
    'amount_out' : IDL.Nat64,
    'amount_in' : IDL.Nat64,
  });
  const WithdrawalType = IDL.Variant({
    'Rune' : IDL.Record({
      'to' : IDL.Text,
      'fee_per_vbytes' : IDL.Opt(IDL.Nat64),
      'runeid' : RuneId,
      'amount' : IDL.Nat,
    }),
    'Bitcoin' : IDL.Record({
      'to' : IDL.Text,
      'fee_per_vbytes' : IDL.Opt(IDL.Nat64),
      'amount' : IDL.Nat64,
    }),
  });
  return IDL.Service({
    'add_liquidity' : IDL.Func([AddLiquidityArgs], [AddLiquidityResult], []),
    'create_pair' : IDL.Func([CreatePairArgs], [IDL.Nat], []),
    'get_amount_out' : IDL.Func(
        [IDL.Nat64, TokenType, TokenType],
        [IDL.Nat64],
        ['query'],
      ),
    'get_balances' : IDL.Func([], [IDL.Vec(IDL.Tuple(TokenType, IDL.Nat))], []),
    'get_bitcoin_address' : IDL.Func([], [IDL.Text], ['query']),
    'get_pairs' : IDL.Func([], [IDL.Vec(PairInfoQuery)], ['query']),
    'get_positions' : IDL.Func([], [IDL.Vec(PositionDetail)], ['query']),
    'get_user_info' : IDL.Func([], [User], ['query']),
    'remove_liquidity' : IDL.Func(
        [RemoveLiquidityArgs],
        [AddLiquidityResult],
        [],
      ),
    'swap_exact_tokens_for_tokens' : IDL.Func(
        [SwapExactTokensForTokensArgs],
        [SwapResult],
        [],
      ),
    'withdraw' : IDL.Func([WithdrawalType], [SubmittedTxidType], []),
  });
};
export const init = ({ IDL }) => {
  const BitcoinNetwork = IDL.Variant({
    'mainnet' : IDL.Null,
    'regtest' : IDL.Null,
    'testnet' : IDL.Null,
  });
  const InitArgs = IDL.Record({
    'commission_receiver' : IDL.Opt(IDL.Principal),
    'auth' : IDL.Opt(IDL.Principal),
    'bitcoin_network' : BitcoinNetwork,
  });
  return [InitArgs];
};
