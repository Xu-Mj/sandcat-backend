use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use crate::model::manager::Manager;
use crate::model::msg::Msg;

pub const HEART_BEAT_INTERVAL: u64 = 10;


pub async fn websocket_handler(
    Path(id): Path<i32>,
    ws: WebSocketUpgrade,
    State(state): State<Manager>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket(id, socket, state))
}

async fn websocket(id: i32, ws: WebSocket, state: Manager) {
    let (ws_tx, mut ws_rx) = ws.split();
    state.register(id, ws_tx).await;
    // 注册客户端
    tracing::debug!("client {} connected", id.clone());
    let cloned_hub = state.clone();
    // 开启任务发送心跳
    let mut ping_task = tokio::spawn(async move {
        loop {
            if let Some(client) = cloned_hub.hub.write().await.get_mut(&id) {
                if let Err(e) = client.send(Message::Ping(Vec::new())).await {
                    // 这里其实是需要处理的，后面再看看有没有别的方式吧？
                    tracing::error!("心跳发送失败：{:?}", e);
                    break;
                } else {
                    tracing::debug!("心跳发送成功");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(HEART_BEAT_INTERVAL)).await;
        }
    });
    let cloned_hub = state.clone();
    // 读取收到的消息
    let mut rec_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            // 处理消息
            match msg {
                Message::Text(text) => {
                    tracing::info!("received message: {}", text.clone());
                    // 处理一下自己给自己发消息，只要做消息漫游就好了

                    let result = serde_json::from_str(&text);
                    if result.is_err() {
                        tracing::error!("反序列化错误: {:?}", result.err());
                        continue;
                    }
                    // 数据入库
                    let msg: Msg = result.unwrap();
                    if msg.send_id() == msg.friend_id() {
                        continue;
                    }
                    if let Err(_) = cloned_hub.broadcast(msg).await {
                        // 如果广播出错，那么服务端服务不可用
                        break;
                    }
                }
                Message::Ping(_) => {
                    tracing::debug!("收到心跳检测请求");
                    if let Some(client) = cloned_hub.hub.write().await.get_mut(&id) {
                        if let Err(e) = client.send(Message::Pong(Vec::new())).await {
                            tracing::error!("心跳回复失败 : {:?}", e);
                            break;
                        }
                    }
                }
                Message::Pong(_) => {
                    tracing::debug!("收到心跳回复消息");
                }
                Message::Close(info) => {
                    tracing::warn!("client closed {}", info.unwrap().reason);
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
    state.unregister(id).await;
    tracing::debug!("client thread exit {}", state.hub.read().await.len());
}
