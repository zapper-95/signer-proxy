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
### Set Up
To use the signer-proxy with AWS KMS, you must have an asymmetric key configured for signing transactions. If you don’t have one, follow this [guide](https://aws.amazon.com/blogs/web3/import-ethereum-private-keys-to-aws-kms/).

You’ll also need the AWS CLI installed on the device where you plan to run the proxy. Installation instructions can be found [here](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html).

### Configuration
Wherever you run the proxy, it must be configured with your KMS key. 

```bash
aws configure
```

Enter the Access Key ID, Secret Access Key and Default region name for an IAM account that has usage permissions for your key. Put the output format as json.

Alternatively, set your environment variables:

```bash
export AWS_ACCESS_KEY_ID=
export AWS_SECRET_ACCESS_KEY=
export AWS_REGION=
```

### serve

Starts an AWS KMS-based proxy server that listens for `eth_signTransaction` requests. By default, it listens on `0.0.0.0:4000`

```bash
signer-proxy aws-kms serve
```

### API


| Method | Endpoint | Description | Parameters | Body |
| ---  | --- | --- | --- | --- | 
| `GET` |    `/ping` | Tests that the server is running | - | - |
| `POST` | `/key/{key_id}` | Signs a proposed transaction and returns it RLP encoded | `key_id` (string) - key identifier of your Amazon KMS key | `{"id": 1,"jsonrpc": "2.0","method": "eth_signTransaction","params": [{"chainId": "{chain_id}","data": "0x","from": "{from_address}","gas": "{gas}","gasPrice": "{gas_price}","nonce": "{nonce}","to": "{to_address}","value": "{value_to_send}"}]}`
| `GET` | `/key/{key_id}/address` | Returns the wallet address of your KMS key | `key_id` (string) - key identifier of your Amazon KMS key | - | 


### Example Requests

#### /key/{key_id}
_Request:_
```bash
curl -X POST -H "Content-Type: application/json" -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "eth_signTransaction",
    "params": [
        {
            "chainId": 17000,
            "data": "0x",
            "from": "0xD0e9d614E8d5C5C3e7F09Dcb31CB3A7552deC836",
            "gas": "0x7b0c",
            "gasPrice": "0x1250b1",
            "nonce": "0x0",
            "to": "0x75dA2Ff67BE16c30195067b3CD40702E1F6D4EAE",
            "value": "0x2386f26fc10000"
        }
    ]
}' http://localhost:4000/key/65021b59-0433-47e7-975d-0dcbfe898f9e
```

_Response:_
```bash
{"id":1,"jsonrpc":"2.0","result":"0xf86b80831250b1827b0c9475da2ff67be16c30195067b3cd40702e1f6d4eae872386f26fc10000808284f4a009c813f8739ef99dae0c28109ffa1c167c62ec3f0b4f9027106969e4f1aaf966a02d696fee7ae9547cfa027193ba2ab514c2c71ee0bcd262034355b16b2b319c78"}
```

#### /key/{key_id}/address
_Request:_
```bash
curl -X GET http://localhost:4000/key/65021b59-0433-47e7-975d-0dcbfe898f9e/address
```

_Response:_
```bash
{"address":"0xD0e9d614E8d5C5C3e7F09Dcb31CB3A7552deC836"}
```

## Authentication and Firewall  

`signer-proxy` does not include built-in basic authentication. For enhanced security, we recommend securing `signer-proxy` behind a firewall or using a reverse proxy, such as [NGINX](https://nginx.org) or [Traefik](https://traefik.io). This setup allows you to implement basic authentication and optionally add a TLS certificate for an extra layer of protection.  

## Using `signer-proxy` with the OP Stack  

To secure the private keys used by [OP Stack Privileged Roles](https://docs.optimism.io/chain/security/privileged-roles) with `signer-proxy`, you **must remove all private keys from environment variables and arguments** passed to any OP Stack services (e.g., `op-batcher`, `op-proposer`, `op-challenger`, `op-node` etc.). Instead, configure the signer address and endpoint as environment variables or arguments as shown below:  

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

# op-node
OP_NODE_SIGNER_ADDRESS=0x... 
OP_NODE_SIGNER_ENDPOINT=http://127.0.0.1:4000/key/...

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
