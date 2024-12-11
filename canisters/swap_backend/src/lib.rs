mod chains;
mod memory;
mod ord_canister;
mod state;
mod txn_handler;
mod types;
mod updater;

use std::{collections::HashMap, time::Duration};

use candid::{CandidType, Principal};
use chains::{
    btc::{
        runestone::transfer::RuneTransferArgs,
        transaction::{combined::CombinedTransactionArgs, BtcTransferArgs},
    },
    generate_subaccount_for_pool, principal_to_subaccount, Addresses,
};
use ic_cdk::{
    api::management_canister::{
        bitcoin::BitcoinNetwork,
        ecdsa::{
            ecdsa_public_key, EcdsaKeyId, EcdsaPublicKeyArgument,
            EcdsaPublicKeyResponse as EcdsaPublicKey,
        },
    },
    init, post_upgrade, pre_upgrade, query, update,
};
use ic_ledger_types::account_balance;
use serde::Deserialize;
use state::{
    pool_manager::PoolInfo, read_config, read_pool_manager, read_utxo_manager, write_config,
    write_pool_manager,
};
use types::{RuneId, SubmittedTxidType, TokenType};
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
        temp.ecdsa_public_key = Some(ecdsa_response);
        let _ = config.set(temp);
    });
}

#[init]
pub fn init(bitcoin_network: BitcoinNetwork) {
    let keyname = match bitcoin_network {
        BitcoinNetwork::Mainnet => "key_1".to_string(),
        BitcoinNetwork::Testnet => "test_key_1".to_string(),
        BitcoinNetwork::Regtest => "dfx_test_key".to_string(),
    };
    write_config(|config| {
        let mut temp = config.get().clone();
        temp.keyname.replace(keyname);
        temp.bitcoin_network.replace(bitcoin_network);
        let _ = config.set(temp);
    });
    ic_cdk_timers::set_timer(Duration::from_secs(0), || ic_cdk::spawn(lazy_ecdsa_setup()));
}

#[pre_upgrade]
pub fn pre_upgrade() {}

#[post_upgrade]
pub fn post_upgrade() {}

#[query]
pub fn get_deposit_addresses() -> Addresses {
    let caller = ic_cdk::caller();
    Addresses::from(&caller)
}

#[update]
pub async fn test_combined_withdrawal(
    runeid: RuneId,
    rune_amount: u128,
    btc_amount: u64,
    to: String,
) -> SubmittedTxidType {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);

    let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
    let receiver = chains::btc::address_validation(to.as_str()).unwrap();

    updater::fetch_utxos_and_update_balances(
        &caller_addresses.bitcoin,
        updater::TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

    chains::btc::transaction::combined::transfer(CombinedTransactionArgs {
        runeid,
        rune_amount,
        rune_sender: sender.clone(),
        rune_receiver: receiver.clone(),
        rune_sender_account: caller_addresses.icrc1,
        btc_amount,
        bitcoin_sender: sender.clone(),
        bitcoin_receiver: receiver.clone(),
        bitcoin_sender_account: caller_addresses.icrc1,
        fee_payer: sender.clone(),
        fee_payer_account: caller_addresses.icrc1,
        postage: None,
        fee_per_vbytes,
    })
    .unwrap()
    .build_and_submit()
    .await
}

#[update]
pub async fn get_combined_balance(addr: String, runeid: RuneId) -> HashMap<TokenType, u128> {
    updater::fetch_utxos_and_update_balances(
        &addr,
        updater::TargetType::Bitcoin { target: u64::MAX },
    )
    .await;
    let mut balances = HashMap::new();
    read_utxo_manager(|manager| {
        let bitcoin_balance = manager.get_bitcoin_balance(&addr);
        let rune_balance = manager.get_runestone_balance(&addr, &runeid);
        balances.insert(TokenType::Bitcoin, bitcoin_balance as u128);
        balances.insert(TokenType::Runestone(runeid), rune_balance);
        balances
    })
}

#[update]
pub async fn get_user_balance() -> HashMap<TokenType, u128> {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    updater::fetch_utxos_and_update_balances(
        &caller_addresses.bitcoin,
        updater::TargetType::Bitcoin { target: u64::MAX },
    )
    .await;
    read_utxo_manager(|manager| {
        let mut balances = HashMap::new();
        let bitcoin_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
        let runes = manager.all_rune_with_balances(&caller_addresses.bitcoin);
        for (rune, balance) in runes {
            balances.insert(TokenType::Runestone(rune), balance);
        }
        balances.insert(TokenType::Bitcoin, bitcoin_balance as u128);
        balances
    })
}

