export const idlFactory = ({ IDL }) => {
  const BitcoinNetwork = IDL.Variant({
    'mainnet' : IDL.Null,
    'regtest' : IDL.Null,
    'testnet' : IDL.Null,
  });
  const InitArgs = IDL.Record({
    'auth' : IDL.Opt(IDL.Principal),
    'commission_receiver_btc' : IDL.Text,
    'commission_receiver_icp' : IDL.Opt(IDL.Vec(IDL.Nat8)),
    'commission_receiver_principal' : IDL.Opt(IDL.Principal),
    'bitcoin_network' : BitcoinNetwork,
    'ord_canister' : IDL.Principal,
  });
  const RuneId = IDL.Record({ 'tx' : IDL.Nat32, 'block' : IDL.Nat64 });
  const TokenType = IDL.Variant({
    'Icp' : IDL.Null,
    'Runestone' : RuneId,
    'Bitcoin' : IDL.Null,
  });
  const AddLiquidityArgs = IDL.Record({
    'amount1_min' : IDL.Nat64,
    'fee_per_vbytes' : IDL.Opt(IDL.Nat64),
    'amount0_desired' : IDL.Nat64,
    'amount0_min' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
    'amount1_desired' : IDL.Nat64,
  });
  const SubmittedTransactionIdType = IDL.Variant({
    'Icp' : IDL.Record({ 'txid' : IDL.Nat64 }),
    'Bitcoin' : IDL.Record({ 'txid' : IDL.Text }),
  });
  const AddLiquidityResult = IDL.Record({
    'txids' : IDL.Vec(SubmittedTransactionIdType),
    'liquidity' : IDL.Nat64,
  });
  const Account = IDL.Record({
    'owner' : IDL.Principal,
    'subaccount' : IDL.Opt(IDL.Vec(IDL.Nat8)),
  });
  const DepositAddresses = IDL.Record({
    'account_identifier' : IDL.Vec(IDL.Nat8),
    'account' : Account,
    'bitcoin' : IDL.Text,
    'account_identifier_string' : IDL.Text,
    'account_string' : IDL.Text,
  });
  const RemoveLiquidityArgs = IDL.Record({
    'amount1_min' : IDL.Nat64,
    'amount0_min' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
  });
  const RemoveLiquidityResult = IDL.Record({
    'liquidity_burned' : IDL.Nat64,
    'txids' : IDL.Vec(SubmittedTransactionIdType),
  });
  const SwapArgs = IDL.Record({
    'to' : IDL.Opt(IDL.Principal),
    'amount_out_min' : IDL.Nat64,
    'token_in' : TokenType,
    'amount_in' : IDL.Nat64,
    'token_out' : TokenType,
  });
  const SwapResult = IDL.Record({
    'txids' : IDL.Vec(SubmittedTransactionIdType),
    'amount_received' : IDL.Nat64,
  });
  const WithdrawalType = IDL.Variant({
    'Icp' : IDL.Record({ 'to' : IDL.Text, 'amount' : IDL.Nat64 }),
    'Runestone' : IDL.Record({
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
    'get_balance_of_rune' : IDL.Func([RuneId], [IDL.Nat], []),
    'get_deposit_addresses' : IDL.Func([], [DepositAddresses], ['query']),
    'remove_liquidity' : IDL.Func(
        [RemoveLiquidityArgs],
        [RemoveLiquidityResult],
        [],
      ),
    'swap_exact_tokens_for_tokens' : IDL.Func([SwapArgs], [SwapResult], []),
    'test_rune_withdrawal_from_other_user' : IDL.Func(
        [IDL.Principal, RuneId, IDL.Nat],
        [SubmittedTransactionIdType],
        [],
      ),
    'withdraw' : IDL.Func([WithdrawalType], [SubmittedTransactionIdType], []),
  });
};
export const init = ({ IDL }) => {
  const BitcoinNetwork = IDL.Variant({
    'mainnet' : IDL.Null,
    'regtest' : IDL.Null,
    'testnet' : IDL.Null,
  });
  const InitArgs = IDL.Record({
    'auth' : IDL.Opt(IDL.Principal),
    'commission_receiver_btc' : IDL.Text,
    'commission_receiver_icp' : IDL.Opt(IDL.Vec(IDL.Nat8)),
    'commission_receiver_principal' : IDL.Opt(IDL.Principal),
    'bitcoin_network' : BitcoinNetwork,
    'ord_canister' : IDL.Principal,
  });
  return [InitArgs];
};
