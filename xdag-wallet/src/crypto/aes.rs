use std::fmt::Write;
use std::io::{Error, ErrorKind};

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
type Aes192CbcEnc = cbc::Encryptor<aes::Aes192>;
type Aes192CbcDec = cbc::Decryptor<aes::Aes192>;

const AES_BLOCK_SIZE: usize = 16;

pub fn aes_cbc_decrypt(ciphertext: &[u8], key: [u8; 24], iv: [u8; 16]) -> Result<Vec<u8>, Error> {
    let mut buf = ciphertext.to_owned();

    let pt = Aes192CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .unwrap();
    let pad_trim = pkcs5_trim(pt)?;
    Ok(pad_trim.to_vec())
}

pub fn aes_cbc_encrypt(plaintext: &[u8], key: [u8; 24], iv: [u8; 16]) -> Vec<u8> {
    let mut buf = pkcs5_pad(plaintext, AES_BLOCK_SIZE);
    let pt_len = buf.len();

    let ct = Aes192CbcEnc::new(&key.into(), &iv.into())
        .encrypt_padded_mut::<NoPadding>(&mut buf, pt_len)
        .unwrap();

    ct.to_vec()
}

fn pkcs5_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - data.len() % block_size;
    let mut padded_data = data.to_vec();
    padded_data.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    padded_data
}

fn pkcs5_trim(data: &[u8]) -> Result<&[u8], Error> {
    let pad_len = data[data.len() - 1] as usize;
    if pad_len >= data.len() {
        Err(Error::new(ErrorKind::InvalidData, "pkcs5 trim error"))
    } else {
        Ok(&data[..data.len() - pad_len])
    }
}
pub fn vec_u8_to_hex(vec: &Vec<u8>) -> String {
    let mut s = String::new();
    for byte in vec {
        write!(s, "{:02x}", byte).expect("Unable to write to string");
    }
    s
}

fn vec_u8_to_ascii_string(vec: &Vec<u8>) -> String {
    String::from_utf8_lossy(vec).to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_aes() {
        let plain = b"1234567890";
        let mut key = [48_u8; 24];
        let iv = [48_u8; 16];

        let cipher = aes_cbc_encrypt(plain, key, iv);
        println!("{:?}", vec_u8_to_hex(&cipher));

        let pt = aes_cbc_decrypt(&cipher, key, iv).unwrap();
        println!("{:?}", vec_u8_to_ascii_string(&pt));

        key[0] += 1;
        match aes_cbc_decrypt(&cipher, key, iv) {
            Ok(pt) => println!("{:?}", vec_u8_to_ascii_string(&pt)),
            Err(e) => println!("{:?}", e),
        }
    }
}
