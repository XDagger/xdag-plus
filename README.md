# Rust XDAG Wallet

This is a cross-platform XDAG GUI wallet, powered by `Rust` and `Slint UI`.

![slint ui icon](https://github.com/slint-ui/slint/raw/master/logo/slint-logo-full-light.svg)

## bip32

bip32 package forked from bip32 crate and changed mnemonic words length.

It supports new 12 words generation and 12 ~ 24 words importation.

## Platforms

support Linux, Windows, MacOS ( Android and Wasm are in the plan)


## Build GUI Wallet

```bash
cargo build --release --package app
```

## Build JsonRpc Server

```bash
cargo build --release --package server
```

## Server usage
usage: by command-line parameter -help

first time run server, you need to import a mnemonic file to create a wallet.
```bash
server --ip <ip address> --port <port number> --mnemonic <mnemonic file path>
```

add --test-net command line parameter when using test net.
```bash
server --ip <ip address> --port <port number> --test-net
```

- jsonrpc server:  server --ip \<ip address\> --port \<port number\>
  - method: Xdag.Unlock
    - params: ["\<wallet password\>"]
    - response: {"id":1,"result":"success"}
  - method: Xdag.Lock
    - params: ["\<wallet password\>"]
    - response: {"id":1,"result":"success"}
  - method: Xdag.Account
    - params: [""]
    - response: {"id":1,"result": "\<wallet address\>"}
  - method: Xdag.Balance
    - params: [""]
    - response: {"id":1,"result": "\<wallet balance\>"}
  - method: Xdag.Balance
    - params: ["\<wallet address\>"]
    - response: {"id":1,"result": "\<balance of the address\>"}
  - method: Xdag.Transfer
    - params: [{"amount":"\<amount\>","address":"\<to address\>","remark":"\<remark\>","express_fee":"\<express fee\>"}]
    - response: {"id":1,"result": {"Status":"success","TxHash":"\<transaction hash\>"}}


## Acknowlegement

<https://github.com/syf20020816/SurrealismUI>
