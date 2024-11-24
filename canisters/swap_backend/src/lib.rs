#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

mod chains;
mod ord_canister;
mod state;
mod transaction_handler;
mod types;
mod updater;

use std::{collections::HashMap, time::Duration};

use candid::{CandidType, Principal};
use ic_cdk::{
    api::management_canister::{
        bitcoin::BitcoinNetwork,
        ecdsa::{
            ecdsa_public_key, EcdsaKeyId, EcdsaPublicKeyArgument,
            EcdsaPublicKeyResponse as EcdsaPublicKey,
        },
    },
    init, query, update,
};
use ic_ledger_types::{AccountBalanceArgs, AccountIdentifier};
use icrc_ledger_types::icrc1::account::Account;

use serde::Deserialize;
use state::{
    pools::PoolDetails, read_config, read_pool_state, read_user_mapping, read_utxo_state,
    write_config, write_pool_state, write_user_mapping,
};
use types::{RuneId, SubmittedTransactionIdType, TokenType};
use updater::TargetType;

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
        temp.ecdsa_key = Some(ecdsa_response);
        let _ = config.set(temp);
    });
}

#[derive(CandidType, Deserialize)]
pub struct InitArgs {
    pub auth: Option<Principal>,
    pub commission_receiver_icp: Option<AccountIdentifier>,
    pub commission_receiver_btc: String,
    pub commission_receiver_principal: Option<Principal>,
    pub bitcoin_network: BitcoinNetwork,
    // pub ckbtc: Principal,
    pub ord_canister: Principal,
}

#[init]
pub fn init(
    InitArgs {
        auth,
        commission_receiver_icp,
        commission_receiver_btc,
        commission_receiver_principal,
        bitcoin_network,
        // ckbtc,
        ord_canister,
    }: InitArgs,
) {
    let caller = ic_cdk::caller();
    let auth = auth.unwrap_or(caller);
    let keyname = match bitcoin_network {
        BitcoinNetwork::Mainnet => "key_1".to_string(),
        BitcoinNetwork::Testnet => "test_key_1".to_string(),
        BitcoinNetwork::Regtest => "dfx_test_key".to_string(),
    };
    let commission_receiver_icp = commission_receiver_icp.unwrap_or(AccountIdentifier::new(
        &caller,
        &ic_ledger_types::DEFAULT_SUBACCOUNT,
    ));
    let commission_receiver_principal = commission_receiver_principal.unwrap_or(caller);
    write_config(|config| {
        let mut temp = config.get().clone();
        temp.auth.replace(auth);
        temp.keyname.replace(keyname);
        temp.bitcoin_network.replace(bitcoin_network);
        temp.commission_receiver_account_identifier
            .replace(commission_receiver_icp);
        temp.commission_receiver_principal
            .replace(commission_receiver_principal);
        temp.commission_receiver_bitcoin
            .replace(commission_receiver_btc);
        temp.ord_canister.replace(ord_canister);
        // temp.ckbtc.replace(ckbtc);
        let _ = config.set(temp);
    });
    ic_cdk_timers::set_timer(Duration::from_secs(0), || ic_cdk::spawn(lazy_ecdsa_setup()));
}

#[derive(CandidType)]
pub struct DepositAddresses {
    pub account: Account,
    pub account_string: String,
    pub account_identifier: AccountIdentifier,
    pub account_identifier_string: String,
    pub bitcoin: String,
}

#[query]
pub fn get_deposit_addresses() -> DepositAddresses {
    let caller = ic_cdk::caller();
    chains::generate_deposit_addresses(&caller)
}

pub fn user_info() {}

#[derive(CandidType, Deserialize)]
pub enum WithdrawalType {
    Icp {
        to: String,
        amount: u64,
    },
    Bitcoin {
        to: String,
        amount: u64,
        fee_per_vbytes: Option<u64>,
    },
    Runestone {
        runeid: RuneId,
        to: String,
        amount: u128,
        fee_per_vbytes: Option<u64>,
    },
}

