use super::Res;
use crate::error::{
    ServerFnErrorErr, ServerFnErrorErr, ServerFnErrorSerde, SERVER_FN_ERROR_HEADER,
};
use axum::body::Body;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::{header, HeaderValue, Response, StatusCode};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

impl<CustErr> Res<CustErr> for Response<Body>
where
    CustErr: Send + Sync + Debug + FromStr + Display + 'static,
{
    fn try_from_string(
        content_type: &str,
        data: String,
    ) -> Result<Self, CustErr> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| ServerFnErrorErr::Response(e.to_string()))
    }

    fn try_from_bytes(
        content_type: &str,
        data: Bytes,
    ) -> Result<Self, CustErr> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| ServerFnErrorErr::Response(e.to_string()))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, CustErr>> + Send + 'static,
    ) -> Result<Self, CustErr> {
        let body =
            Body::from_stream(data.map(|n| n.map_err(ServerFnErrorErr::from)));
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(body)
            .map_err(|e| ServerFnErrorErr::Response(e.to_string()))
    }

    fn error_response(path: &str, err: &CustErr) -> Self {
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(SERVER_FN_ERROR_HEADER, path)
            .body(err.ser().unwrap_or_else(|_| err.to_string()).into())
            .unwrap()
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}
