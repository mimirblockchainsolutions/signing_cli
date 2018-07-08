use mimir_common::types::{Bytes, U256};
use reqwest;
use serde_json::Value;
use transact::RawTxBuilder;
use mimir_crypto::secp256k1::{Signer, Address};
use error::Error;

macro_rules! opt_slice {
    ($opt:ident) => {
        $opt.as_ref().map(|val| val.as_ref())
    }
}
// try some stuff with the optional fields like get them, guess or infer
pub fn build_transaction(
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

pub fn send_transaction(signed: Bytes) -> Result<Value, Error> {
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
