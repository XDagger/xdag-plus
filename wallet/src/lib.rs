use anyhow::Result;
use directories::ProjectDirs;
use std::fs;

pub mod bip44;
use self::bip44::{create_root_key, derive_bip44_key};

mod wallet;
pub use self::wallet::*;

use crypto::hash160;

pub fn seek_wallet() -> Option<Vec<String>> {
    if let Some(proj_dir) = ProjectDirs::from("com", "xdagger", "xdag plus") {
        let path = proj_dir.config_dir().join(DATA_PATH);

        let mut names = Vec::new();
        // println!("{:?}", path);
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let metadata = entry.metadata().unwrap();
                let name = entry.file_name().into_string().unwrap();
                let file_name = gen_file_path(&name);
                if metadata.is_dir() && std::path::Path::new(&file_name).exists() {
                    names.push(name);
                }
            }
            if names.is_empty() {
                None
            } else {
                Some(names)
            }
        } else {
            println!("Failed to read wallet directory.");
            None
        }
    } else {
        println!("Failed to find wallet directory.");
        None
    }
}

pub fn new_hd_wallet(name: &str, pswd: &str) -> Result<XWallet> {
    let (bip32_key, mnemonic) = create_root_key()?;
    let bip44_key = derive_bip44_key(&bip32_key)?;

    // let mut pkey = [0_u8; 32];
    // for (i, b) in bip44_key.private_key().to_bytes().into_iter().enumerate() {
    //     pkey[i] = b;
    // }
    let pkey: [u8; 32] = bip44_key.private_key().to_bytes().as_slice().try_into()?;

    let public_key_bytes = bip44_key.public_key().to_bytes();
    let hash160 = hash160(&public_key_bytes);
    let address = bs58::encode(hash160).with_check().into_string();
    Ok(XWallet {
        is_test: false,
        password: pswd.to_string(),
        file: gen_file_path(name),
        mnemonic: mnemonic.phrase().to_string(),
        private_key: pkey,
        public_key: public_key_bytes,
        name: Some(name.into()),
        hash160,
        address,
        aes_key: [0_u8; 24],
    })
}

pub fn import_wallet(name: &str, password: &str, mnemonic: &str) -> Result<XWallet> {
    let mut wallet = XWallet::new();
    wallet.name = Some(name.into());
    wallet.password = password.into();
    wallet.import_from_mnemonic(mnemonic)?;
    Ok(wallet)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_new_wallet() {
        if let Ok(mut wallet) = new_hd_wallet("aaa", "123456") {
            let res = wallet.flush();
            println!("{:?}", wallet);
            println!("{:?}", res);
        }
    }

    #[test]
    // delete wallet account folder "bbb", before test
    fn test_import_wallet() {
        let mnemonic = "game taxi bag mixture ready smile toward short dog ask very balance pigeon census boat";
        if let Ok(wallet) = import_wallet("bbb", "123456", mnemonic) {
            println!("{:?}", wallet);

            let mut wallet2 = XWallet::new();

            let ret = wallet2.unlock("123456", Some("bbb"));
            println!("{:?}", ret);
            println!("{:?}", wallet2);
        } else {
            println!("import wallet error");
        }
    }
    #[test]
    fn test_import_wallet2() {
        let mnemonic =
            "caught industry sorry science symbol life club sausage kitten tourist shadow transfer";
        if let Ok(wallet) = import_wallet("ccc", "123456", mnemonic) {
            println!("{:?}", wallet);

            let mut wallet2 = XWallet::new();

            let ret = wallet2.unlock("123456", Some("ccc"));
            println!("{:?}", ret);
            println!("{:?}", wallet2);
        } else {
            println!("import wallet error");
        }
    }
}