#[update]
pub async fn withdraw(withdrawal_type: WithdrawalType) -> SubmittedTransactionIdType {
    let caller = ic_cdk::caller();
    let deposit_addresses = chains::generate_deposit_addresses(&caller);
    match withdrawal_type {
        WithdrawalType::Icp { to, amount } => {
            let to = AccountIdentifier::from_hex(&to).expect("failed to validate adddress");
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < amount {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: deposit_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < amount {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        amount, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, amount, false);
            txn.build_and_submit().await.expect("Failed to submit txn")
        }
        WithdrawalType::Bitcoin {
            to,
            amount,
            fee_per_vbytes,
        } => {
            let to = chains::bitcoin::validate_address(&to);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&deposit_addresses.bitcoin));
            if current_balance < amount {
                updater::fetch_and_record_utxos_for_user(
                    &deposit_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&deposit_addresses.bitcoin));
                if current_balance < amount {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount, current_balance
                    );
                    ic_cdk::trap(&err_msg)
                }
            }
            let fee_per_vbytes = match fee_per_vbytes {
                Some(fee_per_vbytes) => fee_per_vbytes,
                None => chains::bitcoin::get_fee_per_vbyte().await,
            };
            let from = chains::bitcoin::validate_address(&deposit_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &deposit_addresses.bitcoin,
                &from,
                &deposit_addresses.account,
                &to,
                amount,
                fee_per_vbytes,
                false,
            );
            txn.build_and_submit().await.expect("Failed to submit txn")
        }
        WithdrawalType::Runestone {
            runeid,
            to,
            amount,
            fee_per_vbytes,
        } => {
            let receiver = chains::bitcoin::validate_address(&to);
            let from = chains::bitcoin::validate_address(&deposit_addresses.bitcoin);
            let (current_rune_balance, current_bitcoin_balance) = read_utxo_state(|state| {
                // runestone balance
                let r = state.get_runestone_balance(&deposit_addresses.bitcoin, &runeid);
                // bitcoin balance
                let b = state.get_bitcoin_balance(&deposit_addresses.bitcoin);
                (r, b)
            });
            if current_rune_balance < amount || current_bitcoin_balance < 20_000 {
                updater::fetch_and_record_utxos_for_user(
                    &deposit_addresses.bitcoin,
                    TargetType::Runic {
                        rune: runeid.clone(),
                        target: amount,
                    },
                )
                .await;
            }
            let (current_rune_balance, current_bitcoin_balance) = read_utxo_state(|state| {
                // runestone balance
                let r = state.get_runestone_balance(&deposit_addresses.bitcoin, &runeid);
                // bitcoin balance
                let b = state.get_bitcoin_balance(&deposit_addresses.bitcoin);
                (r, b)
            });
            if current_rune_balance < amount || current_bitcoin_balance < 20_000 {
                let err_msg = format!("Not Enough Balance\nCurrent rune balance: {}, Required: {}\nCurrent bitcoin balance: {}, Required: {}", current_rune_balance, amount, current_bitcoin_balance, 20_000);
                ic_cdk::trap(&err_msg)
            }
            let fee_per_vbytes = match fee_per_vbytes {
                Some(fee_per_vbytes) => fee_per_vbytes,
                None => chains::bitcoin::get_fee_per_vbyte().await,
            };
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                amount,
                &deposit_addresses.bitcoin,
                &to,
                &from,
                &receiver,
                &deposit_addresses.account,
                &deposit_addresses.account, // we know fee is paid by the caller
                true,
                fee_per_vbytes,
                None,
            );
            txn.build_and_submit().await.expect("failed to submit txn")
        }
    }
}

