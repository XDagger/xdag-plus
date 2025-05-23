// use anyhow::Result;
use bip32::{ChildNumber, Mnemonic, XPrv};
use rand_core::OsRng;

// generate bip32 root key and mnemonic
pub fn create_root_key() -> Result<(XPrv, Mnemonic), bip32::Error> {
    // Generate random Mnemonic using the default language (English)
    let phrase = Mnemonic::random(OsRng, Default::default());

    // Derive a BIP39 seed value using the given password
    let seed = phrase.to_seed("");

    // Create a root key
    let root_key = XPrv::new(&seed)?;

    Ok((root_key, phrase))
}
// derive bip44 key from bip32 key
pub fn derive_bip44_key(bip32_key: &XPrv) -> Result<XPrv, bip32::Error> {
    // Derive the BIP32 purpose key (m/44')
    let purpose = ChildNumber::new(44, true)?;
    let purpose_key = bip32_key.derive_child(purpose)?;

    // Derive the BIP44 coin type key (m/44'/coin_type')
    let coin_type = ChildNumber::new(586, true)?;
    let coin_type_key = purpose_key.derive_child(coin_type)?;

    // Derive the BIP44 account key (m/44'/coin_type'/account')
    let account = ChildNumber::new(0, true)?;
    let account_key = coin_type_key.derive_child(account)?;

    // Derive the BIP44 external chain key (m/44'/coin_type'/account'/0')
    let external = ChildNumber::new(0, false)?;
    let external_chain_key = account_key.derive_child(external)?;

    // Derive the BIP44 external address key (m/44'/coin_type'/account'/0/address_index)
    let index = ChildNumber::new(0, false)?;

    let external_address_key = external_chain_key.derive_child(index)?;

    Ok(external_address_key)
}

pub fn key_from_mnemonic(mnemonic: &str) -> Result<XPrv, bip32::Error> {
    let phrase = Mnemonic::new(mnemonic, Default::default())?;
    let seed = phrase.to_seed("");
    // Create a root key
    let bip32_key = XPrv::new(&seed)?;
    let bip44_key = derive_bip44_key(&bip32_key)?;
    Ok(bip44_key)
}
