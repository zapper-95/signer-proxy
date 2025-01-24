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

## Authentication and Firewall  

`signer-proxy` does not include built-in basic authentication. For enhanced security, we recommend securing `signer-proxy` behind a firewall or using a reverse proxy, such as [NGINX](https://nginx.org) or [Traefik](https://traefik.io). This setup allows you to implement basic authentication and optionally add a TLS certificate for an extra layer of protection.  

## Using `signer-proxy` with the OP Stack  

To secure the private keys used by [OP Stack Privileged Roles](https://docs.optimism.io/chain/security/privileged-roles) with `signer-proxy`, you **must remove all private keys from environment variables and arguments** passed to any OP Stack services, except for `op-node` (e.g., `op-batcher`, `op-proposer`, `op-challenger`, etc.). Instead, configure the signer address and endpoint as environment variables or arguments as shown below:  

### Environment Variables  

Define the signer address and endpoint for each OP Stack service:  

```  
# op-batcher  
OP_BATCHER_SIGNER_ADDRESS=0x...  
OP_BATCHER_SIGNER_ENDPOINT=http://127.0.0.1:4000/key/...  

# op-proposer  
OP_PROPOSER_SIGNER_ADDRESS=0x...  
OP_PROPOSER_SIGNER_ENDPOINT=http://127.0.0.1:4000/key/...  

# op-challenger  
OP_CHALLENGER_SIGNER_ADDRESS=0x...  
OP_CHALLENGER_SIGNER_ENDPOINT=http://127.0.0.1:4000/key/...  

# For other services, replace [SERVICE] with the service name:  
OP_[SERVICE]_SIGNER_ADDRESS=0x...  
OP_[SERVICE]_SIGNER_ENDPOINT=http://127.0.0.1:4000/key/...  
```  

### Command-Line Arguments  

Alternatively, you can pass the same command-line arguments for every service:  

```  
--signer.address=0x...  
--signer.endpoint=http://127.0.0.1:4000/key/...  
```  

### Adding an Authentication Header  

If your reverse proxy enforces authentication headers, include them in your configuration using the following options:  

**Environment Variables:**  

```  
OP_[SERVICE]_SIGNER_HEADER=Authorization=Bearer 123abc  
```  
Replace `[SERVICE]` with each service name.  

**Command-Line Arguments:**  

```  
--signer.header="Authorization=Bearer 123abc"  
```  

### Using TLS  

If `signer-proxy` is hosted with TLS for added security, and you're not using the default certificate paths (`tls/ca.crt`, `tls/tls.crt`, `tls/tls.key`), you can specify custom paths using these options:  

**Environment Variables:**  

```  
OP_[SERVICE]_SIGNER_TLS_CA=tls/ca.crt  
OP_[SERVICE]_SIGNER_TLS_CERT=tls/tls.crt  
OP_[SERVICE]_SIGNER_TLS_KEY=tls/tls.key  
```
Replace `[SERVICE]` with each service name.  

**Command-Line Arguments:**  

```  
--signer.tls.ca=tls/ca.crt  
--signer.tls.cert=tls/tls.crt  
--signer.tls.key=tls/tls.key  
```  

## Tests

Start [anvil](https://github.com/foundry-rs/foundry/tree/master/crates/anvil) and the proxy server, and then:

```bash
cd test
node .
```