#[derive(CandidType, Deserialize)]
pub enum WithdrawalType {
    Bitcoin {
        to: String,
        amount: u64,
    },
    Rune {
        to: String,
        runeid: RuneId,
        amount: u128,
    },
    Icp {
        to: String,
        amount: u64,
    },
    Icrc1 {
        to: String,
        icrc1: Principal,
        amount: u128,
    },
}

pub async fn withdraw(withdrawal_type: WithdrawalType) -> SubmittedTxidType {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    todo!()
}

#[derive(CandidType, Deserialize)]
pub struct CreatePairArgs {
    pub token0: TokenType,
    pub token1: TokenType,
}

#[update]
pub fn create_pair(CreatePairArgs { token0, token1 }: CreatePairArgs) -> u128 {
    if token0 == token1 {
        ic_cdk::trap("CREATE_PAIR_ERROR: Same Token")
    }
    write_pool_manager(|pools| {
        if pools
            .get_pool_id_by_tokens(token0.clone(), token1.clone())
            .is_some()
        {
            ic_cdk::trap("CREATE_PAIR_ERROR: Pair exists");
        }
        let current_count = pools.pool_mapping.len() as u128;
        let current_time = ic_cdk::api::time();

        let subaccount = generate_subaccount_for_pool(current_count, current_time);

        let pool_info = PoolInfo {
            pool_id: current_count,
            created_at: current_time,
            allocated_raw_subaccount: subaccount,
            token0,
            token1,
            total_supply: 0,
            k_last: 0,
            reserve0: 0,
            reserve1: 0,
            last_updated: current_time,
            holders: HashMap::new(),
        };
        pools.create_pair(pool_info);
        current_count
    })
}

#[derive(CandidType)]
pub struct PoolInfoQuery {
    pub pool_id: u128,
    pub deposit_addresses: Addresses,
    pub token0: TokenType,
    pub token1: TokenType,
    pub reserve0: u64,
    pub reserve1: u64,
}

pub fn get_pool_info() {}

#[query]
pub fn pools() -> Vec<PoolInfoQuery> {
    read_pool_manager(|manager| {
        let mut pools = vec![];
        for (_, pool) in manager.pool_mapping.iter() {
            let query = pool.into_query();
            pools.push(query);
        }
        pools
    })
}

#[derive(CandidType, Deserialize)]
pub struct AddLiquidityArgs {
    pub token0: TokenType,
    pub token1: TokenType,
    pub amount0_min: u64,
    pub amount1_min: u64,
    pub amount0_desired: u64,
    pub amount1_desired: u64,
}