/*
 * NOTE: This was for testing

#[update]
pub async fn get_balance_of_rune(runeid: RuneId) -> u128 {
    let caller = ic_cdk::caller();
    let deposit_addresses = chains::generate_deposit_addresses(&caller);
    updater::fetch_and_record_utxos_for_user(
        &deposit_addresses.bitcoin,
        TargetType::Runic {
            rune: runeid.clone(),
            target: u128::MAX,
        },
    )
    .await;
    read_utxo_state(|state| state.get_runestone_balance(&deposit_addresses.bitcoin, &runeid))
}

#[update]
pub async fn test_rune_withdrawal_from_other_user(
    from: Principal,
    runeid: RuneId,
    amount: u128,
) -> SubmittedTransactionIdType {
    let sender_deposit_addresses = chains::generate_deposit_addresses(&from);
    let receiver_deposit_addresses = chains::generate_deposit_addresses(&ic_cdk::caller());
    let sender_address = chains::bitcoin::validate_address(&sender_deposit_addresses.bitcoin);
    let receiver_address = chains::bitcoin::validate_address(&receiver_deposit_addresses.bitcoin);
    let (current_rune_balance, current_bitcoin_balance) = read_utxo_state(|state| {
        // runestone balance
        let r = state.get_runestone_balance(&sender_deposit_addresses.bitcoin, &runeid);
        // bitcoin balance
        let b = state.get_bitcoin_balance(&receiver_deposit_addresses.bitcoin);
        (r, b)
    });
    if current_rune_balance < amount || current_bitcoin_balance < 20_000 {
        updater::fetch_and_record_utxos_for_user(
            &sender_deposit_addresses.bitcoin,
            TargetType::Runic {
                rune: runeid.clone(),
                target: amount,
            },
        )
        .await;
        updater::fetch_and_record_utxos_for_user(
            &receiver_deposit_addresses.bitcoin,
            TargetType::Bitcoin { target: 20_000 },
        )
        .await;
    }
    let (current_rune_balance, current_bitcoin_balance) = read_utxo_state(|state| {
        // runestone balance
        let r = state.get_runestone_balance(&sender_deposit_addresses.bitcoin, &runeid);
        // bitcoin balance
        let b = state.get_bitcoin_balance(&receiver_deposit_addresses.bitcoin);
        (r, b)
    });
    if current_rune_balance < amount || current_bitcoin_balance < 20_000 {
        let err_msg = format!("Not Enough Balance\nCurrent rune balance: {}, Required: {}\nCurrent bitcoin balance: {}, Required: {}", current_rune_balance, amount, current_bitcoin_balance, 20_000);
        ic_cdk::trap(&err_msg)
    }
    let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
    let txn = chains::bitcoin::runestone::transfer(
        &runeid,
        amount,
        &sender_deposit_addresses.bitcoin,
        &receiver_deposit_addresses.bitcoin,
        &sender_address,
        &receiver_address,
        &sender_deposit_addresses.account,
        &receiver_deposit_addresses.account,
        false,
        fee_per_vbytes,
        None,
    );
    txn.build_and_submit().await.expect("failed to submit txn")
}
*/

#[query]
pub fn get_pair_info(token0: TokenType, token1: TokenType) -> PoolDetailsQuery {
    read_pool_state(|state| {
        let id = state
            .get_pool_id(&token0, &token1)
            .expect("pair doesn't exists");
        let pool_info = state.pool_mapping.get(&id).unwrap();
        pool_info.into_query()
    })
}

#[query]
pub fn get_pairs(prev: Option<u128>) -> Vec<PoolDetailsQuery> {
    let mut prev = prev.unwrap_or(0);
    read_pool_state(|state| {
        let mut pools = vec![];
        while let Some(pool) = state.pool_mapping.get(&prev) {
            pools.push(pool.into_query());
            if pools.len() >= 50 {
                return pools;
            }
            prev += 1;
        }
        pools
    })
}

#[derive(CandidType, Deserialize)]
pub struct CreatePairArgs {
    pub token0: TokenType,
    pub token1: TokenType,
}

#[derive(CandidType)]
pub struct PoolDetailsQuery {
    pub pool_id: u128,
    pub created_at: u64,
    pub token0: TokenType,
    pub token1: TokenType,
    pub k_last: u64,
    pub reserve0: u64,
    pub reserve1: u64,
    pub last_updated: u64,
    pub total_supply: u64,
}

