use alloy::{
    eips::eip2718::Encodable2718,
    hex,
    network::{EthereumWallet, TransactionBuilder},
    rpc::types::TransactionRequest,
    primitives::{U256, B256, keccak256, Address},
};

use anyhow::{anyhow, Result as AnyhowResult};
use serde_json::Value;

use crate::{
    jsonrpc::{JsonRpcReply, JsonRpcRequest, JsonRpcResult},
};

use serde::{Deserialize};
use serde_with::{serde_as};
use serde_with::base64::{Base64};

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockPayloadArgs {
    pub domain: [u8; 32],
    pub chain_id: U256,         // U256 is 32 bytes, matches big.Int

    #[serde_as(as = "Base64")]
    pub payload_hash: Vec<u8>,
    
    pub sender_address: Address,
    }


pub async fn handle_eth_sign_transaction(
    payload: JsonRpcRequest<Vec<Value>>,
    signer: EthereumWallet,
) -> AnyhowResult<JsonRpcReply<Value>> {
    let params = payload.params.ok_or_else(|| anyhow!("params is empty"))?;

    if params.is_empty() {
        return Err(anyhow!("params is empty"));
    }

    let tx_object = params[0].clone();
    let tx_request = serde_json::from_value::<TransactionRequest>(tx_object)?;
    let tx_envelope = tx_request.build(&signer).await?;
    println!("tx_envelope: {:?}", tx_envelope);

    let encoded_tx = tx_envelope.encoded_2718();
    let rlp_hex = hex::encode_prefixed(encoded_tx);
    println!("rlp_hex: {:?}", rlp_hex);

    Ok(JsonRpcReply {
        id: payload.id,
        jsonrpc: payload.jsonrpc,
        result: JsonRpcResult::Result(rlp_hex.into()),
    })
}

pub async fn handle_health_status(
    payload: JsonRpcRequest<Vec<Value>>,
) -> AnyhowResult<JsonRpcReply<Value>> {
    Ok(JsonRpcReply {
        id: payload.id,
        jsonrpc: payload.jsonrpc,
        result: JsonRpcResult::Result(env!("CARGO_PKG_VERSION").into()),
    })
}



pub fn to_signing_hash(args: &BlockPayloadArgs) -> B256 {
    let mut msg_input = [0u8; 96];
    msg_input[0..32].copy_from_slice(&args.domain);

    // Convert U256 → B256 (i.e. big-endian bytes) then grab the inner [u8;32]
    let chain_id_bytes: [u8; 32] = B256::from(args.chain_id).0; 
    msg_input[32..64].copy_from_slice(&chain_id_bytes);  //  [oai_citation_attribution:0‡Docs.rs](https://docs.rs/alloy-primitives/latest/alloy_primitives/aliases/type.B256.html)

    let payload_hash_bytes: [u8; 32] = args
    .payload_hash
    .as_slice()                    // &[u8]
    .try_into()                   // &[u8] → [u8;32], copied
    .unwrap();

    println!("payload_hash_bytes: {:?}", payload_hash_bytes);
    msg_input[64..96].copy_from_slice(&payload_hash_bytes);

    keccak256(msg_input)
}
