use anyhow::{Result as AnyhowResult};
use crate::shutdown_signal::shutdown_signal;
#[cfg(debug_assertions)]
use crate::signers::mock::{add_mock_signers, MOCK_KEYS};
use alloy::{
    signers::{
        local::{
        yubihsm::{
            asymmetric::Algorithm::EcK256, device::SerialNumber, Capability, Client, Connector,
            Credentials, Domain, HttpConfig, UsbConfig,
        },
        YubiSigner,
        },
    }, 
};
use axum::routing::get;
use axum::{
    debug_handler,
    extract::{Path, State},
    routing::post,
    Router,
};
use serde_json::Value;
use std::time::Duration;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use structopt::StructOpt;
use strum::{EnumString, VariantNames};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use crate::{
    app_types::{AppJson, AppResult},
    jsonrpc::{JsonRpcReply, JsonRpcRequest, AddressResponse},
    signers::common::{handle_eth_sign_jsonrpc},

};
use alloy::primitives::{Address};


const DEFAULT_USB_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_HTTP_TIMEOUT_MS: u64 = 5000;
const API_TIMEOUT_SECS: u64 = 30;

#[derive(EnumString, VariantNames, Debug)]
#[strum(serialize_all = "kebab_case")]
pub enum YubiMode {
    Usb,
    Http,
    #[cfg(debug_assertions)]
    Mock,
}



#[derive(StructOpt)]
pub struct YubiOpt {
    /// Connection mode (usb or http)
    #[structopt(short, long, possible_values = YubiMode::VARIANTS, case_insensitive = true, default_value = "usb", env = "YUBIHSM_MODE")]
    pub mode: YubiMode,

    /// YubiHSM device serial ID (for USB mode)
    #[structopt(
        short,
        long = "device-serial",
        env = "YUBIHSM_DEVICE_SERIAL_ID",
        required_if("mode", "usb")
    )]
    pub device_serial_id: Option<String>,

    /// YubiHSM HTTP address (for HTTP mode)
    #[structopt(
        long = "addr",
        env = "YUBIHSM_HTTP_ADDRESS",
        required_if("mode", "http")
    )]
    pub http_address: Option<String>,

    /// YubiHSM HTTP port (for HTTP mode)
    #[structopt(long = "port", env = "YUBIHSM_HTTP_PORT", required_if("mode", "http"))]
    pub http_port: Option<u16>,

    /// YubiHSM auth key ID
    #[structopt(short, long = "auth-key", env = "YUBIHSM_AUTH_KEY_ID")]
    pub auth_key_id: u16,

    /// YubiHSM auth key password
    #[structopt(short, long = "pass", env = "YUBIHSM_PASSWORD", hide_env_values = true)]
    pub password: String,

    #[structopt(subcommand)]
    pub cmd: YubiCommand,
}

#[derive(StructOpt)]
pub enum YubiCommand {
    Serve,
    GenerateKey {
        /// Key label
        #[structopt(short, long, default_value)]
        label: String,
        /// The key will be exportable or not
        #[structopt(short, long)]
        exportable: bool,
    },
}

#[derive(Clone)]
pub struct AppState {
    pub connector: Connector,
    pub credentials: Credentials,
    pub signers: Arc<Mutex<HashMap<u16, Arc<YubiSigner>>>>,
}



#[debug_handler]
async fn handle_ping() -> &'static str {
    "pong"
}

#[debug_handler]
async fn handle_request(
    Path(key_id): Path<u16>,
    State(state): State<Arc<AppState>>,
    AppJson(payload): AppJson<JsonRpcRequest<Vec<Value>>>,
) -> AppResult<JsonRpcReply<Value>> {
    println!("{:?}", payload);
    let yubi_signer = get_signer(state.clone(), key_id).await?;
    handle_eth_sign_jsonrpc(payload, yubi_signer).await
}

