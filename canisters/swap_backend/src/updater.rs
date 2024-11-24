use bitcoin::hashes::Hash;
use ic_cdk::api::management_canister::bitcoin::{bitcoin_get_utxos, GetUtxosRequest, UtxoFilter};

use crate::{
    ord_canister,
    state::{
        read_config, read_rune_entries, read_utxo_state, utxo_manager::RunicUtxo,
        write_rune_entries, write_utxo_state,
    },
    types::RuneId,
};

pub enum TargetType {
    Bitcoin { target: u64 },
    Runic { rune: RuneId, target: u128 },
}

fn txid_to_string(txid: &[u8]) -> String {
    bitcoin::Txid::from_raw_hash(Hash::from_slice(txid).unwrap()).to_string()
}

pub async fn fetch_and_record_utxos_for_user(addr: &str, target_type: TargetType) {
    let network = read_config(|config| config.bitcoin_network());
    let mut arg = GetUtxosRequest {
        address: addr.to_owned(),
        network,
        filter: None,
    };
    loop {
        let utxo_response = bitcoin_get_utxos(arg.clone()).await.unwrap().0;
        for utxo in utxo_response.utxos {
            if read_utxo_state(|state| state.contains_utxo(addr, &utxo)) {
                continue;
            }
            let txid = txid_to_string(&utxo.outpoint.txid);
            match ord_canister::get_runes_by_utxo(txid, utxo.outpoint.vout)
                .await
                .expect("failed to call ord canister")
                .0
            {
                Err(_) => write_utxo_state(|state| state.record_bitcoin_utxos(addr, vec![utxo])),
                Ok(runes) => write_utxo_state(|state| {
                    ic_cdk::println!("{:?}", runes);
                    if !runes.is_empty() {
                        for rune in runes {
                            // checking if rune entry exists
                            /*
                             * TODO: move this block out of the closure
                            if read_rune_entries(|entries| entries.get(&rune.id).is_none()) {
                                if let Some(_) = ord_canister::get_rune_entry(rune.id.clone()).await
                                {
                                    write_rune_entries(|entries| {
                                        entries
                                            .insert(rune.id.clone(), RuneEntry { divisibility: 0 })
                                    });
                                }
                            }
                            */
                            ic_cdk::println!("recording runic utxos");
                            state.record_runic_utxos(
                                addr,
                                rune.id,
                                vec![RunicUtxo {
                                    balance: rune.balance,
                                    utxo: utxo.clone(),
                                }],
                            )
                        }
                    } else {
                        ic_cdk::println!("recording btc utxos");
                        state.record_bitcoin_utxos(addr, vec![utxo])
                    }
                }),
            }
            match target_type {
                TargetType::Runic { ref rune, target } => {
                    let balance = read_utxo_state(|state| state.get_runestone_balance(addr, rune));
                    if balance < target {
                        if let Some(ref page) = utxo_response.next_page {
                            arg.filter = Some(UtxoFilter::Page(page.to_vec()));
                            continue;
                        } else {
                            ic_cdk::println!("target didn't reached, not enough utxos");
                            return;
                        }
                    } else {
                        ic_cdk::println!("target reached");
                        return;
                    }
                }
                TargetType::Bitcoin { target } => {
                    let balance = read_utxo_state(|state| state.get_bitcoin_balance(addr));
                    if balance < target {
                        if let Some(ref page) = utxo_response.next_page {
                            arg.filter = Some(UtxoFilter::Page(page.to_vec()));
                            continue;
                        } else {
                            ic_cdk::println!("target didn't reached, not enough utxos");
                            return;
                        }
                    } else {
                        ic_cdk::println!("target reached");
                        return;
                    }
                }
            }
        }
    }
}
