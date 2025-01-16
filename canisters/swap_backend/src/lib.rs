mod addresses;
mod chain;
mod memory;
mod state;
mod token_type;
mod txn_handler;
mod updater;

use std::collections::HashMap;
use std::time::Duration;

use addresses::{gen_subaccount_for_pool, Addresses};
use candid::{CandidType, Principal};
use chain::btc::address_validation;
use chain::btc::transaction::combined::CombinedTransferArgs;
use chain::btc::transaction::runestone::RuneTransferArgs;
use chain::btc::transaction::BtcTransferArgs;
use ic_cdk::api::management_canister::ecdsa::{
    ecdsa_public_key, EcdsaKeyId, EcdsaPublicKeyArgument, EcdsaPublicKeyResponse as EcdsaPublicKey,
};
use ic_cdk::{
    api::management_canister::bitcoin::BitcoinNetwork, init, post_upgrade, pre_upgrade, query,
    update,
};
use serde::Deserialize;
use state::pool_manager::PairInfo;
use state::{
    read_config, read_pool_manager, read_user_manager, read_utxo_manager, write_config,
    write_pool_manager, write_user_manager,
};
use token_type::TokenType;
use txn_handler::SubmittedTxidType;
use updater::{RuneId, TargetType};

const DEFAULT_SLIPPAGE: u8 = 5;

async fn lazy_ecdsa_setup() {
    let ecdsa_keyid: EcdsaKeyId = read_config(|config| config.ecdsakeyid());
    let ecdsa_response = ecdsa_public_key(EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![],
        key_id: ecdsa_keyid,
    })
    .await
    .expect("Failed to get ecdsa key")
    .0;

    write_config(|config| {
        let mut temp = config.get().clone();
        temp.ecdsa_public_key = Some(ecdsa_response);
        let _ = config.set(temp);
    });
}

#[derive(CandidType, Deserialize)]
pub struct InitArgs {
    pub bitcoin_network: BitcoinNetwork,
    pub auth: Option<Principal>,
    pub commission_receiver: Option<Principal>,
}

#[init]
pub fn init(
    InitArgs {
        bitcoin_network,
        auth,
        commission_receiver,
    }: InitArgs,
) {
    let caller = ic_cdk::caller();
    let auth = auth.unwrap_or(caller);
    let commission_receiver = commission_receiver.unwrap_or(caller);
    let keyname = match bitcoin_network {
        BitcoinNetwork::Mainnet => "key_1".to_string(),
        BitcoinNetwork::Testnet => "test_key_1".to_string(),
        BitcoinNetwork::Regtest => "dfx_test_key".to_string(),
    };
    write_config(|config| {
        let mut temp = config.get().clone();
        temp.keyname = Some(keyname);
        temp.bitcoin_network = Some(bitcoin_network);
        temp.commission_receiver_principal = Some(commission_receiver);
        temp.auth = Some(auth);
        let _ = config.set(temp);
    });
    ic_cdk_timers::set_timer(Duration::from_secs(0), || ic_cdk::spawn(lazy_ecdsa_setup()));
}

#[pre_upgrade]
pub fn pre_upgrade() {}

#[post_upgrade]
pub fn post_upgrade() {}

#[derive(CandidType)]
pub struct User {
    pub slippage: u8,
    pub deposit_addresses: Addresses,
}

#[query]
pub fn get_user_info() -> User {
    let caller = ic_cdk::caller();
    read_user_manager(|manager| {
        let slippage = match manager.get(&caller) {
            Some(user) => user.slippage,
            None => DEFAULT_SLIPPAGE,
        };
        User {
            slippage,
            deposit_addresses: Addresses::from(&caller),
        }
    })
}

#[query]
pub fn get_bitcoin_address() -> String {
    let caller = ic_cdk::caller();
    Addresses::from(&caller).bitcoin
}

#[derive(CandidType)]
pub struct PositionDetail {
    pub pool_id: u128,
    pub token0: TokenType,
    pub token1: TokenType,
    pub total_liquidity: u64,
    pub liquidity_owned: u64,
    pub amount0_owned: u64,
    pub amount1_owned: u64,
}

