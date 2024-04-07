use axum::extract::State;
use axum::http::Response;
use axum::Json;
use futures::StreamExt;
use hyper::Body;

use abi::errors::Error;
use abi::message::{GetDbMsgRequest, Msg};
use utils::custom_extract::JsonWithAuthExtractor;

use crate::AppState;

// message handler, offer the ability to pull offline message
#[allow(dead_code)]
pub async fn pull_offline_msg_stream(
    State(state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<GetDbMsgRequest>,
) -> Result<Response<Body>, Error> {
    let mut db_rpc = state.db_rpc.clone();
    // validate
    req.validate()?;

    // request db rpc
    let response = db_rpc.get_msg_stream(req).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure db rpc service error: get_messages {:?}",
            e
        ))
    })?;
    let stream = response.into_inner();

    let stream = stream.map(|msg_result| {
        msg_result
            .map_err(|e| Error::InternalServer(e.to_string()))
            .and_then(|msg| {
                bincode::serialize(&msg)
                    .map(hyper::body::Bytes::from)
                    .map_err(|e| Error::InternalServer(e.to_string()))
            })
    });
    let body = Body::wrap_stream(stream); // 转换流

    // 创建axum的HTTP响应并绑定body
    Ok(Response::new(body))
}

pub async fn pull_offline_messages(
    State(state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<GetDbMsgRequest>,
) -> Result<Json<Vec<Msg>>, Error> {
    let mut db_rpc = state.db_rpc.clone();
    // validate
    req.validate()?;

    // request db rpc
    let response = db_rpc.get_messages(req).await.map_err(|e| {
        Error::InternalServer(format!(
            "procedure db rpc service error: get_messages {:?}",
            e
        ))
    })?;
    Ok(Json(response.into_inner().messages))
}
