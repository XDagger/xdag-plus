// use time::{self, format_description::parse};
use tracing::{error, event, Level};
// use tracing_appender;
// use tracing_subscriber;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;

use qrcode::render::svg;
use qrcode::QrCode;

mod winit_helper;
use rpc::*;
use wallet::*;
use winit_helper::center_window;
use xerror::XwError;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

slint::include_modules!();

const TEST_EXPLORER: &str = "https://testexplorer.xdag.io/api/block";
const EXPLORER_URL: &str = "https://explorer.xdag.io/api/block";

const TEST_BLOCK_URL: &str = "https://testexplorer.xdag.io/block";
const BLOCK_URL: &str = "https://explorer.xdag.io/block";
pub enum NodeType {
    Mainnet,
    Testnet,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    //set slint backend to winit for enable window.set_rendering_notifier()
    if let Err(e) =
        slint::platform::set_platform(Box::new(i_slint_backend_winit::Backend::new().unwrap()))
    {
        event!(Level::ERROR, "set_platform error: {:?}", e);
    }

    let conf = config::get_config()?;
    let wallet_names = seek_wallet();

    let ui = App::new()?;

    let default_model: Vec<WalletItem> = ui.global::<UiWallets>().get_ui_wallets().iter().collect();

    let wallets_model = Rc::new(VecModel::from(default_model));
    ui.global::<UiWallets>()
        .set_ui_wallets(wallets_model.clone().into());

    // Window must be shown first so sizes get calculated properly
    // Don't know if there is a better way, slight redraw artifacting on move
    ui.show()?;
    center_window(ui.window());

    ui.global::<Language>()
        .set_name(conf.language.clone().into());
    ui.global::<WalletAccounts>().set_is_test(conf.istest);
    ui.global::<WalletAccounts>()
        .set_average_express_fee("0.00".into());
    let favorite_vec: Vec<_> = conf
        .favorite
        .iter()
        .map(|(address, name)| FavoriteAddr {
            name: name.into(),
            address: address.into(),
        })
        .collect();
    let favorite_model = Rc::new(VecModel::from(favorite_vec));
    ui.global::<WalletAccounts>()
        .set_favorite_model(favorite_model.clone().into());

    {
        // generate and show qr-code svg image from wallet address
        let ui_handle = ui.as_weak();
        if let Err(e) = ui
            .window()
            .set_rendering_notifier(move |state, _graphics_api| {
                if let slint::RenderingState::BeforeRendering = state {
                    let ui = ui_handle.upgrade().unwrap();
                    let current_address = ui.global::<WalletAccounts>().get_current_address();
                    let code = QrCode::new(current_address.as_bytes()).unwrap();
                    let svg = code
                        .render()
                        .min_dimensions(240, 240)
                        .dark_color(svg::Color("#63D0DF"))
                        .light_color(svg::Color("#08080A"))
                        .build();
                    let image = slint::Image::load_from_svg_data(svg.as_bytes()).unwrap();
                    ui.global::<WalletAccounts>().set_qrcode(image);
                }
            })
        {
            event!(Level::ERROR, "set_rendering_notifier error: {:?}", e);
        }
    }
    {
        let ui_handle = ui.as_weak();
        ui.global::<WalletAccounts>()
            .on_gen_mnemonic_array(move |mnemoic: SharedString| {
                let array: Vec<SharedString> = mnemoic.split(' ').map(|m| m.into()).collect();
                let mut result: Vec<Vec<SharedString>> =
                    array.chunks(4).map(|chunk| chunk.to_vec()).collect();
                if !result.is_empty() {
                    if let Some(last) = result.last_mut() {
                        while last.len() < 4 {
                            last.push("".into());
                        }
                    }
                }

                let mut mnem_vec: Vec<_> = vec![];
                for v in result {
                    mnem_vec.push(ModelRc::from(Rc::new(VecModel::from(v))));
                }

                let mnemonic_model = ModelRc::from(Rc::new(VecModel::from(mnem_vec)));
                let ui = ui_handle.upgrade().unwrap();
                ui.global::<WalletAccounts>()
                    .set_current_mnemonic(mnemonic_model);
            });
    }

