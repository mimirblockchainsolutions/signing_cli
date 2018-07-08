#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate structopt;
extern crate mimir_crypto;
extern crate mimir_common;
extern crate rand;
extern crate toml;
extern crate rlp;
extern crate reqwest;
extern crate ring;
extern crate crypto;
mod transact;
mod transaction;
mod store;
mod error;
mod key;

use structopt::StructOpt;
use mimir_common::types::{Bytes, U256};
use mimir_crypto::secp256k1::Address;

// ---------------------------------------->  main
const MIMIR: &'static str = r#"
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
v0.0.1
"#;

/// Mimir crypto cli
#[derive(Debug, StructOpt)]
#[structopt(name = "crypto-cli")]
enum Opt {
    ///generate a new key
    #[structopt(name = "keygen")]
    KeyGen,
    /// run a test
    #[structopt(name = "test")]
    Test,
    /// decrypt a key file
    #[structopt(name = "decrypt")]
    Decrypt {
        /// path to key file
        keyfile: String,
        /// password for key file
        password: String,
    },
    /// build a transaction and signs it
    #[structopt(name = "new")]
    Transaction {
        /// path to key file
        keyfile: String,
        /// password for key file
        password: String,
        /// address to send to
        #[structopt(short = "t", long = "to")]
        to: Option<Address>,
        /// nonce of the account
        #[structopt(short = "o", long = "nonce")]
        nonce: Option<U256>,
        /// value of the Transaction
        #[structopt(short = "v", long = "value")]
        value: Option<U256>,
        /// data to send with the Transaction
        #[structopt(short = "c", long = "calldata")]
        calldata: Option<Bytes>,
        /// price of gas
        #[structopt(short = "p", long = "gasprice")]
        gasprice: Option<U256>,
        /// gas limit
        #[structopt(short = "l", long = "gaslimit")]
        gaslimit: Option<U256>,
    },
}

fn main() {
    println!("{}", MIMIR);
    let opt = Opt::from_args();
    println!("options: {:?}", opt);
    match opt {
        Opt::KeyGen => {
            let keys = key::keygen();
            store::store_keys(keys).unwrap();
        }
        Opt::Test => {
            // do some test stuff
            println!{"Test!"};
        }
        Opt::Decrypt { keyfile, password } => {
            let wallet = store::retrieve_keys_json(&keyfile).unwrap();
            let decrypted = key::decrypt_wallet(wallet, password).unwrap();
            println!("{:?}", decrypted);
        }
        Opt::Transaction {
            keyfile,
            password,
            to,
            nonce,
            value,
            calldata,
            gasprice,
            gaslimit,
        } => {
            let signer;
            if keyfile.contains(".json") {
                let wallet = store::retrieve_keys_json(&keyfile).unwrap();
                let decrypted = key::decrypt_wallet(wallet, password).unwrap();
                signer = key::create_signer(decrypted).unwrap()
            } else if keyfile.contains(".toml") {
                let key = store::retrieve_keys_toml(&keyfile).unwrap();
                signer = key::create_signer(key.secret).unwrap()
            } else {
                panic!("Key file not supported");
            }
            let transaction = transaction::build_transaction(
                signer,
                nonce,
                gasprice,
                gaslimit,
                to,
                value,
                calldata,
            );
            println!("Transaction: {:?}", transaction);
            transaction::send_transaction(transaction).unwrap();
        }
    }
}
//-------------------------------------------
