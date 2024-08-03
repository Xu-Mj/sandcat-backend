use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use abi::errors::Error;
use abi::message::{DelMsgRequest, GetDbMessagesRequest, Msg};

use crate::api_utils::custom_extract::{JsonWithAuthExtractor, PathWithAuthExtractor};
use crate::AppState;

// message handler, offer the ability to pull offline message
// #[allow(dead_code)]
// pub async fn pull_offline_msg_stream(
//     State(state): State<AppState>,
//     JsonWithAuthExtractor(req): JsonWithAuthExtractor<GetDbMsgRequest>,
// ) -> Result<Response<dyn Body>, Error> {
//     let mut db_rpc = state.db_rpc.clone();
//     // validate
//     req.validate()?;
//
//     // request db rpc
//     let response = db_rpc.get_msg_stream(req).await.map_err(|e| {
//         Error::InternalServer(format!(
//             "procedure db rpc service error: get_messages {:?}",
//             e
//         ))
//     })?;
//     let stream = response.into_inner();
//
//     let stream = stream.map(|msg_result| {
//         msg_result
//             .map_err(|e| Error::InternalServer(e.to_string()))
//             .and_then(|msg| {
//                 bincode::serialize(&msg)
//                     .map(hyper::body::Bytes::from)
//                     .map_err(|e| Error::InternalServer(e.to_string()))
//             })
//     });
//     let body = Body::wrap_stream(stream); // 转换流
//
//     // 创建axum的HTTP响应并绑定body
//     Ok(Response::new(body))
// }

pub async fn pull_offline_messages(
    State(state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<GetDbMessagesRequest>,
) -> Result<Json<Vec<Msg>>, Error> {
    let result = state
        .msg_box
        .get_msgs(
            &req.user_id,
            req.send_start,
            req.send_end,
            req.start,
            req.end,
        )
        .await?;
    Ok(Json(result))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Seq {
    pub seq: i64,
    pub send_seq: i64,
}

pub async fn get_seq(
    State(state): State<AppState>,
    PathWithAuthExtractor(user_id): PathWithAuthExtractor<String>,
) -> Result<Json<Seq>, Error> {
    let seq = state.cache.get_cur_seq(&user_id).await?;
    Ok(Json(Seq {
        seq: seq.0,
        send_seq: seq.1,
    }))
}

pub async fn del_msg(
    State(state): State<AppState>,
    JsonWithAuthExtractor(req): JsonWithAuthExtractor<DelMsgRequest>,
) -> Result<(), Error> {
    state
        .msg_box
        .delete_messages(&req.user_id, req.msg_id)
        .await?;
    Ok(())
}