    {
        let ui_handle = ui.as_weak();
        ui.global::<WalletAccounts>().on_set_config(move || {
            let ui = ui_handle.upgrade().unwrap();
            let lang = ui.global::<Language>().get_name();
            let istest = ui.global::<WalletAccounts>().get_is_test();
            let favorites = ui.global::<WalletAccounts>().get_favorite_model();
            let conf = config::Config {
                istest,
                language: lang.as_str().to_string(),
                favorite: favorites
                    .iter()
                    .map(|f| (f.address.as_str().to_string(), f.name.as_str().to_string()))
                    .collect(),
            };
            let _ = config::set_config(&conf);
        });
    }

    {
        let favorite_weak = Rc::downgrade(&favorite_model);
        ui.global::<WalletAccounts>()
            .on_add_contact(move |name, address| {
                if let Some(favorite_list) = favorite_weak.upgrade() {
                    favorite_list.push(FavoriteAddr { name, address });
                }
            });
    }

    ui.global::<WalletAccounts>().on_open_in_browser(
        move |is_test: bool, address: SharedString| {
            let url = if is_test {
                TEST_BLOCK_URL.to_owned() + "/" + address.as_str()
            } else {
                BLOCK_URL.to_owned() + "/" + address.as_str()
            };

            if let Err(e) = open::that(url) {
                event!(
                    Level::ERROR,
                    "view transaction address in browser error: {:?}",
                    e
                );
            }
        },
    );

    ui.global::<WalletAccounts>()
        .on_open_link(move |link: SharedString| {
            if let Err(e) = open::that(link) {
                event!(Level::ERROR, "open link in browser error: {:?}", e);
            }
        });

    {
        let favorite_weak = Rc::downgrade(&favorite_model);
        ui.global::<WalletAccounts>()
            .on_delete_contact(move |index| {
                if let Some(favorite_list) = favorite_weak.upgrade() {
                    if favorite_list.row_count() > index as usize {
                        favorite_list.remove(index as usize);
                    }
                }
            });
    }

    {
        let wallet_weak = Rc::downgrade(&wallets_model);
        ui.global::<WalletAccounts>()
            .on_delete_wallet(move |index| {
                if let Some(wallet_list) = wallet_weak.upgrade() {
                    if wallet_list.row_count() > index as usize {
                        let item = wallet_list.row_data(index as usize).unwrap();
                        event!(Level::INFO, "Deleting wallet: {}", &item.name); // Log the name of the wallet being deleted
                        let file_name = gen_file_path(&item.name); // Perform the deletion logic here, e.g., removing the wallet from the list
                        let path = std::path::Path::new(&file_name);
                        if path.exists() {
                            let prefix = path.parent().unwrap();
                            std::fs::remove_dir_all(prefix).unwrap();
                        }
                        wallet_list.remove(index as usize);
                        if wallet_list.row_count() == 0 {
                            std::process::exit(0)
                        }
                    }
                }
            });
    }

    {
        let ui_handle = ui.as_weak();
        ui.global::<WalletAccounts>()
            .on_change_password(move |old, new| {
                let handle = ui_handle.upgrade().unwrap();
                if let Err(e) = change_password(&old, &new) {
                    event!(Level::ERROR, "Failed to change password: {}", &e);

                    handle.global::<WalletAccounts>().set_err_visible(true);
                    handle
                        .global::<WalletAccounts>()
                        .set_err_message(e.root_cause().to_string().into());
                } else {
                    handle.global::<WalletAccounts>().set_current_password(new);
                    event!(Level::INFO, "Password changed successfully");
                }
            });
    }

    {
        ui.global::<WalletAccounts>()
            .on_rename_wallet(move |old, new| {
                event!(Level::INFO, "Renaming wallet: {} to {}", old, new);
                let file_old = gen_file_path(&old);
                let path_old = std::path::Path::new(&file_old);
                let file_new = gen_file_path(&new);
                let path_new = std::path::Path::new(&file_new);
                if path_old.exists() {
                    let prefix_old = path_old.parent().unwrap();
                    let prefix_new = path_new.parent().unwrap();
                    if std::fs::rename(prefix_old, prefix_new).is_err() {
                        error!("rename wallet failed");
                    }
                }
            });
    }

