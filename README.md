# Offline signing utility

This tool can be used to build and sign transactions offline to be sent to a network, or with a node running in the background to send from the command line, or send to a remote node. You can generate a key to use or use a json wallet from Parity.

## Usage

`cargo run  keygen`

`cargo run  test`

`cargo run  decrypt <keyfile> <password>`

If you have a node running, the program will try and infer nonce, gasprice, and gaslimit.

`cargo run  transaction <keyfile> <password> <to> [<nonce> <value> <calldata> <gasprice> <gaslimit>]`
