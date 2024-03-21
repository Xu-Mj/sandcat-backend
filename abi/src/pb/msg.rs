/// / use to send single message or group message;
/// / message ws is used to connect the client by websocket;
/// / and it receive message from clients, then send message to mq;
/// / so only provide the send message function for other rpc service;
#[derive(sqlx::Type)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Single {
    /// unique id
    #[prost(string, tag = "1")]
    pub msg_id: ::prost::alloc::string::String,
    /// message content
    #[prost(string, tag = "2")]
    pub content: ::prost::alloc::string::String,
    /// message type
    #[prost(string, tag = "3")]
    pub content_type: ::prost::alloc::string::String,
    /// from
    #[prost(string, tag = "4")]
    pub send_id: ::prost::alloc::string::String,
    /// to
    #[prost(string, tag = "5")]
    pub receiver_id: ::prost::alloc::string::String,
    /// timestamp
    #[prost(int64, tag = "6")]
    pub create_time: i64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendSingleMessageRequest {
    #[prost(message, optional, tag = "1")]
    pub message: ::core::option::Option<Single>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendMessageRequest {
    #[prost(message, optional, tag = "1")]
    pub message: ::core::option::Option<Single>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendSingleMessageResponse {
    #[prost(message, optional, tag = "1")]
    pub message: ::core::option::Option<Single>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendMessageResponse {
    #[prost(message, optional, tag = "1")]
    pub message: ::core::option::Option<Single>,
}
/// Generated client implementations.
pub mod messages_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::http::Uri;
    use tonic::codegen::*;
    ///
    /// message GroupMsgWrapper{
    /// oneof group_msg {
    /// Single message = 1;
    /// GroupInvitation invitation = 2;
    /// UserAndGroupID member_exit = 3;
    /// string dismiss = 4;
    /// UserAndGroupID dismiss_or_exit_received = 5;
    /// UserAndGroupID invitation_received = 6;
    /// }
    /// }
    ///
    /// message UserAndGroupID{
    /// string user_id = 1;
    /// string group_id = 2;
    /// }
    ///
    /// message GroupInvitation{
    /// GroupInfo info = 1;
    /// repeated GroupMemberInfo members = 2;
    /// }
    ///
    /// message GroupInfo {
    /// int64 id = 1;
    /// string owner = 2;
    /// string name = 3;
    /// string avatar = 4;
    /// string description = 5;
    /// string announcement = 6;
    /// int64 member_count = 7;
    /// int64 create_time = 8;
    /// int64 update_time = 9;
    /// }
    ///
    /// message GroupMemberInfo {
    /// int64 id = 1;
    /// int32 age = 2;
    /// string group_id = 3;
    /// string user_id = 4;
    /// string group_name = 5;
    /// string avatar = 6;
    /// int64 joined_at = 7;
    /// optional string region = 8;
    /// string gender = 9;
    /// }
    #[derive(Debug, Clone)]
    pub struct MessagesClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl MessagesClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> MessagesClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> MessagesClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            MessagesClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// errors.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// send message through rpc
        pub async fn send_message(
            &mut self,
            request: impl tonic::IntoRequest<super::SendMessageRequest>,
        ) -> std::result::Result<tonic::Response<super::SendMessageResponse>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/msg.Messages/SendMessage");
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("msg.Messages", "SendMessage"));
            self.inner.unary(req, path, codec).await
        }
        /// send single message
        pub async fn send_single_message(
            &mut self,
            request: impl tonic::IntoRequest<super::SendSingleMessageRequest>,
        ) -> std::result::Result<tonic::Response<super::SendSingleMessageResponse>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/msg.Messages/SendSingleMessage");
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("msg.Messages", "SendSingleMessage"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod messages_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with MessagesServer.
    #[async_trait]
    pub trait Messages: Send + Sync + 'static {
        /// send message through rpc
        async fn send_message(
            &self,
            request: tonic::Request<super::SendMessageRequest>,
        ) -> std::result::Result<tonic::Response<super::SendMessageResponse>, tonic::Status>;
        /// send single message
        async fn send_single_message(
            &self,
            request: tonic::Request<super::SendSingleMessageRequest>,
        ) -> std::result::Result<tonic::Response<super::SendSingleMessageResponse>, tonic::Status>;
    }
    ///
    /// message GroupMsgWrapper{
    /// oneof group_msg {
    /// Single message = 1;
    /// GroupInvitation invitation = 2;
    /// UserAndGroupID member_exit = 3;
    /// string dismiss = 4;
    /// UserAndGroupID dismiss_or_exit_received = 5;
    /// UserAndGroupID invitation_received = 6;
    /// }
    /// }
    ///
    /// message UserAndGroupID{
    /// string user_id = 1;
    /// string group_id = 2;
    /// }
    ///
    /// message GroupInvitation{
    /// GroupInfo info = 1;
    /// repeated GroupMemberInfo members = 2;
    /// }
    ///
    /// message GroupInfo {
    /// int64 id = 1;
    /// string owner = 2;
    /// string name = 3;
    /// string avatar = 4;
    /// string description = 5;
    /// string announcement = 6;
    /// int64 member_count = 7;
    /// int64 create_time = 8;
    /// int64 update_time = 9;
    /// }
    ///
    /// message GroupMemberInfo {
    /// int64 id = 1;
    /// int32 age = 2;
    /// string group_id = 3;
    /// string user_id = 4;
    /// string group_name = 5;
    /// string avatar = 6;
    /// int64 joined_at = 7;
    /// optional string region = 8;
    /// string gender = 9;
    /// }
    #[derive(Debug)]
    pub struct MessagesServer<T: Messages> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Messages> MessagesServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for MessagesServer<T>
    where
        T: Messages,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/msg.Messages/SendMessage" => {
                    #[allow(non_camel_case_types)]
                    struct SendMessageSvc<T: Messages>(pub Arc<T>);
                    impl<T: Messages> tonic::server::UnaryService<super::SendMessageRequest> for SendMessageSvc<T> {
                        type Response = super::SendMessageResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SendMessageRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut =
                                async move { <T as Messages>::send_message(&inner, request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SendMessageSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/msg.Messages/SendSingleMessage" => {
                    #[allow(non_camel_case_types)]
                    struct SendSingleMessageSvc<T: Messages>(pub Arc<T>);
                    impl<T: Messages> tonic::server::UnaryService<super::SendSingleMessageRequest>
                        for SendSingleMessageSvc<T>
                    {
                        type Response = super::SendSingleMessageResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SendSingleMessageRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Messages>::send_single_message(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SendSingleMessageSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: Messages> Clone for MessagesServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: Messages> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Messages> tonic::server::NamedService for MessagesServer<T> {
        const NAME: &'static str = "msg.Messages";
    }
}