    // {
    //     let ui_handle = ui.as_weak();
    //     ui.global::<WalletAccounts>()
    //         .on_character_count(move |text| {
    //             let count = text.len();
    //             let ui = ui_handle.upgrade().unwrap();
    //             ui.global::<WalletAccounts>()
    //                 .set_remark_len_color(if count > 32 {
    //                     slint::Color::from_rgb_u8(255, 0, 0)
    //                 } else {
    //                     slint::Color::from_rgb_u8(255, 255, 255)
    //                 });
    //             count.to_string().into()
    //         });
    // }

    {
        let ui_handle = ui.as_weak();
        ui.global::<WalletAccounts>().on_send_xdag(
            move |is_test, mnemonic, from, to, amount, remark, extra_free| {
                let amount = amount.parse::<f64>().unwrap_or(0.0);
                let ui = ui_handle.upgrade().unwrap();
                let ui_weak = ui.as_weak();
                thread::spawn(move || {
                    let res = tokio::runtime::Runtime::new().unwrap().block_on(async {
                        transfer_xdag(
                            is_test,
                            &mnemonic,
                            &from,
                            &to,
                            amount,
                            &remark,
                            extra_free as f64,
                        )
                        .await
                    });

                    // let ui = ui_handle.upgrade().unwrap();
                    if let Err(e) = res {
                        ui_weak
                            .upgrade_in_event_loop(move |handle| {
                                event!(Level::ERROR, "send_xdag error: {:?}", e);

                                handle.global::<WalletAccounts>().set_sending(false);
                                handle.global::<WalletAccounts>().set_sent(false);
                                handle.global::<WalletAccounts>().set_err_visible(true);
                                handle
                                    .global::<WalletAccounts>()
                                    .set_err_message(e.root_cause().to_string().into());
                            })
                            .unwrap();
                    } else {
                        let block = res.unwrap();

                        ui_weak
                            .upgrade_in_event_loop(move |handle| {
                                handle.global::<WalletAccounts>().set_current_block_info(
                                    TransxBlockInfo {
                                        address: block.balance_address.into(),
                                        hash: block.hash.into(),
                                        fee: truncat_amount(&block.total_fee).into(),
                                        state: block.state.into(),
                                    },
                                );
                                handle.global::<WalletAccounts>().set_sending(false);
                                handle.global::<WalletAccounts>().set_sent(true);
                            })
                            .unwrap();
                    }
                });
            },
        );
    }

    {
        let ui_handle = ui.as_weak();
        ui.global::<WalletAccounts>()
            .on_fetch_average_express(move |is_test| {
                let ui = ui_handle.upgrade().unwrap();
                let ui_weak = ui.as_weak();
                thread::spawn(move || {
                    let res = tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(async { get_average_express_fee(is_test).await });
                    if let Err(e) = res {
                        event!(Level::ERROR, "get_average_express_fee error: {:?}", e);
                    } else {
                        let average = res.unwrap();
                        if let Err(_) = average.parse::<f64>() {
                            event!(Level::ERROR, "parse average express fee error");
                        } else {
                            ui_weak
                                .upgrade_in_event_loop(move |handle| {
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_average_express_fee(average.into());
                                })
                                .unwrap();
                        }
                    }
                });
            });
    }

    {
        let ui_handle = ui.as_weak();
        let wallets_weak = Rc::downgrade(&wallets_model);
        ui.global::<WalletAccounts>()
            .on_create_wallet(move |name, pswd, mnemonic| {
                let mut wallet: XWallet;
                if mnemonic.is_empty() {
                    let result = new_hd_wallet(&name, &pswd);

                    if let Err(e) = result {
                        event!(Level::ERROR, "create new wallet error: {:?}", e);
                        return;
                    }
                    wallet = result.unwrap();
                    if let Err(e) = wallet.flush() {
                        event!(Level::ERROR, "flush new wallet error: {:?}", e);
                        return;
                    }
                    event!(Level::INFO, "create new wallet success: {:?}", &name);
                } else {
                    let result = import_wallet(&name, &pswd, &mnemonic);
                    if let Err(e) = result {
                        let handle = ui_handle.upgrade().unwrap();
                        handle.global::<WalletAccounts>().set_err_visible(true);
                        handle
                            .global::<WalletAccounts>()
                            .set_err_message(e.root_cause().to_string().into());
                        event!(Level::ERROR, "import wallet error: {:?}", e);
                        return;
                    }
                    wallet = result.unwrap();
                    event!(Level::INFO, "import wallet success: {:?}", &name);
                }

                if let Some(wallet_list) = wallets_weak.upgrade() {
                    wallet_list.push(WalletItem {
                        name,
                        address: wallet.address.into(),
                        locked: false,
                        password: pswd,
                        mnemonic: wallet.mnemonic.into(),
                        privatekey: hex::encode(wallet.private_key.as_ref()).into(),
                        balance: "0.00".into(),
                    });
                }
            });
    }