async fn get_signer(state: Arc<AppState>, key_id: u16) -> AnyhowResult<Arc<YubiSigner>> {
    let mut signers = state.signers.lock().await;

    if let Some(signer) = signers.get(&key_id) {
        return Ok(signer.clone());
    }

    let yubi_signer = Arc::new(
        YubiSigner::connect(
            state.connector.clone(),
            state.credentials.clone(),
            key_id,
        )?
    );    
    signers.insert(key_id, yubi_signer.clone());
    Ok(yubi_signer)
}

#[debug_handler]
async fn handle_address_request(
    Path(key_id): Path<u16>,
    State(state): State<Arc<AppState>>,
) -> AppResult<AddressResponse> {
    let yubi_signer = get_signer(state.clone(), key_id).await?;
    let address = yubi_signer.address().to_string();

    Ok(AppJson(AddressResponse { address }))
}

fn generate_new_key(
    connector: Connector,
    credentials: Credentials,
    label: String,
    exportable: bool,
) -> AnyhowResult<(u16, Address)> {
    let client = Client::open(connector.clone(), credentials.clone(), true)?;
    let capabilities = if exportable {
        Capability::SIGN_ECDSA | Capability::EXPORTABLE_UNDER_WRAP
    } else {
        Capability::SIGN_ECDSA
    };
    let id = client.generate_asymmetric_key(
        0,
        label.as_str().into(),
        Domain::all(),
        capabilities,
        EcK256,
    )?;
    let signer = YubiSigner::connect(connector, credentials, id)?;

    Ok((id, signer.address()))
}

fn create_connector(opt: &YubiOpt) -> Connector {
    match opt.mode {
        YubiMode::Usb => {
            let serial = SerialNumber::from_str(
                opt.device_serial_id
                    .as_ref()
                    .expect("USB mode requires a device serial ID"),
            )
            .unwrap();
            Connector::usb(&UsbConfig {
                serial: Some(serial),
                timeout_ms: DEFAULT_USB_TIMEOUT_MS,
            })
        }
        YubiMode::Http => {
            let addr = opt
                .http_address
                .as_ref()
                .expect("HTTP mode requires an address")
                .clone();
            let port = *opt.http_port.as_ref().expect("HTTP mode requires a port");
            Connector::http(&HttpConfig {
                addr,
                port,
                timeout_ms: DEFAULT_HTTP_TIMEOUT_MS,
            })
        }
        #[cfg(debug_assertions)]
        YubiMode::Mock => Connector::mockhsm(),
    }
}

pub async fn handle_yubihsm(opt: YubiOpt) {
    let connector = create_connector(&opt);
    #[cfg(not(debug_assertions))]
    let credentials = Credentials::from_password(opt.auth_key_id, opt.password.as_bytes());
    #[cfg(debug_assertions)]
    let credentials = Credentials::from_password(1, "password".as_bytes());

    match opt.cmd {
        YubiCommand::Serve => {
            let shared_state = Arc::new(AppState {
                connector,
                credentials,
                signers: Arc::new(Mutex::new(HashMap::new())),
            });

            #[cfg(debug_assertions)]
            add_mock_signers(
                shared_state.clone(),
                MOCK_KEYS
                    .iter()
                    .map(|&(key_id, private_key, address)| {
                        (key_id, private_key, address.to_string())
                    })
                    .collect(),
            )
            .await
            .unwrap();

            let app = Router::new()
                .route("/ping", get(handle_ping))
                .route("/key/:key_id", post(handle_request))
                .route("/key/:key_id/address", get(handle_address_request))
                .with_state(shared_state)
                .layer((
                    TraceLayer::new_for_http(),
                    TimeoutLayer::new(Duration::from_secs(API_TIMEOUT_SECS)),
                ));

            let listener = TcpListener::bind("0.0.0.0:4000").await.unwrap();
            info!("listening on {}", listener.local_addr().unwrap());
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap();
        }
        YubiCommand::GenerateKey { label, exportable } => {
            let (id, address) =
                generate_new_key(connector, credentials, label, exportable).unwrap();

            println!("Key ID: {}", id);
            println!("Address: {}", address);
        }
    }
}


