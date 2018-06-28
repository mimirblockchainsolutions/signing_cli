#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate docopt;
extern crate mimir_crypto;
extern crate mimir_common;
extern crate rand;
extern crate toml;
extern crate rlp;
extern crate reqwest;
extern crate ring;
extern crate crypto;

mod transact;

use transact::RawTxBuilder;

use mimir_crypto::secp256k1::Signer;
use mimir_crypto::secp256k1::Public;
use mimir_crypto::secp256k1::Secret;
use mimir_crypto::secp256k1::Address;
use mimir_crypto::keccak256::Keccak256;
use mimir_common::types::{Bytes, U256, H256};
use std::fs;
use serde_json::Value;
use reqwest::Client;
use docopt::Docopt;
use crypto::symmetriccipher::Decryptor;
use crypto::blockmodes::CtrMode;
use crypto::aessafe::AesSafe128Encryptor;
use crypto::buffer::{RefReadBuffer, RefWriteBuffer};


/// get signer for parity dev chain
fn dev_signer() -> Signer {
    let dev_secret = mimir_crypto::secp256k1::dev::SECRET;
    Signer::new(dev_secret).expect("dev secret always valid")
}

#[derive(Debug, Serialize, Deserialize)]
struct Keys {
    public: Public,
    secret: Secret,
    address: Address,
}

fn keygen() -> Keys {
    let signer: Signer = rand::random();
    Keys {
        public: signer.public(),
        secret: signer.secret(),
        address: signer.address(),
    }
}

fn store_keys(keys: Keys) -> Result<(), Error> {
    let toml = toml::to_string(&keys)?;
    fs::write("keys.toml", toml)?;
    Ok(())
}

fn retrieve_keys_toml() -> Result<Keys, Error> {
    let read = fs::read_to_string("keys.toml")?;
    let toml: Keys = toml::from_str(&read)?;
    Ok(toml)
}

#[derive(Deserialize, Debug)]
struct Wallet {
    crypto: Crypto,
}

#[derive(Deserialize, Debug)]
struct Crypto {
    ciphertext: H256,
    cipherparams: Cypherparams,
    kdf: String,
    kdfparams: Kdfparams,
    mac: H256,
}

#[derive(Deserialize, Debug)]
struct Cypherparams {
    iv: Bytes,
}

#[derive(Deserialize, Debug)]
struct Kdfparams {
    c: u32,
    dklen: u32,
    prf: String,
    salt: H256,
}

fn retrieve_keys_json(path: &str) -> Result<Wallet, Error> {
    let read = fs::read_to_string(path)?;
    let wallet: Wallet = serde_json::from_str(&read)?;
    Ok(wallet)
}

fn decrypt_wallet(wallet: Wallet, password: &str) -> Result<Secret, Error> {
    let mut derived_key = [0u8; 32];
    ring::pbkdf2::derive(
        &ring::digest::SHA256,
        wallet.crypto.kdfparams.c,
        &wallet.crypto.kdfparams.salt,
        password.as_ref(),
        &mut derived_key[..],
    );
    let right_bits = &derived_key[0..16];
    let left_bits = &derived_key[16..32];
    right_bits.to_vec();
    left_bits.to_vec();

    let mut mac_buff = vec![0u8; 16 + wallet.crypto.ciphertext.len()];
    mac_buff[0..16].copy_from_slice(left_bits);
    mac_buff[16..wallet.crypto.ciphertext.len() + 16].copy_from_slice(&wallet.crypto.ciphertext);

    let mac = Keccak256::hash(&mac_buff);

    assert_eq!(&mac, &*wallet.crypto.mac);
    let mut encryptor = CtrMode::new(
        AesSafe128Encryptor::new(&left_bits),
        wallet.crypto.cipherparams.iv.into(),
    );
    let mut decrypted = [0u8; 32];
    encryptor
        .decrypt(
            &mut RefReadBuffer::new(&wallet.crypto.ciphertext),
            &mut RefWriteBuffer::new(&mut decrypted),
            true,
        )
        .map_err(|err| Error::message("decryption failed"))?;
    Ok(decrypted.into())
}

fn create_signer(keys: Keys) -> Result<Signer, Error> {
    Signer::new(keys.secret).map_err(|err| err.into())
}


macro_rules! opt_slice {
    ($opt:ident) => {
        $opt.as_ref().map(|val| val.as_ref())
    }
}

fn build_transaction(
    signer: Signer,
    nonce: Option<U256>,
    gas_price: Option<U256>,
    gas_limit: Option<U256>,
    to: Option<Address>,
    value: Option<U256>,
    data: Option<Bytes>,
) -> Bytes {
    let calldata = RawTxBuilder {
        signer: signer,
        nonce: opt_slice!(nonce),
        gas_price: opt_slice!(gas_price),
        gas_limit: opt_slice!(gas_limit),
        to: opt_slice!(to),
        value: opt_slice!(value),
        data: opt_slice!(data),
    }.finish();
    calldata.into()
}