#[update]
pub async fn create_pair(CreatePairArgs { token0, token1 }: CreatePairArgs) -> PoolDetailsQuery {
    let caller = ic_cdk::caller();
    let caller_addresses = chains::generate_deposit_addresses(&caller);

    read_pool_state(|state| {
        if state.get_pool_id(&token0, &token1).is_some() {
            ic_cdk::trap("Pool already exists")
        }
    });

    let mut current_balance =
        read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
    if current_balance < 1_000_000 + ic_ledger_types::DEFAULT_FEE.e8s() {
        current_balance = ic_ledger_types::account_balance(
            ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
            AccountBalanceArgs {
                account: caller_addresses.account_identifier,
            },
        )
        .await
        .unwrap()
        .e8s();
        write_user_mapping(|mapping| {
            let mut details = mapping.get(&caller).unwrap_or_default();
            details.icp_balance = current_balance;
            mapping.insert(caller, details);
        });
        if current_balance < 1_000_000 + ic_ledger_types::DEFAULT_FEE.e8s() {
            ic_cdk::trap("Not Enough Balance")
        }
    }
    let from_subaccount = chains::generate_ic_subaccount(&caller);
    let fee_receiver = read_config(|config| config.commission_receiver_account_identifier());
    let txn = chains::ic::transfer(from_subaccount, fee_receiver, 1_000_000, true);
    txn.build_and_submit().await.expect("failed to submit txn");
    write_pool_state(|state| {
        let id = state.generate_pool_id();
        let created_at = ic_cdk::api::time();
        let allocated_raw_subaccount = chains::generate_subaccount_for_pool(id, created_at);
        let allocated_bitcoin_address =
            chains::bitcoin::address::account_to_p2pkh_address(&Account {
                owner: ic_cdk::id(),
                subaccount: Some(allocated_raw_subaccount),
            });
        let pool_info = PoolDetails {
            pool_id: id,
            created_at,
            allocated_raw_subaccount,
            allocated_bitcoin_address,
            token0,
            token1,
            k_last: 0,
            reserve0: 0,
            reserve1: 0,
            last_updated: created_at,
            total_supply: 0,
            holders: HashMap::new(),
        };
        let response = pool_info.into_query();
        state.create_pair(id, pool_info);
        response
    })
}

#[derive(CandidType, Deserialize)]
pub struct AddLiquidityArgs {
    pub token0: TokenType,
    pub token1: TokenType,
    pub amount0_desired: u64,
    pub amount1_desired: u64,
    pub amount0_min: u64,
    pub amount1_min: u64,
}

#[derive(CandidType)]
pub struct AddLiquidityResult {
    pub liquidity: u64,
    pub txids: Vec<SubmittedTransactionIdType>,
}

