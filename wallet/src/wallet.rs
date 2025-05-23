use super::bip44::key_from_mnemonic;
use anyhow::Result;
use bcrypt::*;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crypto::*;
use directories::ProjectDirs;
use rand::prelude::*;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use xerror::XwError;

const VERSION: u32 = 4;
const SALT_LENGTH: usize = 16;
const BCRYPT_COST: u32 = 12;
pub const DATA_PATH: &str = "wallet_accounts";
const DATA_FILE_NAME: &str = "xdagj_wallet.bin";

#[derive(Clone, Debug)]
pub struct XWallet {
    // pub lock: bool,
    pub password: String,
    pub private_key: [u8; 32],
    pub public_key: [u8; 33],
    pub mnemonic: String,
    pub name: String,
    pub file: String,
    pub hash160: [u8; 20],
    pub address: String,
    pub aes_key: [u8; 24],
}
impl Default for XWallet {
    fn default() -> Self {
        Self::new()
    }
}
impl XWallet {
    pub fn new() -> Self {
        XWallet {
            password: "".to_string(),
            address: "".to_string(),
            mnemonic: "".to_string(),
            file: "".to_string(),
            name: "".to_owned(),
            private_key: [0_u8; 32],
            public_key: [0_u8; 33],
            hash160: [0_u8; 20],
            aes_key: [0_u8; 24],
            // lock: true,
        }
    }
    pub fn unlock(&mut self, password: &str, name: &str) -> Result<()> {
        if password.is_empty() {
            return Err(XwError::NoPassword.into());
        }
        if name.is_empty() {
            return Err(XwError::NoWalletName.into());
        }
        let file_name = gen_file_path(name);
        let file_path = std::path::Path::new(&file_name);
        if !file_path.exists() {
            return Err(XwError::WalletNotFound(file_name).into());
        }
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut reader = Cursor::new(&buffer);

        let version = reader.read_u32::<BigEndian>()?;
        if version != 4 {
            return Err(XwError::VersionDataError.into());
        }

        let salt = read_bytes(&mut reader)?;
        self.aes_key = bcrypt(
            BCRYPT_COST,
            salt.as_slice().try_into()?,
            password.as_bytes(),
        );

        self.read_account(&mut reader)?; // read private key
        self.read_seed(&mut reader)?; // read mnemonic string

        let bip44_key = key_from_mnemonic(&self.mnemonic)?;
        // let mut pkey = [0_u8; 32];
        // for (i, b) in bip44_key.private_key().to_bytes().into_iter().enumerate() {
        //     pkey[i] = b;
        // }
        let private_key: [u8; 32] = bip44_key.private_key().to_bytes().as_slice().try_into()?;

        if private_key != self.private_key {
            return Err(XwError::ReadPrivKeyError.into());
        }
        let public_key = bip44_key.public_key().to_bytes();
        self.public_key = public_key;
        self.hash160 = hash160(&public_key);
        self.address = bs58::encode(&self.hash160).with_check().into_string();
        self.password = password.into();
        self.file = file_name;
        self.name = name.into();

        Ok(())
    }

    fn read_account<T: Read>(&mut self, reader: &mut T) -> Result<(), XwError> {
        _ = reader.read_u32::<BigEndian>()?; // account quantity
        let iv = read_bytes(reader)?;
        let cipher = read_bytes(reader)?;
        self.private_key = aes_cbc_decrypt(&cipher, self.aes_key, iv.as_slice().try_into()?)?
            .as_slice()
            .try_into()?;
        Ok(())
    }