#[query]
pub fn get_positions() -> Vec<PositionDetail> {
    let caller = ic_cdk::caller();
    let holdings = read_user_manager(|manager| manager.get(&caller).unwrap_or_default().holdings);
    read_pool_manager(|manager| {
        let mut positions = vec![];
        for id in holdings {
            let pool = manager.pool_map.get(&id).unwrap();
            let current_liquidity = pool.holders.get(&caller).copied().unwrap_or_default();
            let amount0_owned = current_liquidity * pool.reserve0 / pool.total_supply;
            let amount1_owned = current_liquidity * pool.reserve1 / pool.total_supply;
            positions.push(PositionDetail {
                pool_id: id,
                token0: pool.token0.clone(),
                token1: pool.token1.clone(),
                total_liquidity: pool.total_supply,
                liquidity_owned: current_liquidity,
                amount0_owned,
                amount1_owned,
            })
        }
        positions
    })
}

#[update]
pub async fn get_balances() -> HashMap<TokenType, u128> {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);

    updater::fetch_utxos_and_update(
        &caller_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    read_utxo_manager(|manager| {
        let mut balances = HashMap::new();
        let bitcoin_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
        let runes = manager.all_rune_with_balances(&caller_addresses.bitcoin);
        balances.insert(TokenType::Bitcoin, bitcoin_balance as u128);
        for (id, balance) in runes {
            balances.insert(TokenType::Rune(id), balance);
        }
        balances
    })
}

#[derive(CandidType, Deserialize)]
pub enum WithdrawalType {
    Bitcoin {
        to: String,
        amount: u64,
        fee_per_vbytes: Option<u64>,
    },
    Rune {
        to: String,
        runeid: RuneId,
        amount: u128,
        fee_per_vbytes: Option<u64>,
    },
}

#[update]
pub async fn withdraw(withdrawal_type: WithdrawalType) -> SubmittedTxidType {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);

    updater::fetch_utxos_and_update(
        &caller_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    let txn = match withdrawal_type {
        WithdrawalType::Bitcoin {
            to,
            amount,
            fee_per_vbytes,
        } => {
            let receiver = chain::btc::address_validation(&to).expect("invalid address provided");
            let sender = chain::btc::address_validation(&caller_addresses.bitcoin).unwrap();
            let fee_per_vbytes = match fee_per_vbytes {
                None => chain::btc::get_fee_per_vbyte().await,
                Some(fee) => fee,
            };
            chain::btc::transfer(BtcTransferArgs {
                amount,
                sender,
                receiver,
                sender_account: caller_addresses.icrc1,
                paid_by_sender: true,
                fee_per_vbytes,
            })
            .unwrap()
        }
        WithdrawalType::Rune {
            to,
            runeid,
            amount,
            fee_per_vbytes,
        } => {
            let receiver = chain::btc::address_validation(&to).expect("invalid address provided");
            let sender = chain::btc::address_validation(&caller_addresses.bitcoin).unwrap();
            let fee_per_vbytes = match fee_per_vbytes {
                None => chain::btc::get_fee_per_vbyte().await,
                Some(fee) => fee,
            };
            chain::btc::transaction::runestone::transfer(RuneTransferArgs {
                runeid,
                rune_amount: amount,
                rune_sender: sender.clone(),
                rune_receiver: receiver,
                rune_sender_account: caller_addresses.icrc1,
                fee_per_vbytes,
                fee_payer: sender,
                fee_payer_account: caller_addresses.icrc1,
                postage: None,
            })
            .unwrap()
        }
    };
    txn.sign_and_submit().await.unwrap()
}

#[derive(CandidType)]
pub struct PairInfoQuery {
    pub token0: TokenType,
    pub token1: TokenType,
    pub reserve0: u64,
    pub reserve1: u64,
    pub no_of_holder: u32,
}

#[query]
pub fn get_pairs() -> Vec<PairInfoQuery> {
    read_pool_manager(|manager| {
        let mut pools = vec![];
        for (_id, pool) in manager.pool_map.iter() {
            let info = PairInfoQuery {
                token0: pool.token0.clone(),
                token1: pool.token1.clone(),
                reserve0: pool.reserve0,
                reserve1: pool.reserve1,
                no_of_holder: pool.holders.len() as u32,
            };
            pools.push(info);
            if pools.len() >= 50 {
                break;
            }
        }
        pools
    })
}

