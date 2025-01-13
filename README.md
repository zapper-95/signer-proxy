# signer-proxy

An RPC signer proxy server that listens for the `eth_signTransaction` requests and performs transaction signing using the YubiHSM2 hardware or AWS KMS signer.

## Install

```bash
cargo install --path . --no-default-features
```

```bash
signer-proxy -h
```

Currently, the signer-proxy supports two signers: YubiHSM2 and AWS KMS.

```bash
signer-proxy yubihsm -h
signer-proxy aws-kms -h
```

## YubiHSM2

### Global options for `generate-key` and `serve` subcommands

> [!NOTE]  
> You can connect to YubiHSM2 using two methods: usb or http via `-m, --mode` option.

````bash
-a, --auth-key <auth-key-id>              YubiHSM auth key ID [env: YUBIHSM_AUTH_KEY_ID=]
-d, --device-serial <device-serial-id>    YubiHSM device serial ID (for USB mode) [env: YUBIHSM_DEVICE_SERIAL_ID=]
    --addr <http-address>                 YubiHSM HTTP address (for HTTP mode) [env: YUBIHSM_HTTP_ADDRESS=]
    --port <http-port>                    YubiHSM HTTP port (for HTTP mode) [env: YUBIHSM_HTTP_PORT=]
-m, --mode <mode>                         Connection mode (usb or http) [env: YUBIHSM_MODE=] [default: usb] [possible values: usb, http]
-p, --pass <password>                     YubiHSM auth key password [env: YUBIHSM_PASSWORD]
````

### generate-key

Generates a valid secp256k1 key for signing eth transactions with capability `SIGN_ECDSA` and `EXPORTABLE_UNDER_WRAP` (if flag `-e, --exportable`). See docs about Capability [here](https://docs.yubico.com/hardware/yubihsm-2/hsm-2-user-guide/hsm2-core-concepts.html#capability).

```bash
signer-proxy yubihsm -d <device-serial-id> -a <auth-key-id> -p <password> generate-key -l <label> -e
```

#### Options/flags for `generate-key` subcommand

```bash
signer-proxy yubihsm generate-key -h
```

```bash
-e, --exportable       The key will be exportable or not
-l, --label <label>    Key label [default: ]
```

### serve

Starts a YubiHSM-based proxy server that listens for `eth_signTransaction` requests.

```bash
signer-proxy yubihsm -d <device-serial-id> -a <auth-key-id> -p <password> serve
```

No additional options and flags for `serve` subcommand.

## AWS KMS

### serve

Starts an AWS KMS-based proxy server that listens for `eth_signTransaction` requests.

```bash
signer-proxy aws-kms serve
```

Configuration is managed through shared `.aws/config` and `.aws/credentials` files or environment variables:

```bash
export AWS_ACCESS_KEY_ID=
export AWS_SECRET_ACCESS_KEY=
export AWS_REGION=
```

## Tests

Start [anvil](https://github.com/foundry-rs/foundry/tree/master/crates/anvil) and the proxy server, and then:

```bash
cd test
node .
```
