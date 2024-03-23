/*use tokio::time;
use tonic::transport::Channel;

use abi::message::msg::Data;
use abi::message::Msg;
use abi::{
    config::Config,
    message::{msg_service_client::MsgServiceClient, SendMsgRequest, Single},
};
use ws::ws_server::WsServer;

#[tokio::test]
async fn send_msg_should_work() {
    let config = Config::load("../abi/fixtures/im.yml").unwrap();
    let mut client = get_client(&config).await;
    client
        .send_message(SendMsgRequest {
            message: Some(Msg {
                send_id: "".to_string(),
                receiver_id: "".to_string(),
                local_id: "".to_string(),
                server_id: "".to_string(),
                data: Some(Data::Single(Single {
                    msg_id: "123".to_string(),
                    content: "hello world".to_string(),
                    content_type: 1,
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
    if let Err(err) = MsgServiceClient::connect(url.clone()).await {
        println!("err: {:?}", err);
    }
    // try to connect to server
    let future = async move {
        while MsgServiceClient::connect(url.clone()).await.is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
        // return result
        MsgServiceClient::connect(url).await.unwrap()
    };
    // set timeout
    time::timeout(time::Duration::from_secs(5), future)
        .await
        .unwrap_or_else(|e| panic!("connect timeout{:?}", e))
}
*/
