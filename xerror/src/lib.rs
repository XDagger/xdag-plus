use thiserror::Error;
#[derive(Error, Debug)]
pub enum XwError {
    // xdag wallet error
    #[error("Wallet Not found: {0}")]
    WalletNotFound(String),
    #[error("Wallet Already Exist: {0}")]
    WalletExist(String),
    #[error("Array from Slice Error")]
    ArrayError(#[from] std::array::TryFromSliceError),
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
    #[error("BIP error")]
    BipError(#[from] bip32::Error),
    #[error("History query error")]
    HistoryQueryError(#[from] reqwest::Error),
    // #[error("Node rpc communicate error")]
    // NodeRpcError(#[from] jsonrpsee::core::client::error::Error),
    #[error("Transaction nonce parse error")]
    NonceParseError(#[from] std::num::ParseIntError),
    #[error("bs58 decode error")]
    Bs58DecodeError(#[from] bs58::decode::Error),
    #[error("Password is Empty")]
    NoPassword,
    #[error("Input Password Error")]
    InputPasswordError,
    #[error("Wallet Name is Empty")]
    NoWalletName,

    #[error("Wallet Version Error")]
    VersionDataError,
    #[error("Read Private Key Error")]
    ReadPrivKeyError,
    #[error("Remark format Error")]
    RemarkFormatError,
    #[error("Less than fee Error")]
    LessThanFeeError,

    #[error("Config file toml serialize Error")]
    ConfigSerError(#[from] toml::ser::Error),
    #[error("Config file toml deserialize Error")]
    ConfigDeserError(#[from] toml::de::Error),
    #[error("Config file parse utf-8 Error")]
    ConfigParseError(#[from] std::string::FromUtf8Error),
    #[error("Config file location Error")]
    ConfigLocationError,
    #[error("Config file path to str Error")]
    ConfigPath2StrError,

    #[error("Address length Error")]
    AddressLengthError,
    #[error("Address invalid character Error")]
    AddressInvalidCharacter,
    #[error("Address to bytes overflow  Error")]
    AddressOverflow,

    #[error("Rpc error: {0}")]
    RpcError(String),
    #[error("Mnemonic is invalid")]
    MnemonicInvalidError,
    #[error("Change wallet password failed")]
    ChangePasswordFailedError,
}
