use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use deadpool_diesel::postgres::Pool;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use redis::Client;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::domain::model;
use crate::domain::model::friend_request_status::FriendStatus;
use crate::domain::model::msg::Msg;
use crate::infra::errors::InfraError;
use crate::infra::repositories::friendship_repo::{
    get_agree_by_user_id, get_by_user_id_and_status,
};
use crate::infra::repositories::messages::get_offline_msg;
use crate::infra::repositories::user_repo::update_online;
use crate::utils::redis::redis_crud;
use crate::utils::PathExtractor;
use crate::AppState;

// pub struct WebSocketObject{
//
// }
pub async fn register_ws(redis: Client, user_id: String) -> Result<String, InfraError> {
    let redis_conn = redis
        .get_async_connection()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    // 向redis注册
    // 记录用户id以及浏览器指纹
    // 生成uuid，并将这个id返回，用户链接ws时需要提供这个uuid
    // let uuid = nanoid!();
    // 向注册中心获取ws服务地址
    let addr = get_ws_addr().await?;
    // let value = format!("{}/{}", addr, uuid);
    redis_crud::set_string(redis_conn, user_id, addr.clone())
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;

    Ok(addr)
}

pub async fn get_ws_addr() -> Result<String, InfraError> {
    Ok(String::from("ws://172.24.48.1:3000/ws"))
}

pub const HEART_BEAT_INTERVAL: u64 = 10;

pub async fn websocket_handler(
    PathExtractor((user_id, token, pointer_id)): PathExtractor<(String, String, String)>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 验证token
    tracing::debug!("token is {}", token);
    ws.on_upgrade(move |socket| websocket(user_id, pointer_id, socket, state))
}

async fn sync_offline_msg(
    pool: &Pool,
    pg_pool: &PgPool,
    user_id: String,
    shared_tx: Arc<RwLock<SplitSink<WebSocket, Message>>>,
) {
    // 因为这里取得了锁，因此其他消息会阻塞，只有将离线消息都发送给客户端后才能正常进行在线消息的发送
    let mut guard = shared_tx.write().await;
    // 查询离线消息
    match get_offline_msg(pool, user_id.clone()).await {
        Ok(list) => {
            // guard.send(Message::Binary())
            // 先循环实现，然后使用trait的方式修改消息
            for msg in list {
                let msg = Msg::single_from_db(msg);
                if let Err(err) = guard
                    .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                    .await
                {
                    tracing::error!("发送离线消息错误: {:?}", err);
                }
            }
        }
        Err(err) => {
            tracing::error!("查询离线消息错误: {:?}", err);
        }
    }

    // 查询好友请求，
    match get_by_user_id_and_status(pool, user_id.clone(), FriendStatus::Pending).await {
        Ok(list) => {
            for msg in list {
                let msg = Msg::RecRelationship(msg);
                if let Err(err) = guard
                    .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                    .await
                {
                    tracing::error!("发送离线消息错误: {:?}", err);
                }
            }
        }
        Err(err) => {
            tracing::error!("查询好友请求列表错误: {:?}", err);
        }
    }

    // 查询请求回复
    match get_agree_by_user_id(pg_pool, user_id.clone()).await {
        Ok(list) => {
            for msg in list {
                let msg = Msg::RelationshipRes(msg);
                if let Err(err) = guard
                    .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                    .await
                {
                    tracing::error!("发送离线消息错误: {:?}", err);
                }
            }
        }
        Err(err) => {
            tracing::error!("查询好友请求列表错误: {:?}", err);
        }
    }
}

/// user_id
/// pointer_id browser fingerprint
async fn websocket(user_id: String, pointer_id: String, ws: WebSocket, state: AppState) {
    // 注册客户端
    tracing::debug!(
        "client {} connected, user id : {}",
        user_id.clone(),
        pointer_id.clone()
    );
    let pool = state.pool;
    let hub = state.hub;
    let (ws_tx, mut ws_rx) = ws.split();
    let shared_tx = Arc::new(RwLock::new(ws_tx));
    let client = model::Client {
        id: pointer_id.clone(),
        sender: shared_tx.clone(),
    };
    hub.register(user_id.clone(), client).await;
    // mark user online
    if let Err(err) = update_online(&state.pg_pool, &user_id, true).await {
        tracing::error!("更新用户在线状态错误: {}", err);
    }
    // 注册完成后，查询离线消息，以同步的方式发送给客户端
    sync_offline_msg(&pool, &state.pg_pool, user_id.clone(), shared_tx.clone()).await;
    // drop(guard);
    tracing::debug!("离线消息发送完成");
    // 开启任务发送心跳,这里直接回发即可
    let cloned_tx = shared_tx.clone();
    let mut ping_task = tokio::spawn(async move {
        loop {
            if let Err(e) = cloned_tx
                .write()
                .await
                .send(Message::Ping(Vec::new()))
                .await
            {
                // 这里其实是需要处理的，后面再看看有没有别的方式吧？
                tracing::error!("心跳发送失败：{:?}", e);
                break;
            } else {
                // tracing::debug!("心跳发送成功");
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(HEART_BEAT_INTERVAL)).await;
        }
    });
    let cloned_hub = hub.clone();
    let shared_tx = shared_tx.clone();
    let cloned_user_id = user_id.clone();
    // 读取收到的消息
    let mut rec_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            // 处理消息
            match msg {
                Message::Text(text) => {
                    let result = serde_json::from_str(&text);
                    if result.is_err() {
                        tracing::error!("反序列化错误: {:?}", result.err());
                        continue;
                    }
                    let msg: Msg = result.unwrap();

                    if cloned_hub.broadcast(msg).await.is_err() {
                        // 如果广播出错，那么服务端服务不可用
                        break;
                    }
                }
                Message::Ping(_) => {
                    if let Err(e) = shared_tx
                        .write()
                        .await
                        .send(Message::Pong(Vec::new()))
                        .await
                    {
                        tracing::error!("心跳回复失败 : {:?}", e);
                        break;
                    }
                }
                Message::Pong(_) => {
                    // tracing::debug!("收到心跳回复消息");
                }
                Message::Close(info) => {
                    if let Some(info) = info {
                        tracing::warn!("client closed {}", info.reason);
                    }
                    // mark client offline
                    if let Err(e) = update_online(&state.pg_pool, &cloned_user_id, false).await {
                        tracing::error!("更新用户在线状态失败: {}", e);
                    }
                    break;
                }

                Message::Binary(_) => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut ping_task) => rec_task.abort(),
        _ = (&mut rec_task) => ping_task.abort(),
    }
    hub.unregister(user_id, pointer_id).await;
    tracing::debug!("client thread exit {}", hub.hub.read().await.len());
}