    if let Some(wallet_folders) = wallet_names {
        // event!(
        //     Level::INFO,
        //     "found and opening wallet [{:?}] .",
        //     wallet_folders
        // );
        let wallets_weak = Rc::downgrade(&wallets_model);
        let ui_handle = ui.as_weak();
        {
            ui.global::<WalletAccounts>()
                .on_check_wallet_pswd(move |pswd| match read_wallet(&wallet_folders, &pswd) {
                    Ok(wallets) => {
                        event!(
                            Level::INFO,
                            "open wallet success...{:?}",
                            wallets.iter().map(|w| w.name.as_str()).collect::<Vec<_>>()
                        );
                        if let Some(wallet_list) = wallets_weak.upgrade() {
                            // for wlt in wallets {
                            // wallet_list.push(wlt);
                            // }
                            wallet_list.clear();
                            wallet_list.extend_from_slice(&wallets);
                            let app = ui_handle.upgrade().unwrap();
                            app.set_router_index(2);
                            true
                        } else {
                            error!("wallets model's reference dropped...");
                            false
                        }
                    }
                    Err(e) => {
                        error!("unlock wallet failed...{}", e.root_cause().to_string());
                        false
                    }
                });
        }

        ui.set_router_index(1); // input password to open wallet
    } else {
        event!(Level::INFO, "creating a new wallet...");
        ui.set_router_index(0); // create new wallet
    }

    // async fetch wallet balance and transaction history
    {
        let ui_handle = ui.as_weak();

        ui.global::<WalletAccounts>().on_fetch_history(
            move |is_test: bool, address: SharedString| {
                let net_type = if is_test {
                    NodeType::Testnet
                } else {
                    NodeType::Mainnet
                };
                let ui = ui_handle.upgrade().unwrap();
                let ui_weak = ui.as_weak();
                thread::spawn(move || {
                    match tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(async { fetch_balance(net_type, address.as_str()).await })
                    {
                        Ok(block) => {
                            ui_weak
                                .upgrade_in_event_loop(move |handle| {
                                    let balance: SharedString =
                                        truncat_amount(&block.balance).into();
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_current_balance(balance);

                                    let tranx_vec = block2tranx(block);
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_is_tranx_empty(tranx_vec.is_empty());

                                    let tranx_model = Rc::new(VecModel::from(tranx_vec));
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_tranx_model(tranx_model.into()); // update transactions in UI

                                    // update fetching status in UI
                                    handle.global::<WalletAccounts>().set_fetching(false);
                                })
                                .unwrap();
                        }
                        Err(e) => {
                            error!(
                                "fetch wallet balance failed, {}, {}",
                                address.as_str(),
                                e.root_cause().to_string()
                            );
                            ui_weak
                                .upgrade_in_event_loop(move |handle| {
                                    // update fetching status in UI
                                    handle.global::<WalletAccounts>().set_fetching(false);
                                    handle.global::<WalletAccounts>().set_err_visible(true);
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_err_message(e.root_cause().to_string().into());
                                })
                                .unwrap();
                        }
                    }
                });
            },
        );
    }