#[update]
pub async fn add_liquidity(
    AddLiquidityArgs {
        token0,
        token1,
        amount0_desired,
        amount1_desired,
        amount0_min,
        amount1_min,
    }: AddLiquidityArgs,
) -> AddLiquidityResult {
    let caller = ic_cdk::caller();
    let caller_addresses = chains::generate_deposit_addresses(&caller);
    let (pool_id, mut pool_info, amount0, amount1) = read_pool_state(|pools| {
        let pool_id = pools
            .get_pool_id(&token0, &token1)
            .expect("pool doesn't exist");
        let pool_info = pools
            .pool_mapping
            .get(&pool_id)
            .expect("pair doesn't exists");
        let amount0;
        let amount1;
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
        if let Err(err) = pool_info.pre_mint(amount0, amount1) {
            ic_cdk::trap(&err)
        }
        (pool_id, pool_info, amount0, amount1)
    });
    let pool_addresses = pool_info.deposit_addresses();
    let mut txns = vec![];
    match token0.clone() {
        TokenType::Icp => {
            let to = pool_addresses.account_identifier;
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < amount0 {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: pool_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < amount0 {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        amount0, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, amount0, true);
            txns.push(txn);
        }
        TokenType::Bitcoin => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < amount0 {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount0 },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < amount0 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount0, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &pool_addresses.bitcoin,
                &from,
                &pool_addresses.account,
                &to,
                amount0,
                fee_per_vbytes,
                true,
            );
            txns.push(txn);
        }
        TokenType::Runestone(ref runeid) => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < amount0 {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount0 },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < amount0 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount0, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                amount0 as u128,
                &pool_addresses.bitcoin,
                &caller_addresses.bitcoin,
                &from,
                &to,
                &pool_addresses.account,
                &caller_addresses.account,
                true,
                fee_per_vbytes,
                None,
            );
            txns.push(txn);
        }
        _ => unimplemented!(),
    }
    match token1.clone() {
        TokenType::Icp => {
            let to = pool_addresses.account_identifier;
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < amount1 {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: pool_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < amount1 {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        amount1, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, amount1, true);
            txns.push(txn);
        }
        TokenType::Bitcoin => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < amount1 {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount1 },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < amount1 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount1, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &pool_addresses.bitcoin,
                &from,
                &pool_addresses.account,
                &to,
                amount1,
                fee_per_vbytes,
                true,
            );
            txns.push(txn);
        }
        TokenType::Runestone(ref runeid) => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < amount1 {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount1 },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < amount1 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount1, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                amount1 as u128,
                &pool_addresses.bitcoin,
                &caller_addresses.bitcoin,
                &from,
                &to,
                &pool_addresses.account,
                &caller_addresses.account,
                true,
                fee_per_vbytes,
                None,
            );
            txns.push(txn);
        }
        _ => unimplemented!(),
    }
    let mut txids = vec![];
    for txn in txns {
        let txid = txn.build_and_submit().await.expect("failed to submit txn");
        txids.push(txid)
    }
    let liquidity = pool_info
        .post_mint(caller, amount0, amount1)
        .expect("should mint");
    write_pool_state(|pools| pools.pool_mapping.insert(pool_id, pool_info));
    AddLiquidityResult { liquidity, txids }
}

#[derive(CandidType, Deserialize)]
pub struct RemoveLiquidityArgs {
    pub token0: TokenType,
    pub token1: TokenType,
    pub liquidity: u64,
    pub amount0_min: u64,
    pub amount1_min: u64,
}

#[derive(CandidType)]
pub struct RemoveLiquidityResult {
    pub txids: Vec<SubmittedTransactionIdType>,
}

