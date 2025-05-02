use alloy::{
    eips::eip2718::Encodable2718,
    hex,
    //rpc::types::TransactionRequest,
    primitives::{U256, B256, keccak256, Address, Signature},
    network::{TxSigner, EthereumWallet, TransactionBuilder},
    signers::{Signer},
    rpc::types::TransactionRequest,

};
use std::{sync::Arc};

use anyhow::{anyhow, Result as AnyhowResult};
use serde_json::Value;

use crate::{
    app_types::{AppError, AppJson, AppResult},
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


pub async fn handle_eth_sign_jsonrpc<S>(
    payload: JsonRpcRequest<Vec<Value>>,
    signer: Arc<S>,
) -> AppResult<JsonRpcReply<Value>> 
where S: Signer + std::marker::Sync + std::marker::Send + TxSigner<Signature> + 'static

{
    let method = payload.method.as_str();

    let result = match method {
        "eth_signTransaction" => handle_eth_sign_transaction(payload, EthereumWallet::from(signer)).await,
        "health_status" => handle_health_status(payload).await,
        "opsigner_signBlockPayload" => handle_eth_sign_block(payload, signer).await,
        _ => Err(anyhow!(
            "method not supported (only eth_signTransaction, health_status and opsigner_signBlockPayload): {}",
            method
        )),
    };

    result.map(AppJson).map_err(AppError)
}



pub async fn handle_eth_sign_block<S>(
    payload: JsonRpcRequest<Vec<Value>>,
    signer: Arc<S>,
) -> AnyhowResult<JsonRpcReply<Value>> 
where S: Signer
{

    println!("handle_eth_sign_block payload: {:?}", payload);
    let params = payload.params.ok_or_else(|| anyhow!("params is empty"))?;
    if params.is_empty() {
        return Err(anyhow!("params is empty"));
    }

    let block_object = params[0].clone();
    let block: BlockPayloadArgs = serde_json::from_value(block_object)?;

    println!("block: {:?}", block);
    let signing_hash = to_signing_hash(&block);

    let signed_hash  = signer.sign_hash(&signing_hash).await?;

    // extract the 65-byte array
    let mut sig_bytes: [u8; 65] = signed_hash.as_bytes();
    if sig_bytes[64] < 27 {
        return Err(anyhow!("Invalid recovery id: expected value >= 27, got {}", sig_bytes[64]));
    }

    sig_bytes[64] = sig_bytes[64] - 27; // Adjust the recovery id to be 0 or 1

    // encode as a "0x"-prefixed hex string
    let signed_hash_hex = hex::encode_prefixed(&sig_bytes[..]);
    println!("signed_hash_hex: {:?}", signed_hash_hex);
    Ok(JsonRpcReply {
        id: payload.id,
        jsonrpc: payload.jsonrpc,
        result: JsonRpcResult::Result(Value::String(signed_hash_hex)),
    })
}


pub fn to_signing_hash(args: &BlockPayloadArgs) -> B256 {
    // hash the block to be signed
    let mut msg_input = [0u8; 96];
    msg_input[0..32].copy_from_slice(&args.domain);

    // Convert U256 → B256 (i.e. big-endian bytes) then grab the inner [u8;32]
    let chain_id_bytes: [u8; 32] = B256::from(args.chain_id).0; 
    msg_input[32..64].copy_from_slice(&chain_id_bytes); 

    let payload_hash_bytes: [u8; 32] = args
    .payload_hash
    .as_slice()                    // &[u8]
    .try_into()                   // &[u8] → [u8;32], copied
    .unwrap();

    println!("payload_hash_bytes: {:?}", payload_hash_bytes);
    msg_input[64..96].copy_from_slice(&payload_hash_bytes);

    keccak256(msg_input)
}
