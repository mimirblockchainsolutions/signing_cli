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
mod transaction;
mod store;
mod error;
mod key;

use docopt::Docopt;
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
        let _keys = key::keygen();
    // store_keys(keygen()).unwrap()
    } else if args.cmd_test {
        println!{"Test!"};
    } else if args.cmd_decrypt == true {
        let wallet = store::retrieve_keys_json(&args.arg_file).unwrap();
        let decrypted = key::decrypt_wallet(wallet, args.arg_password).unwrap();
        println!("{:?}", decrypted);
    } else {
        println!("Something is wrong with keygen");
    }
}
//-------------------------------------------