#[update]
pub async fn remove_liquidity(
    RemoveLiquidityArgs {
        token0,
        token1,
        liquidity,
        amount0_min,
        amount1_min,
    }: RemoveLiquidityArgs,
) -> RemoveLiquidityResult {
    let caller = ic_cdk::caller();
    let caller_addresses = chains::generate_deposit_addresses(&caller);
    let burn_result = write_pool_state(|state| {
        let pool_id = state
            .get_pool_id(&token0, &token1)
            .expect("Pool doesn't exists");
        let mut pool_info = state.pool_mapping.get(&pool_id).unwrap();
        let burn_result = pool_info
            .burn(&caller, liquidity, amount0_min, amount1_min)
            .expect("failed to burn");
        state.pool_mapping.insert(pool_id, pool_info);
        burn_result
    });
    let pool_addresses =
        chains::generate_deposit_addresses_with_subaccount(burn_result.raw_subaccount);
    let mut txns = vec![];
    let mut utxo_sync = false;
    match burn_result.token0 {
        TokenType::Icp => {
            let to = caller_addresses.account_identifier;
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < burn_result.amount0 {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: pool_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < burn_result.amount0 {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        burn_result.amount0, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, burn_result.amount0, true);
            txns.push(txn);
        }
        TokenType::Bitcoin => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < burn_result.amount0 {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin {
                        target: burn_result.amount0,
                    },
                )
                .await;
                utxo_sync = true;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < burn_result.amount0 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        burn_result.amount0, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &pool_addresses.bitcoin,
                &from,
                &pool_addresses.account,
                &to,
                burn_result.amount0,
                fee_per_vbytes,
                true,
            );
            txns.push(txn);
        }
        TokenType::Runestone(runeid) => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < burn_result.amount0 {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin {
                        target: burn_result.amount0,
                    },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < burn_result.amount0 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        burn_result.amount0, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                burn_result.amount0 as u128,
                &pool_addresses.bitcoin,
                &caller_addresses.bitcoin,
                &from,
                &to,
                &pool_addresses.account,
                &caller_addresses.account,
                false,
                fee_per_vbytes,
                None,
            );
            txns.push(txn);
        }
        _ => unimplemented!(),
    };
    match burn_result.token1 {
        TokenType::Icp => {
            let to = caller_addresses.account_identifier;
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < burn_result.amount1 {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: pool_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < burn_result.amount1 {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        burn_result.amount1, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, burn_result.amount1, true);
            txns.push(txn);
        }
        TokenType::Bitcoin => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < burn_result.amount1 {
                if !utxo_sync {
                    updater::fetch_and_record_utxos_for_user(
                        &pool_addresses.bitcoin,
                        TargetType::Bitcoin {
                            target: burn_result.amount1,
                        },
                    )
                    .await;
                }
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < burn_result.amount1 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        burn_result.amount1, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &pool_addresses.bitcoin,
                &from,
                &pool_addresses.account,
                &to,
                burn_result.amount1,
                fee_per_vbytes,
                true,
            );
            txns.push(txn);
        }
        TokenType::Runestone(runeid) => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < burn_result.amount1 {
                if !utxo_sync {
                    updater::fetch_and_record_utxos_for_user(
                        &pool_addresses.bitcoin,
                        TargetType::Bitcoin {
                            target: burn_result.amount1,
                        },
                    )
                    .await;
                }
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < burn_result.amount1 {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        burn_result.amount1, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                burn_result.amount1 as u128,
                &pool_addresses.bitcoin,
                &caller_addresses.bitcoin,
                &from,
                &to,
                &pool_addresses.account,
                &caller_addresses.account,
                false,
                fee_per_vbytes,
                None,
            );
            txns.push(txn);
        }
        _ => unimplemented!(),
    }
    let mut ids = vec![];
    for txn in txns {
        let id = txn.build_and_submit().await.expect("failed to submit txn");
        ids.push(id);
    }
    RemoveLiquidityResult { txids: ids }
}

#[derive(CandidType, Deserialize)]
pub struct SwapArgs {
    pub token_in: TokenType,
    pub token_out: TokenType,
    pub amount_in: u64,
    pub amount_out_min: u64,
}

#[derive(CandidType)]
pub struct SwapResult {
    pub amount_received: u64,
    pub txids: Vec<SubmittedTransactionIdType>,
}

