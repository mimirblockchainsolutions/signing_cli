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

mod transact;

use transact::RawTxBuilder;

use mimir_crypto::secp256k1::Signer;
use mimir_crypto::secp256k1::Public;
use mimir_crypto::secp256k1::Secret;
use mimir_crypto::secp256k1::Address;
use mimir_common::types::{Bytes, U256};
use std::fs;
use serde_json::Value;
use reqwest::Client;
use docopt::Docopt;


/// get signer for parity dev chain
fn dev_signer() -> Signer {
    let dev_secret = mimir_crypto::secp256k1::dev::SECRET;
    Signer::new(dev_secret).expect("dev secret always valid")
}


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
"#;

#[derive(Debug, Deserialize)]
struct Args {
    cmd_keygen: bool,
    cmd_test: bool,
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

fn retrieve_keys() -> Result<Keys, Error> {
    let read = fs::read_to_string("keys.toml")?;
    let toml: Keys = toml::from_str(&read)?;
    Ok(toml)
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
fn main() {
    println!("{}", USAGE);
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_keygen == true {
        let keys = retrieve_keys();
        match keys {
            Ok(t) => println!("{:?}", t),
            Err(e) => println!("{:?}", e),
        }
    // store_keys(keygen()).unwrap()
    } else if args.cmd_test {
        let result = testimate();
        match result {
            Ok(price) => println!("{:?}",price),
            Err(err) => println!("{:?}", err),
        }
    } else {
        println!("Something is wrong with keygen");
    }
}
//-------------------------------------------
