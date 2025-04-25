use alloy::{
    eips::eip2718::Encodable2718,
    hex,
    network::{EthereumWallet, TransactionBuilder},
    rpc::types::TransactionRequest,
    primitives::{U256, B256, keccak256},
};

use anyhow::{anyhow, Result as AnyhowResult};
use serde_json::Value;

use crate::{
    jsonrpc::{JsonRpcReply, JsonRpcRequest, JsonRpcResult},
};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPayloadArgs {

    #[serde(with = "hex::serde")]
    pub domain: [u8; 32],
    pub chain_id: U256,         // U256 is 32 bytes, matches big.Int

    #[serde(with = "hex::serde")]
    pub payload_hash: [u8; 32], // 32 bytes
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

    msg_input[64..96].copy_from_slice(&args.payload_hash);

    keccak256(msg_input)
}
