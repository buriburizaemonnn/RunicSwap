export const idlFactory = ({ IDL }) => {
  const BitcoinNetwork = IDL.Variant({
    'mainnet' : IDL.Null,
    'regtest' : IDL.Null,
    'testnet' : IDL.Null,
  });
  const RuneId = IDL.Record({ 'tx' : IDL.Nat32, 'block' : IDL.Nat64 });
  const TokenType = IDL.Variant({
    'Icp' : IDL.Null,
    'Runestone' : RuneId,
    'Bitcoin' : IDL.Null,
    'CkBTC' : IDL.Null,
  });
  const AddLiquidityArgs = IDL.Record({
    'amount1_min' : IDL.Nat64,
    'amount0_desired' : IDL.Nat64,
    'amount0_min' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
    'amount1_desired' : IDL.Nat64,
  });
  const SubmittedTxidType = IDL.Variant({
    'Ic' : IDL.Record({ 'txid' : IDL.Nat64 }),
    'Icrc1' : IDL.Record({ 'txid' : IDL.Nat }),
    'Bitcoin' : IDL.Record({ 'txid' : IDL.Text }),
  });
  const CreatePairArgs = IDL.Record({
    'token0' : TokenType,
    'token1' : TokenType,
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
  const PoolInfoQuery = IDL.Record({
    'reserve0' : IDL.Nat64,
    'reserve1' : IDL.Nat64,
    'token0' : TokenType,
    'token1' : TokenType,
    'pool_id' : IDL.Nat,
    'deposit_addresses' : Addresses,
  });
  const SwapArgs = IDL.Record({
    'amount_out_min' : IDL.Nat64,
    'token_in' : TokenType,
    'amount_in' : IDL.Nat64,
    'token_out' : TokenType,
  });
  const SwapResult = IDL.Record({
    'txids' : IDL.Vec(SubmittedTxidType),
    'amount_out' : IDL.Nat64,
  });
  return IDL.Service({
    'add_liquidity' : IDL.Func(
        [AddLiquidityArgs],
        [IDL.Nat64, IDL.Vec(SubmittedTxidType)],
        [],
      ),
    'create_pair' : IDL.Func([CreatePairArgs], [IDL.Nat], []),
    'get_combined_balance' : IDL.Func(
        [IDL.Text, RuneId],
        [IDL.Vec(IDL.Tuple(TokenType, IDL.Nat))],
        [],
      ),
    'get_deposit_addresses' : IDL.Func([], [Addresses], ['query']),
    'get_user_balance' : IDL.Func(
        [],
        [IDL.Vec(IDL.Tuple(TokenType, IDL.Nat))],
        [],
      ),
    'pools' : IDL.Func([], [IDL.Vec(PoolInfoQuery)], ['query']),
    'swap' : IDL.Func([SwapArgs], [SwapResult], []),
    'test_combined_withdrawal' : IDL.Func(
        [RuneId, IDL.Nat, IDL.Nat64, IDL.Text],
        [SubmittedTxidType],
        [],
      ),
  });
};
export const init = ({ IDL }) => {
  const BitcoinNetwork = IDL.Variant({
    'mainnet' : IDL.Null,
    'regtest' : IDL.Null,
    'testnet' : IDL.Null,
  });
  return [BitcoinNetwork];
};