    // async fetch transaction block
    {
        let ui_handle = ui.as_weak();

        ui.global::<WalletAccounts>().on_get_tranx_block(
            move |is_test: bool, address: SharedString, direct: TranxType| {
                let net_type = if is_test {
                    NodeType::Testnet
                } else {
                    NodeType::Mainnet
                };
                let ui = ui_handle.upgrade().unwrap();
                let ui_weak = ui.as_weak();
                thread::spawn(move || {
                    match tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(async { fetch_tranx(net_type, address.as_str()).await })
                    {
                        Ok(block) => {
                            ui_weak
                                .upgrade_in_event_loop(move |handle| {
                                    let dir = match direct {
                                        TranxType::Input => "input",
                                        TranxType::Output => "output",
                                        TranxType::Snapshot => "snapshot",
                                    };
                                    let mut tx_address = "".to_string();
                                    for tx in block.block_as_transaction {
                                        if tx.direction == dir {
                                            tx_address = tx.address;
                                            break;
                                        }
                                    }
                                    handle.global::<WalletAccounts>().set_current_block_info(
                                        TransxBlockInfo {
                                            address: tx_address.into(),
                                            hash: block.hash.into(),
                                            fee: truncat_amount(&block.total_fee).into(),
                                            state: block.state.into(),
                                        },
                                    );

                                    handle.global::<WalletAccounts>().set_block_refreshed(true);

                                    // update fetching status in UI
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_block_refreshing(false);
                                })
                                .unwrap();
                        }
                        Err(e) => {
                            error!(
                                "fetch transaction block failed, {}, {}",
                                address.as_str(),
                                e.root_cause().to_string()
                            );
                            ui_weak
                                .upgrade_in_event_loop(move |handle| {
                                    // update fetching status in UI
                                    handle.global::<WalletAccounts>().set_block_refreshed(false);

                                    // update fetching status in UI
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_block_refreshing(false);
                                    handle.global::<WalletAccounts>().set_err_visible(true);
                                    handle
                                        .global::<WalletAccounts>()
                                        .set_err_message(e.root_cause().to_string().into());
                                })
                                .unwrap();
                        }
                    }
                });
            },
        );
    }

    Ok(ui.run()?)
}

// pub fn read_wallet(folders: &Vec<String>, pswd: &str) -> Result<Vec<WalletItem>> {
//     let mut vm = Vec::new();
//     let mut unlocked = 0;
//     // event!(Level::INFO, "Password:{}", pswd);
//     for name in folders {
//         // event!(Level::INFO, "Name:{}", name);
//         let mut wallet = xdag_wallet::XWallet::new();
//         if let Err(_e) = wallet.unlock(pswd, name) {
//             vm.push(WalletItem {
//                 name: name.into(),
//                 locked: true,
//                 balance: "0.0".into(),
//                 ..Default::default()
//             });
//         } else {
//             unlocked += 1;
//             // let key = hex.encode(&wallet.private_key);
//             vm.push(WalletItem {
//                 name: name.into(),
//                 address: wallet.address.into(),
//                 locked: false,
//                 password: pswd.into(),
//                 mnemonic: wallet.mnemonic.into(),
//                 privatekey: hex::encode(wallet.private_key.as_ref()).into(),
//                 balance: "0.0".into(),
//             });
//         }
//     }

//     if unlocked == 0 {
//         Err(XwError::InputPasswordError.into())
//     } else {
//         Ok(vm)
//     }
// }

// parallel read wallets
pub fn read_wallet(folders: &Vec<String>, pswd: &str) -> Result<Vec<WalletItem>> {
    let unlocked = AtomicU16::new(0);
    let vm: Vec<_> = folders
        .par_iter()
        .map(|name| {
            let mut wallet = wallet::XWallet::new();
            if let Err(_e) = wallet.unlock(pswd, Some(name)) {
                WalletItem {
                    name: name.into(),
                    locked: true,
                    balance: "0.0".into(),
                    ..Default::default()
                }
            } else {
                unlocked.fetch_add(1, Ordering::Relaxed);
                WalletItem {
                    name: name.into(),
                    address: wallet.address.into(),
                    locked: false,
                    password: pswd.into(),
                    mnemonic: wallet.mnemonic.into(),
                    privatekey: hex::encode(wallet.private_key.as_ref()).into(),
                    balance: "0.00".into(),
                }
            }
        })
        .collect();

    if unlocked.load(Ordering::Relaxed) == 0 {
        Err(XwError::InputPasswordError.into())
    } else {
        Ok(vm)
    }
}

