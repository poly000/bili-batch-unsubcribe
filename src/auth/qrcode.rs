use crate::Client;
use serde::Deserialize;
use serde_json::value::Value;

use crate::Result;

#[derive(Deserialize)]
struct Response {
    code: i32,
    message: String,
    ttl: u8,
    data: QrGenResponse,
}

#[derive(Deserialize)]
pub struct QrGenResponse {
    pub url: String,
    pub qrcode_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrScanStatus {
    Success { timestamp: u64, csrf: String },
    Expired,
    Unconfirmed,
    Unscanned,
}

/// returns qrcode_key
pub async fn generate_qrcode_key(client: &Client) -> Result<QrGenResponse> {
    let resp = client
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .send()
        .await?;
    let qr_resp = resp.json::<Response>().await?;
    Ok(qr_resp.data)
}

/// returns login timestamp on success
///
/// cookies will be set to client automatically
pub async fn verify_qrcode_key(client: &Client, qrcode_key: &str) -> Result<QrScanStatus> {
    let resp = client.get(format!("https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={qrcode_key}")).send().await?;
    let csrf: Option<String> = resp
        .cookies()
        .find(|c| c.name() == "bili_jct")
        .map(|c| c.value().into());
    let scan_result = resp.json::<Value>().await?;

    let code = scan_result
        .pointer("/data/code")
        .expect("no code in qrcode_key poll response");

    match code.as_u64().expect("error parsing code as a integer") {
        0 => Ok(QrScanStatus::Success {
            timestamp: scan_result
                .pointer("/data/timestamp")
                .expect("no timestamp in response with code 0")
                .as_u64()
                .unwrap(),
            csrf: csrf.unwrap(),
        }),
        86038 => Ok(QrScanStatus::Expired),
        86090 => Ok(QrScanStatus::Unconfirmed),
        86101 => Ok(QrScanStatus::Unscanned),
        _ => panic!("unknown error code on qr scan result"),
    }
}
