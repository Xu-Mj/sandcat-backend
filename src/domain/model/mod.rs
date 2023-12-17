use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) mod friends;
pub(crate) mod manager;
pub(crate) mod msg;
pub(crate) mod user;

type ClientSender = SplitSink<WebSocket, Message>;
type Hub = Arc<RwLock<HashMap<i32, ClientSender>>>;
