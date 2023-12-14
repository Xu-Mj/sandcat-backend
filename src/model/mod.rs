use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use tokio::sync::RwLock;

pub mod manager;
pub mod msg;

type ClientSender = SplitSink<WebSocket, Message>;
type Hub = Arc<RwLock<HashMap<i32, ClientSender>>>;