pub fn change_password(pswd: &str, new_pswd: &str) -> Result<()> {
    let wallet_names = seek_wallet();
    if wallet_names.is_none() {
        error!("changing password: no wallet found");
        return Ok(());
    }
    let folders = wallet_names.unwrap();

    let unlocked = AtomicU16::new(0);
    let _vm: Vec<_> = folders
        .par_iter()
        .map(|name| {
            let mut wallet = wallet::XWallet::new();
            if let Err(e) = wallet.unlock(pswd, Some(name)) {
                event!(
                    Level::ERROR,
                    "changing password: unlock walllet {} error, {}",
                    name,
                    e.root_cause().to_string()
                );
            } else if let Err(e) = wallet.change_password(pswd, new_pswd) {
                event!(
                    Level::ERROR,
                    "change {} wallet password error, {}",
                    name,
                    e.root_cause().to_string()
                );
            } else {
                unlocked.fetch_add(1, Ordering::Relaxed);
            }
        })
        .collect();

    if unlocked.load(Ordering::Relaxed) == 0 {
        Err(XwError::ChangePasswordFailedError.into())
    } else {
        Ok(())
    }
}

pub fn new_wallet(name: String, pswd: String, mnemonic: Option<String>) -> Result<WalletItem> {
    let file_name = gen_file_path(&name);
    if std::path::Path::new(&file_name).exists() {
        return Err(XwError::WalletExist(name).into());
    }

    let mut wallet: XWallet;
    if let Some(m) = mnemonic {
        wallet = import_wallet(&name, &pswd, &m)?; // wallet from mnemonic
    } else {
        wallet = new_hd_wallet(&name, &pswd)?;
    }

    wallet.flush()?;

    Ok(WalletItem {
        name: name.into(),
        address: wallet.address.into(),
        locked: false,
        mnemonic: wallet.mnemonic.into(),
        password: pswd.into(),
        balance: "0.0".into(),
        privatekey: hex::encode(wallet.private_key.as_ref()).into(),
    })
}

// async fn fetch_balance(node: NodeType, address: &str) -> Result<String> {
//     let uri = match node {
//         NodeType::Mainnet => NODE_RPC,
//         NodeType::Testnet => TEST_NODE,
//     };
//     let balance = get_balance(uri, address).await?;
//     Ok(balance)
// }

async fn fetch_balance(node: NodeType, address: &str) -> Result<WalletBlock> {
    let uri = match node {
        NodeType::Mainnet => EXPLORER_URL,
        NodeType::Testnet => TEST_EXPLORER,
    };
    let block = get_history::<WalletBlock>(uri, address, 1).await?;
    Ok(block)
}

async fn fetch_tranx(node: NodeType, address: &str) -> Result<TranxBlock> {
    let uri = match node {
        NodeType::Mainnet => EXPLORER_URL,
        NodeType::Testnet => TEST_EXPLORER,
    };
    let block = get_history::<TranxBlock>(uri, address, 1).await?;
    // event!(Level::INFO, "tranx block {:?}", block);
    Ok(block)
}

// fn block2tranx(block: WalletBlock) -> Vec<TranxDay> {
//     // Iterate through each transaction in the block's address list
//     // and transform it into a TranxDay record.
//     block
//         .block_as_address
//         .into_iter()
//         .map(|tx| {
//             let item_vec = vec![TranxItem {
//                 tranxType: match tx.direction {
//                     Direction::Input => TranxType::Input,
//                     Direction::Output => TranxType::Output,
//                     Direction::Snapshot => TranxType::Snapshot,
//                 },
//                 address: tx.address.into(),
//                 amount: tx.amount.into(),
//                 time: tx.time.clone().into(),
//                 day: tx
//                     .time
//                     .clone()
//                     .split_ascii_whitespace()
//                     .next()
//                     .unwrap()
//                     .into(),
//                 remark: match tx.remark {
//                     Some(ref r) => r.into(),
//                     None => String::new().into(),
//                 },
//             }];
//             let item_model = Rc::new(slint::VecModel::from(item_vec));

//             TranxDay {
//                 day: tx.time.split_ascii_whitespace().next().unwrap().into(),
//                 item: item_model.into(),
//             }
//         })
//         .collect()
// }

