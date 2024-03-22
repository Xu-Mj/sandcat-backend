use tokio::time;
use tonic::transport::Channel;

use abi::{
    config::Config,
    msg::{
        msg_service_client::MsgServiceClient, msg_wrapper::Msg, MsgWrapper, SendMsgRequest, Single,
    },
};
use ws::ws_server::WsServer;

#[tokio::test]
async fn send_msg_should_work() {
    let config = Config::load("../abi/fixtures/im.yml").unwrap();
    let mut client = get_client(&config).await;
    client
        .send_message(SendMsgRequest {
            message: Some(MsgWrapper {
                msg: Some(Msg::Single(Single {
                    msg_id: "123".to_string(),
                    content: "hello world".to_string(),
                    content_type: 1,
                    send_id: "11".to_string(),
                    receiver_id: "22".to_string(),
                    create_time: 123,
                })),
            }),
        })
        .await
        .unwrap();
}

// setup server
fn setup_server(config: &Config) {
    let cloned_config = config.clone();
    tokio::spawn(async move {
        WsServer::start(cloned_config).await;
    });
}

// get client
async fn get_client(config: &Config) -> MsgServiceClient<Channel> {
    // start server at first
    setup_server(config);
    let url = config.server.url(false);

    println!("connect to {}", url);
    // try to connect to server
    let future = async move {
        while MsgServiceClient::connect(url.clone()).await.is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
        // return result
        MsgServiceClient::connect(url).await.unwrap()
    };
    // set timeout
    match time::timeout(time::Duration::from_secs(5), future).await {
        Ok(client) => client,
        Err(e) => {
            panic!("connect timeout{:?}", e)
        }
    }
}