#[derive(CandidType, Deserialize)]
pub struct CreatePairArgs {
    pub token0: TokenType,
    pub token1: TokenType,
}

#[update]
pub fn create_pair(CreatePairArgs { token0, token1 }: CreatePairArgs) -> u128 {
    write_pool_manager(|manager| {
        if manager
            .get_pool_id(token0.clone(), token1.clone())
            .is_some()
        {
            ic_cdk::trap("CREATE_PAIR_ERROR: Pair Already Exists for the tokens");
        }

        let pool_id = manager.pool_map.len() as u128;
        let current_time = ic_cdk::api::time();

        let allocated_raw_subaccount = gen_subaccount_for_pool(pool_id, current_time);

        manager.record_pair(PairInfo {
            created_at: current_time,
            allocated_raw_subaccount,
            pool_id,
            token0,
            token1,
            reserve0: 0,
            reserve1: 0,
            total_supply: 0,
            k_last: 0,
            last_updated: current_time,
            holders: HashMap::new(),
        });
        pool_id
    })
}

#[derive(CandidType, Deserialize)]
pub struct AddLiquidityArgs {
    pub token0: TokenType,
    pub token1: TokenType,
    pub amount0_min: u64,
    pub amount0_desired: u64,
    pub amount1_min: u64,
    pub amount1_desired: u64,
    pub fee_per_vbytes: Option<u64>,
}

#[derive(CandidType)]
pub struct AddLiquidityResult {
    pub amount0: u64,
    pub amount1: u64,
    pub liquidity: u64,
    pub txids: SubmittedTxidType,
}