#[update]
pub async fn add_liquidity(
    AddLiquidityArgs {
        mut token0,
        mut token1,
        mut amount0_min,
        mut amount1_min,
        mut amount0_desired,
        mut amount1_desired,
    }: AddLiquidityArgs,
) -> (u64, Vec<SubmittedTxidType>) {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);

    if token0 == token1 {
        ic_cdk::trap("ADD_LIQUIDITY_ERROR: Same Token");
    }

    let (pool_id, pool_addresses, amount0, amount1) = write_pool_manager(|pools| {
        let pool_info = match pools.get_pool_id_by_tokens(token0.clone(), token1.clone()) {
            None => ic_cdk::trap("ADD_LIQUIDITY_ERROR: Non-existing Pair"),
            Some(id) => pools.pool_mapping.get(&id).unwrap(),
        };

        let pool_addresses = Addresses::from(pool_info.allocated_raw_subaccount);
        if token0 != pool_info.token0 {
            std::mem::swap(&mut token0, &mut token1);
            std::mem::swap(&mut amount0_desired, &mut amount1_desired);
            std::mem::swap(&mut amount0_min, &mut amount1_min);
        }

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
        };
        (pool_info.pool_id, pool_addresses, amount0, amount1)
    });

    updater::fetch_utxos_and_update_balances(
        &caller_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    if let (TokenType::Bitcoin, TokenType::Runestone(rune)) = (token0.clone(), token1.clone()) {
        read_utxo_manager(|manager| {
            let btc_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
            let rune_balance = manager.get_runestone_balance(&caller_addresses.bitcoin, &rune);
            if btc_balance < amount0 || rune_balance < amount1 as u128 {
                ic_cdk::trap("Insufficient balance")
            }
        });
        let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
        let receiver = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
        let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

        let txn = chains::btc::transaction::combined::transfer(CombinedTransactionArgs {
            runeid: rune,
            rune_amount: amount1 as u128,
            rune_sender: sender.clone(),
            rune_receiver: receiver.clone(),
            rune_sender_account: caller_addresses.icrc1,
            btc_amount: amount0,
            bitcoin_sender: sender.clone(),
            bitcoin_receiver: receiver,
            bitcoin_sender_account: caller_addresses.icrc1,
            fee_payer: sender,
            fee_payer_account: caller_addresses.icrc1,
            postage: None,
            fee_per_vbytes,
        })
        .unwrap();
        let txid = txn.build_and_submit().await;
        let liquidity = write_pool_manager(|pools| {
            let mut pool_info = pools.pool_mapping.get(&pool_id).unwrap();
            let liquidity = pool_info.post_mint(caller, amount0, amount1).unwrap();
            pools.pool_mapping.insert(pool_id, pool_info);
            liquidity
        });
        return (liquidity, vec![txid]);
    }

    if let (TokenType::Runestone(rune), TokenType::Bitcoin) = (token0.clone(), token1.clone()) {
        read_utxo_manager(|manager| {
            let btc_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
            let rune_balance = manager.get_runestone_balance(&caller_addresses.bitcoin, &rune);
            if btc_balance < amount1 || rune_balance < amount0 as u128 {
                ic_cdk::trap("Insufficient balance")
            }
        });
        let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
        let receiver = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
        let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

        let txn = chains::btc::transaction::combined::transfer(CombinedTransactionArgs {
            runeid: rune,
            rune_amount: amount0 as u128,
            rune_sender: sender.clone(),
            rune_receiver: receiver.clone(),
            rune_sender_account: caller_addresses.icrc1,
            btc_amount: amount1,
            bitcoin_sender: sender.clone(),
            bitcoin_receiver: receiver,
            bitcoin_sender_account: caller_addresses.icrc1,
            fee_payer: sender,
            fee_payer_account: caller_addresses.icrc1,
            postage: None,
            fee_per_vbytes,
        })
        .unwrap();
        let txid = txn.build_and_submit().await;
        let liquidity = write_pool_manager(|pools| {
            let mut pool_info = pools.pool_mapping.get(&pool_id).unwrap();
            let liquidity = pool_info.post_mint(caller, amount0, amount1).unwrap();
            pools.pool_mapping.insert(pool_id, pool_info);
            liquidity
        });
        return (liquidity, vec![txid]);
    }

    let mut txns = vec![];

    match token0 {
        TokenType::Bitcoin => {
            read_utxo_manager(|manager| {
                let btc_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
                if btc_balance < amount0 {
                    ic_cdk::trap("Insufficient balance")
                }
            });
            let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
            let receiver = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
            let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

            let txn = chains::btc::transaction::transfer(BtcTransferArgs {
                sender,
                receiver,
                amount: amount0,
                sender_account: caller_addresses.icrc1,
                paid_by_sender: true,
                fee_per_vbytes,
            })
            .unwrap();
            txns.push(txn);
        }
        TokenType::Runestone(rune) => {
            read_utxo_manager(|manager| {
                let balance = manager.get_runestone_balance(&caller_addresses.bitcoin, &rune);
                if balance < amount0 as u128 {
                    ic_cdk::trap("Insufficient balance")
                }
            });
            let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
            let receiver = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
            let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

            let txn = chains::btc::runestone::transfer(RuneTransferArgs {
                runeid: rune.clone(),
                amount: amount0 as u128,
                sender_account: caller_addresses.icrc1,
                sender: sender.clone(),
                receiver,
                fee_per_vbytes,
                fee_payer: sender,
                fee_payer_account: caller_addresses.icrc1,
                postage: None,
            })
            .unwrap();
            txns.push(txn);
        }
        TokenType::Icp => {
            let subaccount = principal_to_subaccount(&caller);
            let balance = account_balance(
                ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                ic_ledger_types::AccountBalanceArgs {
                    account: caller_addresses.account_identifier,
                },
            )
            .await
            .unwrap()
            .e8s();

            if balance < amount0 {
                ic_cdk::trap("Insufficient balance")
            }

            let txn = chains::ic::icp::transfer(
                subaccount,
                pool_addresses.account_identifier,
                amount0,
                true,
            );
            txns.push(txn);
        }
        _ => unimplemented!(),
    }

    match token1 {
        TokenType::Bitcoin => {
            read_utxo_manager(|manager| {
                let btc_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
                if btc_balance < amount1 {
                    ic_cdk::trap("Insufficient balance")
                }
            });
            let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
            let receiver = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
            let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

            let txn = chains::btc::transaction::transfer(BtcTransferArgs {
                sender,
                receiver,
                amount: amount1,
                sender_account: caller_addresses.icrc1,
                paid_by_sender: true,
                fee_per_vbytes,
            })
            .unwrap();
            txns.push(txn);
        }
        TokenType::Runestone(rune) => {
            read_utxo_manager(|manager| {
                let balance = manager.get_runestone_balance(&caller_addresses.bitcoin, &rune);
                if balance < amount1 as u128 {
                    ic_cdk::trap("Insufficient balance")
                }
            });
            let sender = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
            let receiver = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
            let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

            let txn = chains::btc::runestone::transfer(RuneTransferArgs {
                runeid: rune.clone(),
                amount: amount1 as u128,
                sender_account: caller_addresses.icrc1,
                sender: sender.clone(),
                receiver,
                fee_per_vbytes,
                fee_payer: sender,
                fee_payer_account: caller_addresses.icrc1,
                postage: None,
            })
            .unwrap();
            txns.push(txn);
        }
        TokenType::Icp => {
            let subaccount = principal_to_subaccount(&caller);
            let balance = account_balance(
                ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                ic_ledger_types::AccountBalanceArgs {
                    account: caller_addresses.account_identifier,
                },
            )
            .await
            .unwrap()
            .e8s();

            if balance < amount1 {
                ic_cdk::trap("Insufficient balance")
            }

            let txn = chains::ic::icp::transfer(
                subaccount,
                pool_addresses.account_identifier,
                amount1,
                true,
            );
            txns.push(txn);
        }
        _ => unimplemented!(),
    }
    let mut txids = vec![];
    for txn in txns {
        let txid = txn.build_and_submit().await;
        txids.push(txid);
    }

    let liquidity = write_pool_manager(|pools| {
        let mut pool_info = pools.pool_mapping.get(&pool_id).unwrap();
        let liquidity = pool_info.post_mint(caller, amount0, amount1).unwrap();
        pools.pool_mapping.insert(pool_id, pool_info);
        liquidity
    });
    (liquidity, txids)
}

