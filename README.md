# nagara Network's Simple Wallet

## CLI (Command Line Interface)

### Installations

```bash
cargo install --git "https://github.com/nagara-network/simple-wallet.git" nagara-simple-wallet-cli
```

### Usage - Check Balance

```bash
nagara-simple-wallet-cli-check

USAGE:
    nagara-simple-wallet-cli check --account <ACCOUNT>

OPTIONS:
    -a, --account <ACCOUNT>    SS58 address to check
    -h, --help                 Print help information
```

### Usage - Transfer Balance

```bash
nagara-simple-wallet-cli-transfer

USAGE:
    nagara-simple-wallet-cli transfer [OPTIONS] --private-key <PRIVATE_KEY> --recipient <RECIPIENT> --amount <AMOUNT>

OPTIONS:
    -a, --amount <AMOUNT>              NGR Amount in decimal
    -h, --help                         Print help information
    -p, --private-key <PRIVATE_KEY>    Sender private key hex (starts with "0x"), can also be mnemonic. Always surround it with ""
    -r, --recipient <RECIPIENT>        Recipient's SS58 address
    -s, --schnorrkel <schnorrkel>      Sender use sr25519 instead of ed25519 [default: true]
```