    fn read_seed<T: Read>(&mut self, reader: &mut T) -> Result<(), XwError> {
        let iv = read_bytes(reader)?;
        let cipher = read_bytes(reader)?;
        let decrypt = aes_cbc_decrypt(&cipher, self.aes_key, iv.as_slice().try_into()?)?;
        let mut cursor = Cursor::new(&decrypt);
        let mnemonic = read_bytes(&mut cursor)?;
        self.mnemonic = String::from_utf8_lossy(&mnemonic).trim().into();
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        let buffer: Vec<u8> = Vec::new();
        let mut writer = Cursor::new(buffer);
        writer.write_u32::<BigEndian>(VERSION)?;

        let mut rng = rand::thread_rng();
        let mut salt = vec![0_u8; SALT_LENGTH];
        rng.fill_bytes(&mut salt);
        writer.write_all(&sized_bytes(&salt))?;

        self.aes_key = bcrypt(
            BCRYPT_COST,
            salt.as_slice().try_into()?,
            self.password.as_bytes(),
        );

        self.write_account(&mut writer, &mut rng)?; // write private key
        self.write_seed(&mut writer, &mut rng)?; // write mnemonic

        let path = std::path::Path::new(&self.file);
        let prefix = path.parent().unwrap();
        if !prefix.exists() {
            std::fs::create_dir_all(prefix)?;
        }

        let mut file = File::create(&self.file)?;
        file.write_all(&writer.into_inner())?;

        Ok(())
    }

    // write bytes of private key of an account
    fn write_account<T: Write>(&self, writer: &mut T, rng: &mut ThreadRng) -> Result<(), XwError> {
        let key_count = 1_u32;
        writer.write_u32::<BigEndian>(key_count)?;

        let mut iv = vec![0_u8; 16];
        rng.fill_bytes(&mut iv);
        writer.write_all(&sized_bytes(&iv))?;

        let prv_key = aes_cbc_encrypt(&self.private_key, self.aes_key, iv.as_slice().try_into()?);
        writer.write_all(&sized_bytes(&prv_key))?;
        Ok(())
    }

    fn write_seed<T: Write>(&self, writer: &mut T, rng: &mut ThreadRng) -> Result<(), XwError> {
        let mut iv = vec![0_u8; 16];
        rng.fill_bytes(&mut iv);
        writer.write_all(&sized_bytes(&iv))?;

        let mut seed_writer = Cursor::new(Vec::new());
        let mnemonic = self.mnemonic.clone();
        seed_writer.write_all(&sized_bytes(&mnemonic.into_bytes()))?;

        let next_index = 1_u32;
        seed_writer.write_u32::<BigEndian>(next_index)?;

        let encrypted = aes_cbc_encrypt(
            &seed_writer.into_inner(),
            self.aes_key,
            iv.as_slice().try_into()?,
        );
        writer.write_all(&sized_bytes(&encrypted))?; // write encrypted mnemonic

        Ok(())
    }

    pub fn import_from_mnemonic(&mut self, mnemonic: &str) -> Result<()> {
        if self.password.is_empty() {
            return Err(XwError::NoPassword.into());
        }
        if self.name.is_empty() {
            return Err(XwError::NoWalletName.into());
        }

        let trimed = mnemonic.trim();
        let bip44_res = key_from_mnemonic(trimed);
        if bip44_res.is_err() {
            return Err(XwError::MnemonicInvalidError.into());
        }

        let file_name = gen_file_path(&self.name);
        let file_path = std::path::Path::new(&file_name).parent().unwrap();
        if file_path.exists() {
            return Err(XwError::WalletExist(self.name.clone()).into());
        }
        std::fs::create_dir_all(file_path)?;

        let bip44_key = bip44_res.unwrap();
        // let mut pkey = [0_u8; 32];
        // for (i, b) in bip44_key.private_key().to_bytes().into_iter().enumerate() {
        //     pkey[i] = b;
        // }
        let private_key: [u8; 32] = bip44_key.private_key().to_bytes().as_slice().try_into()?;
        let public_key = bip44_key.public_key().to_bytes();
        self.file = file_name;
        self.private_key = private_key;
        self.mnemonic = trimed.into();
        self.public_key = public_key;
        self.hash160 = hash160(&public_key);
        self.address = bs58::encode(&self.hash160).with_check().into_string();
        self.flush()?;
        Ok(())
    }