#[update]
pub async fn swap_exact_tokens_for_tokens(
    SwapArgs {
        token_in,
        token_out,
        amount_in,
        amount_out_min,
    }: SwapArgs,
) -> SwapResult {
    let caller = ic_cdk::caller();
    let caller_addresses = chains::generate_deposit_addresses(&caller);
    let mut txids = vec![];

    let (pool_id, pool_addresses) = read_pool_state(|state| {
        let pool_id = state
            .get_pool_id(&token_in, &token_out)
            .expect("pool doesn't exists");
        let addresses = state
            .pool_mapping
            .get(&pool_id)
            .unwrap()
            .deposit_addresses();
        (pool_id, addresses)
    });
    match token_in {
        TokenType::Icp => {
            let to = pool_addresses.account_identifier;
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < amount_in {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: pool_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < amount_in {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        amount_in, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, amount_in, true);
            let txid = txn.build_and_submit().await.expect("failed to submit txn");
            txids.push(txid);
        }
        TokenType::Bitcoin => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < amount_in {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount_in },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < amount_in {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount_in, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &pool_addresses.bitcoin,
                &from,
                &pool_addresses.account,
                &to,
                amount_in,
                fee_per_vbytes,
                true,
            );
            let txid = txn.build_and_submit().await.expect("failed to submit txn");
            txids.push(txid);
        }
        TokenType::Runestone(ref runeid) => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < amount_in {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin { target: amount_in },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < amount_in {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        amount_in, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                amount_in as u128,
                &pool_addresses.bitcoin,
                &caller_addresses.bitcoin,
                &from,
                &to,
                &pool_addresses.account,
                &caller_addresses.account,
                true,
                fee_per_vbytes,
                None,
            );
            let txid = txn.build_and_submit().await.expect("failed to submit txn");
            txids.push(txid);
        }
        _ => unimplemented!(),
    }
    let swap_result = write_pool_state(|state| {
        let mut pool_info = state.pool_mapping.get(&pool_id).unwrap();
        let (amount0in, amount0out, amount1in, amount1out) =
            pool_info.sort_tokens(&token_in, amount_in, amount_out_min);
        let swap_result = pool_info
            .swap(amount0in, amount1in, amount0out, amount1out)
            .expect("failed to swap");
        state.pool_mapping.insert(pool_id, pool_info);
        swap_result
    });
    match swap_result.token {
        TokenType::Icp => {
            let to = caller_addresses.account_identifier;
            let mut current_balance =
                read_user_mapping(|mapping| mapping.get(&caller).unwrap_or_default().icp_balance);
            if current_balance < swap_result.amount {
                current_balance = ic_ledger_types::account_balance(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    AccountBalanceArgs {
                        account: pool_addresses.account_identifier,
                    },
                )
                .await
                .expect("failed to call canister")
                .e8s();
                write_user_mapping(|mapping| {
                    let mut details = mapping.get(&caller).unwrap_or_default();
                    details.icp_balance = current_balance;
                    mapping.insert(caller, details);
                });
                if current_balance < swap_result.amount {
                    let err_msg = format!(
                        "Not enough balance!\nRequired: {}, Current Balance: {}",
                        swap_result.amount, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let ic_subaccount = chains::generate_ic_subaccount(&caller);
            let txn = chains::ic::transfer(ic_subaccount, to, swap_result.amount, false);
            let txid = txn.build_and_submit().await.expect("failed to submit txn");
            txids.push(txid);
        }
        TokenType::Bitcoin => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < swap_result.amount {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin {
                        target: swap_result.amount,
                    },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < swap_result.amount {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        swap_result.amount, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::transaction::transfer(
                &pool_addresses.bitcoin,
                &from,
                &pool_addresses.account,
                &to,
                swap_result.amount,
                fee_per_vbytes,
                false,
            );
            let txid = txn.build_and_submit().await.expect("failed to submit txn");
            txids.push(txid);
        }
        TokenType::Runestone(runeid) => {
            let to = chains::bitcoin::validate_address(&caller_addresses.bitcoin);
            let mut current_balance =
                read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
            if current_balance < swap_result.amount {
                updater::fetch_and_record_utxos_for_user(
                    &pool_addresses.bitcoin,
                    TargetType::Bitcoin {
                        target: swap_result.amount,
                    },
                )
                .await;
                current_balance =
                    read_utxo_state(|state| state.get_bitcoin_balance(&pool_addresses.bitcoin));
                if current_balance < swap_result.amount {
                    let err_msg = format!(
                        "Not enough balance\nRequired: {}, Current Balance: {}",
                        swap_result.amount, current_balance
                    );
                    ic_cdk::trap(&err_msg);
                }
            }
            let fee_per_vbytes = chains::bitcoin::get_fee_per_vbyte().await;
            let from = chains::bitcoin::validate_address(&pool_addresses.bitcoin);
            let txn = chains::bitcoin::runestone::transfer(
                &runeid,
                swap_result.amount as u128,
                &pool_addresses.bitcoin,
                &caller_addresses.bitcoin,
                &from,
                &to,
                &pool_addresses.account,
                &caller_addresses.account,
                false,
                fee_per_vbytes,
                None,
            );
            let txid = txn.build_and_submit().await.expect("failed to submit txn");
            txids.push(txid);
        }
        _ => unimplemented!(),
    }
    SwapResult {
        amount_received: swap_result.amount,
        txids,
    }
}

ic_cdk::export_candid!();
