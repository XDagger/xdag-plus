use bip32::secp256k1::ecdsa::{signature::Signer, Signature};
use byteorder::{LittleEndian, WriteBytesExt};
use crypto::sha256;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use wallet::bip44;
use xerror::XwError;

use std::io::{Cursor, Seek, SeekFrom, Write};
use std::time::{Duration, SystemTime};

const MIME2BITS: [u8; 256] = {
    //"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/" to bytes array
    let mut table = [0xFF; 256];
    let mut i = 0u8;
    while i < 64 {
        let c = match i {
            0..=25 => b'A' + i,
            26..=51 => b'a' + i - 26,
            52..=61 => b'0' + i - 52,
            62 => b'+',
            63 => b'/',
            _ => unreachable!(),
        };
        table[c as usize] = i;
        i += 1;
    }
    table
};

const FEE: f64 = 0.1;

const TEST_NODE: &str = "https://testnet-rpc.xdagj.org";
const NODE_RPC: &str = "https://mainnet-rpc.xdagj.org";

pub async fn get_balance(is_test_net: bool, address: &str) -> Result<String, XwError> {
    let uri = if is_test_net { TEST_NODE } else { NODE_RPC };
    let client = HttpClientBuilder::default()
        .request_timeout(Duration::from_secs(18))
        .build(uri)?;
    let res: String = client
        .request("xdag_getBalance", rpc_params![address.to_string()])
        .await?;

    Ok(res)
}

pub async fn send_transaction(uri: &str, block: &str) -> Result<String, XwError> {
    let client = HttpClientBuilder::default()
        .request_timeout(Duration::from_secs(18))
        .build(uri)?;
    let res: String = client
        .request("xdag_sendRawTransaction", rpc_params![block.to_string()])
        .await?;

    Ok(res)
}

fn transaction_block(
    is_test_net: bool,
    amount: f64,
    from: &str,
    to: &str,
    remark: &str,
    key: &bip32::XPrv,
    nonce: u64,
) -> Result<String, XwError> {
    if amount < FEE {
        return Err(XwError::LessThanFeeError);
    }
    let mut from_address = bs58::decode(from).with_check(None).into_vec()?;
    from_address.reverse();
    let mut to_address = bs58::decode(to).with_check(None).into_vec()?;
    to_address.reverse();
    check_remark(remark)?;

    let buffer: Vec<u8> = vec![0_u8; 512];
    let mut writer = Cursor::new(buffer);
    // header: transport
    writer.seek(SeekFrom::Start(8)).unwrap();

    // header: field types
    let even = key.public_key().to_bytes()[0] == 0x02;
    writer.write_u64::<LittleEndian>(get_fields_type(is_test_net, !remark.is_empty(), even))?;

    // header: timestamp
    let t = get_timestamp();
    writer.write_u64::<LittleEndian>(t)?;

    // if is_test_net {
    // header: fee, nonce
    writer.seek(SeekFrom::Current(32)).unwrap();
    writer.write_u64::<LittleEndian>(nonce)?;
    // } else {
    //     writer.seek(SeekFrom::Current(8)).unwrap();
    // }

    // input field: input address
    writer.write_u32::<LittleEndian>(0)?;
    writer.write_all(&from_address)?;

    // input field: input value
    let value = amount_to_xdag(amount);
    writer.write_u64::<LittleEndian>(value)?;

    // output field: output address
    writer.write_u32::<LittleEndian>(0)?;
    writer.write_all(&to_address)?;

    // output field: out value
    writer.write_u64::<LittleEndian>(value)?;

    // remark field
    if !remark.is_empty() {
        writer.write_all(remark.as_bytes())?;
        // if is_test_net {
        writer.seek(SeekFrom::Start(160)).unwrap();
        // } else {
        //     writer.seek(SeekFrom::Start(128)).unwrap();
        // }
    }
    // public key field
    writer.write_all(&key.public_key().to_bytes()[1..33])?;

    // sign field: sign_r
    let signature = sign_transaction(writer.clone().into_inner().as_slice(), key);
    let r = signature.r();
    writer.write_all(&r.to_bytes())?;

    // sign field: sign_s
    let s = signature.s();
    writer.write_all(&s.to_bytes())?;

    Ok(hex_encode(writer.into_inner().as_slice()))
    // Ok(writer
    //     .into_inner()
    //     .iter()
    //     .map(|byte| format!("{:02x}", byte))
    //     .collect())
}