    pub fn export_mnemonic(&self, file: &str) -> Result<(), XwError> {
        let trimed = self.mnemonic.trim();
        let path = std::path::Path::new(file);
        let prefix = path.parent().unwrap();
        if !prefix.exists() {
            std::fs::create_dir_all(prefix)?;
        }

        let mut file = File::create(file)?;
        file.write_all(trimed.as_bytes())?;
        Ok(())
    }

    pub fn change_password(&mut self, old: &str, new: &str) -> Result<()> {
        if self.password != old {
            return Err(XwError::InputPasswordError.into());
        }
        if self.name.is_empty() {
            return Err(XwError::NoWalletName.into());
        }
        if new.is_empty() {
            return Err(XwError::NoPassword.into());
        }
        self.password = new.to_string();
        self.flush()
    }
}

pub fn gen_file_path(name: &str) -> String {
    ProjectDirs::from("com", "xdagger", "xdag plus")
        .unwrap()
        .config_dir()
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string()
        + "/"
        + DATA_PATH
        + "/"
        + name
        + "/"
        + DATA_FILE_NAME
}
// read size of a vector befor read the vector
fn bytes2size<T: Read>(reader: &mut T) -> Result<u32, XwError> {
    let mut size = 0_u32;
    let mut i = 0;
    while i < 4 {
        let b = reader.read_u8()?;
        size = (size << 7) | (b & 0x7f) as u32;
        if (b & 0x80) == 0 {
            break;
        }
        i += 1;
    }
    Ok(size)
}

// vector's u32 size to bytes for write vector to file
fn size2bytes(size: u32) -> Vec<u8> {
    let mut s = size;
    let mut b = [0_u8; 4];
    let mut i = 4_usize;
    // b[3] = (s & 0x7f) as u8;
    // s >>= 7;

    while s > 0 {
        i -= 1;
        b[i] = (s & 0x7f) as u8;
        s >>= 7;
    }
    let c = i;
    while i < 4 {
        if i != 3 {
            b[i] |= 0x80;
        }
        i += 1;
    }
    b[c..].into()
}

fn sized_bytes(buf: &[u8]) -> Vec<u8> {
    let size_bytes = size2bytes(buf.len() as u32);
    let mut result = Vec::with_capacity(size_bytes.len() + buf.len());
    result.extend_from_slice(&size_bytes);
    result.extend_from_slice(buf);
    result
}

fn read_bytes<T: Read>(reader: &mut T) -> Result<Vec<u8>, XwError> {
    let size = bytes2size(reader)?;
    let mut buff = vec![0_u8; size as usize];
    reader.read_exact(&mut buff)?;
    Ok(buff)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_flush() {
        let mut wallet = XWallet::new();
        wallet.file = "./my_wallet.bin".to_string();
        wallet.password = "1234567890".to_string();

        let ret = wallet.flush();
        println!("{:?}", ret);
    }

    #[test]
    fn test_unlock() {
        let mut wallet = XWallet::new();
        // wallet.file = "./my_wallet.bin".to_string();
        // wallet.password = "1234567890".to_string();

        let ret = wallet.unlock("123456", "aaa");
        println!("{:?}", ret);
        println!("{:?}", wallet);
    }

    #[test]
    fn test_size_bytes() {
        let b = size2bytes(444_u32);
        let mut reader = Cursor::new(&b);
        let size = bytes2size(&mut reader).unwrap();
        assert_eq!(size, 444_u32);
        assert_eq!(reader.position(), 2);
    }

    #[test]
    fn test_change_pswd() {
        let mut wallet = XWallet::new();
        let ret = wallet.unlock("111", "aaa");
        println!("{:?}", ret);
        println!("{:?}", wallet);

        let res = wallet.change_password("111", "123456");
        println!("{:?}", res);
        println!("{:?}", wallet);
    }
}