fn send_transaction(signed: Bytes) -> Result<Value, Error> {
    let rpc = json!({
        "method": "eth_sendRawTransaction",
        "params": [signed],
        "id": 0,
        "jsonrpc": "2.0"
    });
    let client = reqwest::Client::new();
    let mut res = client.post("http://127.0.0.1:8545").json(&rpc).send()?;
    let rsp_json = res.json()?;
    Ok(rsp_json)
}

#[derive(Debug, Serialize, Deserialize)]
struct EstimateTx {
    to: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Bytes>,
}

fn estimate_gas_cost(tx: EstimateTx) -> Result<U256, Error> {
    let rpc = json!({
        "method": "eth_estimateGas",
        "params": [tx],
        "id": 0,
        "jsonrpc": "2.0"
    });
    let client = reqwest::Client::new();
    let mut res = client.post("http://127.0.0.1:8545").json(&rpc).send()?;
    let rsp_json: Value = res.json()?;
    if let Some(Value::String(price_string)) = rsp_json.get("result") {
        let parsed = price_string.parse()?;
        Ok(parsed)
    } else {
        let msg = "expected jsonrpc result of type `String` for gas price";
        Err(Error::message(msg))
    }
}


fn get_gas_price() -> Result<U256, Error> {
    let rpc = json!({
        "method": "eth_gasPrice",
        "id": 0,
        "jsonrpc": "2.0"
    });
    let client = reqwest::Client::new();
    let mut res = client.post("http://127.0.0.1:8545").json(&rpc).send()?;
    let rsp_json: Value = res.json()?;
    if let Some(Value::String(price_string)) = rsp_json.get("result") {
        let parsed = price_string.parse()?;
        Ok(parsed)
    } else {
        let msg = "expected jsonrpc result of type `String` for gas price";
        Err(Error::message(msg))
    }
}

// -------------------------------
fn testimate() -> Result<U256, Error> {
    let test_tx = EstimateTx {
        to: "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
            .parse()
            .unwrap(),
        from: Some(
            "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
                .parse()
                .unwrap(),
        ),
        value: Some(0xdeadbeefu32.into()),
        data: Some("0xdeadbeef".parse().unwrap()),
    };
    estimate_gas_cost(test_tx)
}

fn test_tx() {
    let nonce = 1u32.into();
    let value = 0xdeadbeefu32.into();
    let gas_price = 0x01u32.into();
    let gas_limit = 0x5555u32.into();
    let signer = dev_signer();
    let to = "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        .parse()
        .unwrap();
    let calldata = build_transaction(
        signer,
        Some(nonce),
        Some(gas_price),
        Some(gas_limit),
        Some(to),
        Some(value),
        None,
    );
    send_transaction(calldata);
    // println!("calldata = {:?}", calldata)
}
// -------------------------------> Error

use std::{fmt, error};

#[derive(Debug)]
pub enum Error {
    Error(Box<error::Error>),
    Message(&'static str),
}

impl Error {
    pub fn message(msg: &'static str) -> Self {
        Error::Message(msg)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Error(err) => err.fmt(f),
            Error::Message(msg) => f.write_str(msg),
        }
    }
}

impl<T> From<T> for Error
where
    T: error::Error + 'static,
{
    fn from(err: T) -> Self {
        Error::Error(Box::new(err))
    }
}

// ---------------------------------------->  main
const USAGE: &'static str = r#"
Mimir-Crypto cli
           ____
          /\   \
         /  \___\
        _\  / __/_
       /\ \/_/\   \
      /  \__/  \___\
     _\  /  \  / __/_
    /\ \/___/\/_/\   \
   /  \___\    /  \___\
  _\  /   /_  _\__/ __/_
 /\ \/___/  \/\   \/\   \
/  \___\ \___\ \___\ \___\
\  /   / /   / /   / /   /
 \/___/\/___/\/___/\/___/

Usage:
crypto-cli keygen
crypto-cli test
crypto-cli decrypt <file> <password>
"#;

#[derive(Debug, Deserialize)]
struct Args {
    arg_file: String,
    arg_password: String,
    cmd_keygen: bool,
    cmd_test: bool,
    cmd_decrypt: bool,
}

fn main() {
    println!("{}", USAGE);
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_keygen == true {
        let keys = keygen();
    // store_keys(keygen()).unwrap()
    } else if args.cmd_test {
        let wallet = retrieve_keys_json("keys.json").unwrap();
        let decrypted = decrypt_wallet(wallet, "").unwrap();
        println!("{:?}", decrypted);
    } else if args.cmd_decrypt == true {
        println!("You're trying to decrypt");
    } else {
        println!("Something is wrong with keygen");
    }
}
//-------------------------------------------