pub fn check_remark(remark: &str) -> Result<(), XwError> {
    if remark.is_empty() {
        return Ok(());
    }
    if remark.is_ascii() && remark.len() < 33 {
        return Ok(());
    }
    Err(XwError::RemarkFormatError)
}

fn amount_to_xdag(value: f64) -> u64 {
    let amount = value.floor() as u64;
    let integer = amount << 32;
    let digit = value - (amount as f64);
    let digit = digit * (1_u64 << 32) as f64;
    let digit = digit.ceil() as u64;

    digit + integer
}

fn get_timestamp() -> u64 {
    let duration_since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let timestamp_nanos = duration_since_epoch.as_nanos() as u64;
    let sec = timestamp_nanos / (1e9 as u64);
    let usec = (timestamp_nanos - sec * (1e9 as u64)) / (1e3 as u64);
    let xmsec = (usec << 10) / (1e6 as u64);
    (sec << 10) | xmsec
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{b:02x}");
        output
    })
}

fn sign_transaction(block: &[u8], key: &bip32::XPrv) -> Signature {
    let priv_key = key.private_key();
    let public_key = key.public_key();
    let mut v = vec![];
    v.extend_from_slice(block);
    v.extend_from_slice(&public_key.to_bytes());
    // sign double sha256 of block+pub_key
    // priv_key.sign itself is a sha256WithEcdsa
    priv_key.sign(&sha256(&v))
}

pub async fn get_tranx_nonce(uri: &str, address: &str) -> Result<u64, XwError> {
    let client = HttpClientBuilder::default()
        .request_timeout(Duration::from_secs(18))
        .build(uri)?;
    let res: String = client
        .request("xdag_getTransactionNonce", rpc_params![address.to_string()])
        .await?;
    let nonce: u64 = res.parse()?;
    Ok(nonce)
}

pub async fn send_xdag(
    is_test_net: bool,
    mnemonic: &str,
    from: &str,
    to: &str,
    amount: f64,
    remark: &str,
) -> Result<String, XwError> {
    let url = if is_test_net { TEST_NODE } else { NODE_RPC };
    let nonce = get_tranx_nonce(url, from).await?;
    let key = bip44::key_from_mnemonic(mnemonic)?;
    let block = transaction_block(is_test_net, amount, from, to, remark, &key, nonce)?;
    let res = send_transaction(url, &block).await?;
    if address_to_hash(&res).is_err() {
        return Err(XwError::RpcError(res));
    }
    Ok(res)
}

fn get_fields_type(_is_test_net: bool, has_remark: bool, pub_key_even: bool) -> u64 {
    // if is_test_net {
    // 1--E--C--D--[9]--6/7--5--5
    // header--tranx_nonce--input--output--[remark]--pubKey(even/odd)--sign_r--sign_s
    let mut fields = 0xdce1_u64;

    let mut keys = 0x550_u64;
    if pub_key_even {
        keys |= 0x06_u64;
    } else {
        keys |= 0x07_u64;
    }
    keys <<= 16;

    if has_remark {
        keys <<= 4;
        fields |= 0x90000_u64;
    }
    fields |= keys;

    fields
    // } else {
    //     // 1--C--D--[9]--6/7--5--5
    //     // header--input--output--[remark]--pubKey(even/odd)--sign_r--sign_s
    //     let mut fields = 0xdc1_u64;
    //
    //     let mut keys = 0x550_u64;
    //     if pub_key_even {
    //         keys |= 0x06_u64;
    //     } else {
    //         keys |= 0x07_u64;
    //     }
    //     keys <<= 12;
    //
    //     if has_remark {
    //         keys <<= 4;
    //         fields |= 0x9000_u64;
    //     }
    //     fields |= keys;
    //
    //     fields
    // }
}