// merge tranx of the same day into one TranxDay
fn block2tranx(block: WalletBlock) -> Vec<TranxDay> {
    let mut order: Vec<String> = Vec::new(); // keep tranx's date in order
    let mut map: HashMap<String, Vec<TranxItem>> = HashMap::new();
    for tx in block.block_as_address.into_iter() {
        let time = tx.time.clone(); // clone the time to avoid borrowing issues
        let mut iter: std::str::SplitAsciiWhitespace<'_> = time.split_ascii_whitespace();
        let key: String = iter.next().unwrap().into();
        let time = iter.next().unwrap();
        let time = time[..time.len() - 4].to_string(); // remove the last 4 characters

        let element = TranxItem {
            tranxType: match tx.direction {
                Direction::Input => TranxType::Input,
                Direction::Output => TranxType::Output,
                Direction::Snapshot => TranxType::Snapshot,
            },
            address: tx.address.into(),
            amount: sign_amount(&truncat_amount(&tx.amount), &tx.direction).into(),
            time: (time + " UTC").into(),
            day: key.clone().into(),
            remark: match tx.remark {
                Some(ref r) => r.into(),
                None => SharedString::new(),
            },
        };

        if let Some(items) = map.get_mut(&key) {
            items.push(element);
        } else {
            map.insert(key.clone(), vec![element]);
            order.push(key);
        }
    }

    order
        .into_iter()
        .map(|day| TranxDay {
            day: day.clone().into(),
            item: Rc::new(VecModel::from(map.remove(&day).unwrap())).into(),
        })
        .collect()
}

fn truncat_amount(amount: &str) -> String {
    if let Some(dot_index) = amount.find('.') {
        let integer_part = &amount[..dot_index];
        let decimal_part = &amount[dot_index + 1..];
        let truncated_decimal = if decimal_part.len() > 2 {
            &decimal_part[..2]
        } else {
            decimal_part
        };
        format!("{}.{}", integer_part, truncated_decimal)
    } else {
        amount.to_string() + ".00"
    }
}

async fn transfer_xdag(
    is_test_net: bool,
    mnemonic: &str,
    from: &str,
    to: &str,
    amount: f64,
    remark: &str,
    extra_fee: f64,
) -> Result<TranxBlock> {
    let hash = send_xdag(is_test_net, mnemonic, from, to, amount, remark, extra_fee).await?;
    let uri = if is_test_net {
        TEST_EXPLORER
    } else {
        EXPLORER_URL
    };
    event!(Level::INFO, "send_xdag success: {:?}", &hash);

    let block = get_history::<TranxBlock>(uri, &hash, 1).await?;
    Ok(block)
}

fn sign_amount(amount: &str, direct: &Direction) -> String {
    match direct {
        Direction::Input => "+".to_string() + amount,
        Direction::Output => "-".to_string() + amount,
        Direction::Snapshot => amount.to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;
    #[tokio::test]
    async fn test_block2tranx() {
        if let Ok(res) = get_history::<WalletBlock>(
            "https://explorer.xdag.io/api/block",
            "4RfUFL7XwLi3gaprob6uPUQ41dHbnjcD6",
            2,
        )
        .await
        {
            println!("{:?}", res);
            let tranx = block2tranx(res);
            tranx.iter().for_each(|m| {
                println!(
                    "{:?}--{:?}--{:?}",
                    m.day,
                    m.item.row_count(),
                    m.item.row_data(0)
                )
            });
        } else {
            println!("Error fetching address history")
        }
    }
    #[test]
    fn test_qrcode() {
        let code = QrCode::new(b"4duPWMbYUgAifVYkKDCWxLvRRkSByf5gb").unwrap();
        let image = code
            .render()
            .min_dimensions(240, 240)
            .dark_color(svg::Color("#63D0DF"))
            .light_color(svg::Color("#08080A"))
            .build();
        // println!("{image}");
        let mut svg_file = std::fs::File::create("test.svg").unwrap();
        svg_file.write_all(image.as_bytes()).unwrap();

        let render_image = slint::Image::load_from_svg_data(image.as_bytes()).unwrap();
        assert_eq!(render_image.size().width, 259);
        assert_eq!(render_image.size().height, 259);
    }

    #[test]
    fn test_change_pswd() {
        if let Err(e) = change_password("123456", "111111") {
            println!("Error: {:?}", e.root_cause().to_string());
        } else {
            println!("Password changed successfully");
        }
    }
}