#[derive(CandidType, Deserialize)]
pub struct RemoveLiquidityArgs {
    pub token0: TokenType,
    pub token1: TokenType,
    pub amount0_min: u64,
    pub amount1_min: u64,
    pub liquidity: u64,
}

pub fn remove_liquidity(
    RemoveLiquidityArgs {
        token0,
        token1,
        amount0_min,
        amount1_min,
        liquidity,
    }: RemoveLiquidityArgs,
) {
    if token0 == token1 {
        ic_cdk::trap("REMOVE_LIQUIDITY_ERROR: Same Token")
    }
    unimplemented!("UNIMPLEMENTED")
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
    pub amount_out: u64,
    pub txids: Vec<SubmittedTxidType>,
}

#[update]
pub async fn swap(
    SwapArgs {
        token_in,
        token_out,
        amount_in,
        amount_out_min,
    }: SwapArgs,
) -> SwapResult {
    let caller = ic_cdk::caller();
    let caller_addresses = Addresses::from(&caller);
    if token_in == token_out {
        ic_cdk::trap("SWAP_ERROR: Same Token")
    }
    let (pool_id, pool_addresses) = read_pool_manager(|pools| {
        let pool_info = match pools.get_pool_id_by_tokens(token_in.clone(), token_out.clone()) {
            None => ic_cdk::trap("ADD_LIQUIDITY_ERROR: Non-existing Pair"),
            Some(id) => pools.pool_mapping.get(&id).unwrap(),
        };
        (
            pool_info.pool_id,
            Addresses::from(pool_info.allocated_raw_subaccount),
        )
    });

    updater::fetch_utxos_and_update_balances(
        &caller_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;
    updater::fetch_utxos_and_update_balances(
        &pool_addresses.bitcoin,
        TargetType::Bitcoin { target: u64::MAX },
    )
    .await;

    // balance check
    match token_in.clone() {
        TokenType::Icp => {
            let balance = account_balance(
                ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                ic_ledger_types::AccountBalanceArgs {
                    account: caller_addresses.account_identifier,
                },
            )
            .await
            .unwrap()
            .e8s();
            if balance < amount_in {
                ic_cdk::trap("Insufficient Balance")
            }
        }
        TokenType::Bitcoin => {
            let balance =
                read_utxo_manager(|manager| manager.get_bitcoin_balance(&caller_addresses.bitcoin));
            if balance < amount_in {
                ic_cdk::trap("Insufficient Balance")
            }
        }
        TokenType::Runestone(rune) => {
            let balance = read_utxo_manager(|manager| {
                manager.get_runestone_balance(&caller_addresses.bitcoin, &rune)
            });
            if balance < amount_in as u128 {
                ic_cdk::trap("Insufficient Balance")
            }
        }
        _ => unimplemented!(),
    }

    let swap_result = write_pool_manager(|manager| {
        let mut pool = manager.pool_mapping.get(&pool_id).unwrap();

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
        manager.pool_mapping.insert(pool_id, pool);
        swap_result
    });

    if let (TokenType::Bitcoin, TokenType::Runestone(rune)) =
        (token_in.clone(), swap_result.token.clone())
    {
        read_utxo_manager(|manager| {
            let btc_balance = manager.get_bitcoin_balance(&caller_addresses.bitcoin);
            let rune_balance = manager.get_runestone_balance(&pool_addresses.bitcoin, &rune);
            if btc_balance < amount_in || rune_balance < swap_result.amount as u128 {
                ic_cdk::trap("Insufficient balance")
            }
        });
        let caller_address = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
        let pool_address = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
        let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

        let txn = chains::btc::transaction::combined::transfer(CombinedTransactionArgs {
            runeid: rune,
            rune_amount: swap_result.amount as u128,
            rune_sender: pool_address.clone(),
            rune_receiver: caller_address.clone(),
            rune_sender_account: pool_addresses.icrc1,
            btc_amount: amount_in,
            bitcoin_sender: caller_address.clone(),
            bitcoin_receiver: pool_address,
            bitcoin_sender_account: caller_addresses.icrc1,
            fee_payer: caller_address,
            fee_payer_account: caller_addresses.icrc1,
            postage: None,
            fee_per_vbytes,
        })
        .unwrap();
        let txid = txn.build_and_submit().await;
        return SwapResult {
            amount_out: swap_result.amount,
            txids: vec![txid],
        };
    }

    if let (TokenType::Runestone(rune), TokenType::Bitcoin) =
        (token_in.clone(), swap_result.token.clone())
    {
        read_utxo_manager(|manager| {
            let btc_balance = manager.get_bitcoin_balance(&pool_addresses.bitcoin);
            let rune_balance = manager.get_runestone_balance(&caller_addresses.bitcoin, &rune);
            if btc_balance < swap_result.amount || rune_balance < amount_in as u128 {
                ic_cdk::trap("Insufficient balance")
            }
        });
        let caller_address = chains::btc::address_validation(&caller_addresses.bitcoin).unwrap();
        let pool_address = chains::btc::address_validation(&pool_addresses.bitcoin).unwrap();
        let fee_per_vbytes = chains::btc::get_fee_per_vbyte().await;

        let txn = chains::btc::transaction::combined::transfer(CombinedTransactionArgs {
            runeid: rune,
            rune_amount: amount_in as u128,
            rune_sender: caller_address.clone(),
            rune_receiver: pool_address.clone(),
            rune_sender_account: caller_addresses.icrc1,
            btc_amount: swap_result.amount,
            bitcoin_sender: pool_address.clone(),
            bitcoin_receiver: caller_address.clone(),
            bitcoin_sender_account: pool_addresses.icrc1,
            fee_payer: caller_address,
            fee_payer_account: caller_addresses.icrc1,
            postage: None,
            fee_per_vbytes,
        })
        .unwrap();
        let txid = txn.build_and_submit().await;
        return SwapResult {
            amount_out: swap_result.amount,
            txids: vec![txid],
        };
    }

    unimplemented!("Unsupported Token Type: Supports swap between Runestones and Bitcoin only")
}

ic_cdk::export_candid!();
