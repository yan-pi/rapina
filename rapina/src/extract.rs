use bytes::Bytes;
use http::Request;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use crate::context::RequestContext;
use crate::error::Error;
use crate::response::{BoxBody, IntoResponse};
use crate::state::AppState;

pub struct Json<T>(pub T);
pub struct Path<T>(pub T);
pub struct Query<T>(pub T);
pub struct State<T>(pub T);
pub struct Context(pub RequestContext);

pub type PathParams = HashMap<String, String>;

pub trait FromRequest: Sized {
    fn from_request(
        req: Request<Incoming>,
        params: &PathParams,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Error>> + Send;
}

pub trait FromRequestParts: Sized + Send {
    fn from_request_parts(
        parts: &http::request::Parts,
        params: &PathParams,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Error>> + Send;
}

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Path<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Query<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> State<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl Context {
    pub fn into_inner(self) -> RequestContext {
        self.0
    }

    pub fn trace_id(&self) -> &str {
        &self.0.trace_id
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.0.elapsed()
    }
}

impl<T: DeserializeOwned + Send> FromRequest for Json<T> {
    async fn from_request(
        req: Request<Incoming>,
        _params: &PathParams,
        _state: &Arc<AppState>,
    ) -> Result<Self, Error> {
        let body = req.into_body();
        let bytes = body
            .collect()
            .await
            .map_err(|_| Error::bad_request("failed to read body"))?
            .to_bytes();

        let value: T = serde_json::from_slice(&bytes)
            .map_err(|e| Error::bad_request(format!("invalid JSON: {}", e)))?;

        Ok(Json(value))
    }
}

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> http::Response<BoxBody> {
        let body = serde_json::to_vec(&self.0).unwrap_or_default();
        http::Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(http_body_util::Full::new(Bytes::from(body)))
            .unwrap()
    }
}

impl<T: Clone + Send + Sync + 'static> FromRequestParts for State<T> {
    async fn from_request_parts(
        _parts: &http::request::Parts,
        _params: &PathParams,
        state: &Arc<AppState>,
    ) -> Result<Self, Error> {
        let value = state
            .get::<T>()
            .ok_or_else(|| Error::internal("state not found"))?;
        Ok(State(value.clone()))
    }
}

impl FromRequestParts for Context {
    async fn from_request_parts(
        parts: &http::request::Parts,
        _params: &PathParams,
        _state: &Arc<AppState>,
    ) -> Result<Self, Error> {
        parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .map(Context)
            .ok_or_else(|| Error::internal("RequestContext not found"))
    }
}

impl<T: DeserializeOwned + Send> FromRequestParts for Query<T> {
    async fn from_request_parts(
        parts: &http::request::Parts,
        _params: &PathParams,
        _state: &Arc<AppState>,
    ) -> Result<Self, Error> {
        let query = parts.uri.query().unwrap_or("");
        let value: T = serde_urlencoded::from_str(query)
            .map_err(|e| Error::bad_request(format!("invalid query: {}", e)))?;
        Ok(Query(value))
    }
}

impl<T: FromStr + Send> FromRequestParts for Path<T>
where
    T::Err: std::fmt::Display,
{
    async fn from_request_parts(
        _parts: &http::request::Parts,
        params: &PathParams,
        _state: &Arc<AppState>,
    ) -> Result<Self, Error> {
        let value = params
            .values()
            .next()
            .ok_or_else(|| Error::bad_request("missing path param"))?;

        let parsed = value
            .parse::<T>()
            .map_err(|e| Error::bad_request(format!("invalid path param: {}", e)))?;

        Ok(Path(parsed))
    }
}

impl<T: FromRequestParts> FromRequest for T {
    async fn from_request(
        req: Request<Incoming>,
        params: &PathParams,
        state: &Arc<AppState>,
    ) -> Result<Self, Error> {
        let (parts, _body) = req.into_parts();
        Self::from_request_parts(&parts, params, state).await
    }
}

pub fn extract_path_params(pattern: &str, path: &str) -> Option<PathParams> {
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    if pattern_parts.len() != path_parts.len() {
        return None;
    }

    let mut params = HashMap::new();

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        if let Some(param_name) = pattern_part.strip_prefix(':') {
            params.insert(param_name.to_string(), path_part.to_string());
        } else if pattern_part != path_part {
            return None;
        }
    }

    Some(params)
}
