use bitcoin::hashes::Hash;
use ic_cdk::api::management_canister::bitcoin::{bitcoin_get_utxos, GetUtxosRequest, UtxoFilter};

use crate::{
    ord_canister,
    state::{read_config, read_utxo_manager, write_utxo_manager},
    types::{RuneId, RunicUtxo},
};

fn txid_to_string(txid: &[u8]) -> String {
    bitcoin::Txid::from_raw_hash(Hash::from_slice(txid).unwrap()).to_string()
}

pub enum TargetType {
    Bitcoin { target: u64 },
    Runic { runeid: RuneId, target: u128 },
}

pub async fn fetch_utxos_and_update_balances(addr: &str, target: TargetType) {
    let network = read_config(|config| config.bitcoin_network());
    let mut arg = GetUtxosRequest {
        address: addr.to_string(),
        network,
        filter: None,
    };
    loop {
        let utxo_response = bitcoin_get_utxos(arg.clone())
            .await
            .expect("failed getting the utxo response")
            .0;
        let mut btc_utxos = vec![];
        for utxo in utxo_response.utxos {
            if read_utxo_manager(|manager| manager.is_recorded_as_runic(addr, &utxo)) {
                continue;
            }
            let txid = txid_to_string(&utxo.outpoint.txid);
            match ord_canister::get_runes_by_utxo(txid, utxo.outpoint.vout)
                .await
                .unwrap()
                .0
            {
                Err(_) => {
                    ic_cdk::println!("err while checking for runes, recording as non runic utxo");
                    btc_utxos.push(utxo);
                    continue;
                }
                Ok(runes) => {
                    if runes.is_empty() {
                        btc_utxos.push(utxo);
                        continue;
                    }
                    for rune in runes {
                        write_utxo_manager(|manager| {
                            manager.remove_btc_utxo(addr, &utxo);
                            manager.record_runic_utxos(
                                addr,
                                rune.id,
                                vec![RunicUtxo {
                                    utxo: utxo.clone(),
                                    balance: rune.balance,
                                }],
                            )
                        });
                    }
                }
            }
        }
        // recording of bitcoin utxo
        write_utxo_manager(|manager| manager.record_btc_utxos(addr, btc_utxos));

        match target {
            TargetType::Runic { ref runeid, target } => {
                let balance =
                    read_utxo_manager(|manager| manager.get_runestone_balance(addr, runeid));
                if balance < target && utxo_response.next_page.is_some() {
                    arg.filter = Some(UtxoFilter::Page(utxo_response.next_page.unwrap()));
                    continue;
                } else {
                    break;
                }
            }
            TargetType::Bitcoin { target } => {
                let balance = read_utxo_manager(|manager| manager.get_bitcoin_balance(addr));
                if balance < target && utxo_response.next_page.is_some() {
                    arg.filter = Some(UtxoFilter::Page(utxo_response.next_page.unwrap()));
                    continue;
                } else {
                    break;
                }
            }
        }
    }
}