#[update]
pub async fn add_liquidity(
    AddLiquidityArgs {
        mut token0,
        mut token1,
        mut amount0_min,
        mut amount0_desired,
        mut amount1_min,
        mut amount1_desired,
        fee_per_vbytes,
    }: AddLiquidityArgs,
) -> AddLiquidityResult {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    let (mut pool_info, pool_addresses, amount0, amount1) = read_pool_manager(|manager| {
        let pool_info = match manager.get_pool_id(token0.clone(), token1.clone()) {
            None => ic_cdk::trap("ADD_LIQUIDITY_ERROR: Non-existing Pair"),
            Some(id) => manager.pool_map.get(&id).unwrap(),
        };
        let pool_addresses = pool_info.deposit_addresses();
        if token0 != pool_info.token0 {
            std::mem::swap(&mut token0, &mut token1);
            std::mem::swap(&mut amount0_desired, &mut amount1_desired);
            std::mem::swap(&mut amount0_min, &mut amount1_min);
        }
        let (amount0, amount1);
        if pool_info.is_reserve_empty() {
            amount0 = amount0_desired;
            amount1 = amount1_desired;
        } else {
            let amount1_optimal = pool_info.quote_a(amount0_desired).expect("failed to quote");
            if amount1_optimal <= amount1_desired {
                if amount1_optimal < amount1_min {
                    ic_cdk::trap("Insufficient B_AMOUNT")
                }
                amount0 = amount0_desired;
                amount1 = amount1_optimal;
            } else {
                let amount0_optimal = pool_info.quote_b(amount1_desired).expect("failed to quote");
                if amount0_optimal > amount0_desired {
                    ic_cdk::trap("failed")
                }
                if amount0_optimal < amount0_min {
                    ic_cdk::trap("Insufficient A_AMOUNT")
                }
                amount0 = amount0_optimal;
                amount1 = amount1_desired;
            }
        }
        if let Err(err) = pool_info.pre_mint_check(amount0, amount1) {
            ic_cdk::trap(&err);
        }
        (pool_info, pool_addresses, amount0, amount1)
    });

    let sender = address_validation(&caller_addresses.bitcoin).expect("should work");
    let receiver = address_validation(&pool_addresses.bitcoin).expect("should work");

    // NOTE: updates all utxos for now
    updater::fetch_utxos_and_update(
        &caller_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    let fee_per_vbytes = match fee_per_vbytes {
        None => chain::btc::get_fee_per_vbyte().await,
        Some(fee) => fee,
    };

    let txn = match (token0, token1) {
        (TokenType::Bitcoin, TokenType::Rune(runeid)) => {
            let (bitcoin_balance, rune_balance) = read_utxo_manager(|manager| {
                (
                    manager.get_bitcoin_balance(&caller_addresses.bitcoin),
                    (manager.get_runestone_balance(&caller_addresses.bitcoin, &runeid)),
                )
            });
            if amount0 > bitcoin_balance || amount1 as u128 > rune_balance {
                ic_cdk::trap("ADD_LIQUIDITY_ERROR: Not Enough Balance")
            }
            chain::btc::transaction::combined::transfer(CombinedTransferArgs {
                rune_sender_account: caller_addresses.icrc1,
                rune_sender: sender.clone(),
                rune_receiver: receiver.clone(),
                runeid,
                rune_amount: amount1 as u128,
                bitcoin_sender: sender.clone(),
                bitcoin_sender_account: caller_addresses.icrc1,
                bitcoin_receiver: receiver,
                bitcoin_amount: amount0,
                fee_per_vbytes,
                fee_payer_account: caller_addresses.icrc1,
                fee_payer: sender,
                postage: None,
            })
            .unwrap()
        }
        (TokenType::Rune(runeid), TokenType::Bitcoin) => {
            let (bitcoin_balance, rune_balance) = read_utxo_manager(|manager| {
                (
                    manager.get_bitcoin_balance(&caller_addresses.bitcoin),
                    (manager.get_runestone_balance(&caller_addresses.bitcoin, &runeid)),
                )
            });
            if amount1 > bitcoin_balance || amount0 as u128 > rune_balance {
                ic_cdk::trap("ADD_LIQUIDITY_ERROR: Not Enough Balance")
            }
            chain::btc::transaction::combined::transfer(CombinedTransferArgs {
                rune_amount: amount0 as u128,
                runeid,
                rune_receiver: receiver.clone(),
                rune_sender: sender.clone(),
                rune_sender_account: caller_addresses.icrc1,
                bitcoin_amount: amount1,
                bitcoin_receiver: receiver,
                bitcoin_sender_account: caller_addresses.icrc1,
                bitcoin_sender: sender.clone(),
                fee_per_vbytes,
                fee_payer: sender.clone(),
                fee_payer_account: caller_addresses.icrc1,
                postage: None,
            })
            .unwrap()
        }
        _ => unimplemented!("supports bitcoin and runestone pairs only"),
    };
    let txid = txn.sign_and_submit().await.unwrap();
    let liquidity = pool_info.mint(caller, amount0, amount1).unwrap();
    write_user_manager(|manager| {
        let mut user = manager.get(&caller).unwrap_or_default();
        if !user.holdings.contains(&pool_info.pool_id) {
            user.holdings.insert(pool_info.pool_id);
        }
        manager.insert(caller, user);
    });
    write_pool_manager(|manager| {
        manager.pool_map.insert(pool_info.pool_id, pool_info);
    });
    AddLiquidityResult {
        txids: txid,
        amount1,
        amount0,
        liquidity,
    }
}

#[derive(CandidType, Deserialize)]
pub struct RemoveLiquidityArgs {
    pub token0: TokenType,
    pub token1: TokenType,
    pub liquidity: u64,
    pub amount0_min: u64,
    pub amount1_min: u64,
    pub fee_per_vbytes: Option<u64>,
}

#[derive(CandidType)]
pub struct RemoveLiquidityResult {
    pub liquidity: u64,
    pub amount0: u64,
    pub amount1: u64,
    pub txids: SubmittedTxidType,
}

#[update]
pub async fn remove_liquidity(
    RemoveLiquidityArgs {
        token0,
        token1,
        liquidity,
        amount1_min,
        amount0_min,
        fee_per_vbytes,
    }: RemoveLiquidityArgs,
) -> RemoveLiquidityResult {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    let result = write_pool_manager(|manager| {
        let id = match manager.get_pool_id(token0.clone(), token1.clone()) {
            None => ic_cdk::trap("REMOVE_LIQUIDITY_ERROR: Pair doesn't exist"),
            Some(id) => id,
        };
        let mut pair_info = manager.pool_map.get(&id).unwrap();
        match pair_info.burn(&caller, liquidity, amount0_min, amount1_min) {
            Err(e) => ic_cdk::trap(&e),
            Ok(res) => {
                manager.pool_map.insert(id, pair_info);
                res
            }
        }
    });
    let pool_addresses = Addresses::from(result.raw_subaccount);
    updater::fetch_utxos_and_update(
        &pool_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    let fee_per_vbytes = match fee_per_vbytes {
        None => chain::btc::get_fee_per_vbyte().await,
        Some(fee) => fee,
    };

    let sender = address_validation(&pool_addresses.bitcoin).unwrap();
    let receiver = address_validation(&caller_addresses.bitcoin).unwrap();

    let txn = match (result.token0, result.token1) {
        (TokenType::Bitcoin, TokenType::Rune(runeid)) => {
            chain::btc::transaction::combined::transfer(CombinedTransferArgs {
                rune_sender_account: caller_addresses.icrc1,
                rune_sender: sender.clone(),
                rune_receiver: receiver.clone(),
                runeid,
                rune_amount: result.amount1 as u128,
                bitcoin_sender: sender.clone(),
                bitcoin_sender_account: caller_addresses.icrc1,
                bitcoin_receiver: receiver,
                bitcoin_amount: result.amount0,
                fee_per_vbytes,
                fee_payer_account: caller_addresses.icrc1,
                fee_payer: sender,
                postage: None,
            })
            .unwrap()
        }
        (TokenType::Rune(runeid), TokenType::Bitcoin) => {
            chain::btc::transaction::combined::transfer(CombinedTransferArgs {
                rune_amount: result.amount0 as u128,
                runeid,
                rune_receiver: receiver.clone(),
                rune_sender: sender.clone(),
                rune_sender_account: caller_addresses.icrc1,
                bitcoin_amount: result.amount1,
                bitcoin_receiver: receiver,
                bitcoin_sender_account: caller_addresses.icrc1,
                bitcoin_sender: sender.clone(),
                fee_per_vbytes,
                fee_payer: sender.clone(),
                fee_payer_account: caller_addresses.icrc1,
                postage: None,
            })
            .unwrap()
        }
        _ => unimplemented!("supports bitcoin and runestone pairs only"),
    };
    let txid = txn.sign_and_submit().await.unwrap();

    RemoveLiquidityResult {
        liquidity,
        amount0: result.amount0,
        amount1: result.amount1,
        txids: txid,
    }
}

#[derive(CandidType)]
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub txids: SubmittedTxidType,
}

#[derive(CandidType, Deserialize)]
pub struct SwapExactTokensForTokensArgs {
    pub token_in: TokenType,
    pub token_out: TokenType,
    pub amount_in: u64,
    pub amount_out_min: u64,
    pub fee_per_vbytes: Option<u64>,
}

#[query]
pub fn get_amount_out(amount_in: u64, token_in: TokenType, token_out: TokenType) -> u64 {
    read_pool_manager(|manager| {
        let pair_info = match manager.get_pool_id(token_in.clone(), token_out) {
            None => ic_cdk::trap("Non existing pair"),
            Some(id) => manager.pool_map.get(&id).unwrap(),
        };
        pair_info.get_amount_out(amount_in, &token_in)
    })
}

#[update]
pub async fn swap_exact_tokens_for_tokens(
    SwapExactTokensForTokensArgs {
        token_in,
        token_out,
        amount_in,
        amount_out_min,
        fee_per_vbytes,
    }: SwapExactTokensForTokensArgs,
) -> SwapResult {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    let (pool_id, pool_addresses) = read_pool_manager(|manager| {
        match manager.get_pool_id(token_in.clone(), token_out.clone()) {
            None => ic_cdk::trap("SWAP_ERROR: Pair doesn't exist"),
            Some(id) => (id, manager.pool_map.get(&id).unwrap().deposit_addresses()),
        }
    });

    updater::fetch_utxos_and_update(
        &caller_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;
    updater::fetch_utxos_and_update(
        &pool_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    let swap_result = write_pool_manager(|manager| {
        let mut pool = manager.pool_map.get(&pool_id).unwrap();

        let amount_out = pool.get_amount_out(amount_in, &token_in);
        ic_cdk::println!("amount out: {}", amount_out);
        ic_cdk::println!("amount_out as u128: {}", amount_out as u128);
        if amount_out < amount_out_min {
            ic_cdk::trap("exceeds amount_out_min")
        }

        let (amount0in, amount0out, amount1in, amount1out) =
            pool.sort_tokens(&token_in, amount_in, amount_out);
        let swap_result = pool
            .swap(amount0in, amount1in, amount0out, amount1out)
            .expect("failed to swap");
        manager.pool_map.insert(pool_id, pool);
        swap_result
    });

    let fee_per_vbytes = match fee_per_vbytes {
        None => chain::btc::get_fee_per_vbyte().await,
        Some(fee) => fee,
    };

    let pool_bitcoin_address = address_validation(&pool_addresses.bitcoin).unwrap();
    let caller_bitcoin_address = address_validation(&caller_addresses.bitcoin).unwrap();

    let txn = match (token_in, swap_result.token) {
        (TokenType::Bitcoin, TokenType::Rune(runeid)) => {
            chain::btc::transaction::combined::transfer(CombinedTransferArgs {
                rune_sender_account: pool_addresses.icrc1,
                rune_sender: pool_bitcoin_address.clone(),
                rune_receiver: caller_bitcoin_address.clone(),
                runeid,
                rune_amount: swap_result.amount as u128,
                bitcoin_sender: caller_bitcoin_address.clone(),
                bitcoin_sender_account: caller_addresses.icrc1,
                bitcoin_receiver: pool_bitcoin_address,
                bitcoin_amount: amount_in,
                fee_per_vbytes,
                fee_payer_account: caller_addresses.icrc1,
                fee_payer: caller_bitcoin_address,
                postage: None,
            })
            .unwrap()
        }
        (TokenType::Rune(runeid), TokenType::Bitcoin) => {
            chain::btc::transaction::combined::transfer(CombinedTransferArgs {
                rune_amount: amount_in as u128,
                runeid,
                rune_receiver: pool_bitcoin_address.clone(),
                rune_sender: caller_bitcoin_address.clone(),
                rune_sender_account: caller_addresses.icrc1,
                bitcoin_amount: swap_result.amount,
                bitcoin_receiver: caller_bitcoin_address.clone(),
                bitcoin_sender_account: pool_addresses.icrc1,
                bitcoin_sender: pool_bitcoin_address.clone(),
                fee_per_vbytes,
                fee_payer: caller_bitcoin_address.clone(),
                fee_payer_account: caller_addresses.icrc1,
                postage: None,
            })
            .unwrap()
        }
        _ => unimplemented!("supports bitcoin and runestone pairs only"),
    };

    let txid = txn.sign_and_submit().await.unwrap();

    SwapResult {
        txids: txid,
        amount_in,
        amount_out: swap_result.amount,
    }
}

/*
#[derive(CandidType, Deserialize)]
pub struct SwapTokensForExactTokensArgs {
    pub token_in: TokenType,
    pub token_out: TokenType,
    pub amount_in_max: u64,
    pub amount_out: u64,
    pub fee_per_vbytes: Option<u64>,
}

pub fn swap_tokens_for_exact_tokens(
    SwapTokensForExactTokensArgs {
        token_in,
        token_out,
        amount_in_max,
        amount_out,
        fee_per_vbytes,
    }: SwapTokensForExactTokensArgs,
) -> SwapResult {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    let mut pair_info = read_pool_manager(|manager| {
        let id = match manager.get_pool_id(token_in.clone(), token_out.clone()) {
            None => ic_cdk::trap("SWAP_ERROR: Pair doesn't exist"),
            Some(id) => id,
        };
        manager.pool_map.get(&id).unwrap()
    });
    todo!()
}
*/

ic_cdk::export_candid!();