pub fn address_to_hash(addr: &str) -> Result<[u8; 32], XwError> {
    if addr.len() != 32 {
        return Err(XwError::AddressLengthError);
    }

    let mut hash = [0u8; 32];
    let mut buffer = 0u32;
    let mut bits_in_buffer = 0u32;
    let mut index = 0usize;

    for c in addr.chars() {
        if !c.is_ascii() {
            return Err(XwError::AddressInvalidCharacter);
        }
        let value = MIME2BITS[c as usize];
        if value & 0xC0 != 0 {
            return Err(XwError::AddressInvalidCharacter);
        }

        buffer = (buffer << 6) | (value as u32);
        bits_in_buffer += 6;

        while bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            let byte = (buffer >> bits_in_buffer) as u8;
            if index >= 24 {
                return Err(XwError::AddressOverflow);
            }
            hash[index] = byte;
            index += 1;
        }
    }

    if index != 24 {
        return Err(XwError::AddressOverflow);
    }

    // Set last 8 bytes to zero
    hash[24..].fill(0);

    Ok(hash)
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_get_balance() {
        // let url = "https://testnet-rpc.xdagj.org";
        let address = "Fii9BuhR1KogfNzWbtSH1YJgQQDwFMomK";

        if let Ok(res) = get_balance(true, address).await {
            println!("balance: {:}", res);
        } else {
            println!("test get balance error");
        }
    }

    #[test]
    fn test_check_address() {
        let res = bs58::decode("Fii9BuhR1KogfNzWbtSH1YJgQQDwFMomK")
            .with_check(None)
            .into_vec();
        println!("{:?}", res);
    }

    #[test]
    fn test_xdag_amount() {
        let xdags = amount_to_xdag(1.1);
        println!("{xdags:016x}");

        let xdags = amount_to_xdag(1_f64);
        println!("{xdags:016x}");

        let xdags = amount_to_xdag(0.5);
        println!("{xdags:016x}");

        let xdags = amount_to_xdag(10_f64);
        println!("{xdags:016x}");

        let xdags = amount_to_xdag(100_f64);
        println!("{xdags:016x}");
    }

    #[test]
    fn test_hex_encode() {
        let v = vec![127_u8, 15_u8, 7_u8, 3_u8];
        println!("{}", hex_encode(&v));
    }

    #[test]
    fn test_get_timestamp() {
        let t = get_timestamp();
        let t = t.swap_bytes(); // for print little-endian bytes order
        println!("{t:016x}");
    }

    #[test]
    fn test_fields_type() {
        let t = get_fields_type(false, true, true);
        let t = t.swap_bytes(); // for print little-endian bytes order
        println!("{t:016x}");

        let t = get_fields_type(true, false, false);
        let t = t.swap_bytes(); // for print little-endian bytes order
        println!("{t:016x}");
    }

    #[test]
    fn test_transaction_block() {
        let mnemonic =
            "caught industry sorry science symbol life club sausage kitten tourist shadow transfer";
        if let Ok(key) = wallet::bip44::key_from_mnemonic(mnemonic) {
            let from = "Fii9BuhR1KogfNzWbtSH1YJgQQDwFMomK";
            let to = "Fve2AF8NrEPjNcAj5BABTBeqn7LW7WfeT";
            let block = transaction_block(true, 1.5, from, to, "hello", &key, 111);
            match block {
                Ok(s) => println!("{}", s),
                Err(e) => println!("{}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_send_transaction() {
        let url = "https://testnet-rpc.xdagj.org";

        let mnemonic =
            "caught industry sorry science symbol life club sausage kitten tourist shadow transfer";
        let key = wallet::bip44::key_from_mnemonic(mnemonic).unwrap();
        let from = "Fii9BuhR1KogfNzWbtSH1YJgQQDwFMomK";
        let nonce = get_tranx_nonce(url, from).await.unwrap();
        let to = "Fve2AF8NrEPjNcAj5BABTBeqn7LW7WfeT";
        let block = transaction_block(true, 1.0, from, to, "hello", &key, nonce).unwrap();
        println!("{}", block);
        let res = send_transaction(url, &block).await;
        match res {
            Ok(s) => println!("{}", s),
            Err(e) => println!("{}", e),
        }
    }

    #[test]
    fn test_valid_address() {
        let addr = "ABCDabcdwxyz0123456789+/ABCDEFGH";
        let result = address_to_hash(addr);
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash[24..], [0u8; 8]);
    }

    #[test]
    fn test_length_error() {
        let addr_short = "short";
        let addr_long = "this_is_a_very_long_address_that_exceeds_32_chars";
        println!("{:?}", address_to_hash(addr_short)); // print AddressLengthError
        println!("{:?}", address_to_hash(addr_long));
    }

    #[test]
    fn test_invalid_character() {
        let addr = "ABCD@abcdwxyz0123456789+/ABCDEFGHIJKL"; // '@' is invalid
        println!("{:?}", address_to_hash(addr)); // print AddressInvalidCharacter
    }

    #[test]
    fn test_all_zeros() {
        let addr = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"; // 32 'A's
        let hash = address_to_hash(addr).unwrap();
        assert_eq!(hash[..24], [0u8; 24]);
        assert_eq!(hash[24..], [0u8; 8]);
    }
}
