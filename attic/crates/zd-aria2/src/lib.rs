//! Minimal aria2 JSON-RPC client (stub).
//!
//! ponytail: blocking calls only — GUI pings the daemon from a worker thread.
//! Upgrade path: async (tokio + reqwest) once continuous status polling lands.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("aria2 returned error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("malformed response")]
    Malformed,
}

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    id: &'a str,
    method: &'a str,
    params: Vec<Value>,
}

#[derive(Deserialize)]
struct RpcResponse<R> {
    result: Option<R>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    pub version: String,
    #[serde(rename = "enabledFeatures", default)]
    pub enabled_features: Vec<String>,
}

#[derive(Clone)]
pub struct Client {
    url: String,
    secret: Option<String>,
    http: reqwest::blocking::Client,
}

impl Client {
    pub fn new(url: impl Into<String>, secret: Option<String>) -> Self {
        Self {
            url: url.into(),
            secret,
            http: reqwest::blocking::Client::new(),
        }
    }

    /// Default local daemon endpoint (`aria2c --enable-rpc`).
    pub fn localhost(secret: Option<String>) -> Self {
        Self::new("http://127.0.0.1:6800/jsonrpc", secret)
    }

    /// Build params with the optional secret token prepended (aria2 convention).
    fn params(&self, args: impl IntoIterator<Item = Value>) -> Vec<Value> {
        let mut v: Vec<Value> = Vec::with_capacity(2);
        if let Some(s) = &self.secret {
            v.push(Value::String(format!("token:{s}")));
        }
        v.extend(args);
        v
    }

    fn call<R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        args: Vec<Value>,
    ) -> Result<R, Error> {
        let req = RpcRequest {
            jsonrpc: "2.0",
            id: "zd3",
            method,
            params: self.params(args),
        };
        let resp: RpcResponse<R> = self.http.post(&self.url).json(&req).send()?.json()?;
        if let Some(e) = resp.error {
            return Err(Error::Rpc { code: e.code, message: e.message });
        }
        resp.result.ok_or(Error::Malformed)
    }

    pub fn get_version(&self) -> Result<Version, Error> {
        self.call("aria2.getVersion", vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_token_is_prepended() {
        let c = Client::localhost(Some("s3cret".into()));
        let p = c.params(vec![Value::from(42)]);
        assert_eq!(p[0], Value::String("token:s3cret".into()));
        assert_eq!(p[1], Value::from(42));
    }

    #[test]
    fn no_secret_no_token() {
        let c = Client::localhost(None);
        assert_eq!(c.params(vec![]).len(), 0);
    }
}
