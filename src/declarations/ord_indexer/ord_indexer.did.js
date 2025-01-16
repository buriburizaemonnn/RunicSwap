export const idlFactory = ({ IDL }) => {
  const Result = IDL.Variant({ 'Ok' : IDL.Null, 'Err' : IDL.Text });
  const CandidRuneId = IDL.Record({ 'tx' : IDL.Nat32, 'block' : IDL.Nat64 });
  const CandidRuneEntry = IDL.Record({
    'id' : IDL.Nat,
    'runeid' : CandidRuneId,
    'divisibility' : IDL.Nat8,
    'block' : IDL.Nat64,
    'runename' : IDL.Text,
    'symbol' : IDL.Opt(IDL.Nat32),
  });
  const RpcError = IDL.Variant({
    'Io' : IDL.Tuple(IDL.Text, IDL.Text, IDL.Text),
    'Endpoint' : IDL.Tuple(IDL.Text, IDL.Text, IDL.Text),
    'Decode' : IDL.Tuple(IDL.Text, IDL.Text, IDL.Text),
  });
  const MintError = IDL.Variant({
    'Cap' : IDL.Nat,
    'End' : IDL.Nat64,
    'Start' : IDL.Nat64,
    'Unmintable' : IDL.Null,
  });
  const OrdError = IDL.Variant({
    'Rpc' : RpcError,
    'Overflow' : IDL.Null,
    'Params' : IDL.Text,
    'NotEnoughConfirmations' : IDL.Null,
    'Index' : MintError,
    'Unrecoverable' : IDL.Null,
    'BlockVerification' : IDL.Nat32,
    'OutPointNotFound' : IDL.Null,
    'Recoverable' : IDL.Record({ 'height' : IDL.Nat32, 'depth' : IDL.Nat32 }),
  });
  const Result_1 = IDL.Variant({
    'Ok' : IDL.Tuple(IDL.Nat32, IDL.Text),
    'Err' : OrdError,
  });
  const RuneId = IDL.Record({ 'tx' : IDL.Nat32, 'block' : IDL.Nat64 });
  const RuneBalance = IDL.Record({ 'id' : RuneId, 'balance' : IDL.Nat });
  const Result_2 = IDL.Variant({
    'Ok' : IDL.Vec(RuneBalance),
    'Err' : OrdError,
  });
  return IDL.Service({
    'admin_set_url' : IDL.Func([IDL.Text], [Result], []),
    'get_50_rune_entries' : IDL.Func([], [IDL.Vec(CandidRuneEntry)], ['query']),
    'get_height' : IDL.Func([], [Result_1], ['query']),
    'get_rune_entry_by_runeid' : IDL.Func(
        [CandidRuneId],
        [IDL.Opt(CandidRuneEntry)],
        ['query'],
      ),
    'get_runes_by_utxo' : IDL.Func(
        [IDL.Text, IDL.Nat32],
        [Result_2],
        ['query'],
      ),
  });
};
export const init = ({ IDL }) => { return [IDL.Text, IDL.Text]; };
