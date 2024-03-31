use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc::Receiver;
use tonic::{async_trait, Request, Response, Status};
use tracing::debug;

use abi::errors::Error;
use abi::message::db_service_server::DbService;
use abi::message::{
    GetDbMsgRequest, GroupCreateRequest, GroupCreateResponse, GroupDeleteRequest,
    GroupDeleteResponse, GroupMemberExitResponse, GroupMembersIdRequest, GroupMembersIdResponse,
    GroupUpdateRequest, GroupUpdateResponse, MsgToDb, SaveMessageRequest, SaveMessageResponse,
    UserAndGroupId,
};

use crate::rpc::DbRpcService;

/// FIXME issue: what if the cache operation is failed?
/// shall we rollback the db operate?
#[async_trait]
impl DbService for DbRpcService {
    async fn save_message(
        &self,
        request: Request<SaveMessageRequest>,
    ) -> Result<Response<SaveMessageResponse>, Status> {
        let message = request.into_inner().message;
        if message.is_none() {
            return Err(Status::invalid_argument("message is empty"));
        }
        debug!("save message: {:?}", message.unwrap());
        return Ok(Response::new(SaveMessageResponse {}));
    }

    type GetMessagesStream = Pin<Box<dyn Stream<Item = Result<MsgToDb, Status>> + Send>>;

    async fn get_messages(
        &self,
        request: Request<GetDbMsgRequest>,
    ) -> Result<Response<Self::GetMessagesStream>, Status> {
        let req = request.into_inner();
        let result = self
            .mongodb
            .get_messages(req.start, req.end, "".to_string())
            .await?;
        Ok(Response::new(Box::pin(TonicReceiverStream::new(result))))
    }

    async fn group_create(
        &self,
        request: Request<GroupCreateRequest>,
    ) -> Result<Response<GroupCreateResponse>, Status> {
        let group = request.into_inner().group;
        if group.is_none() {
            return Err(Status::invalid_argument("group is empty"));
        }

        let group = group.unwrap();
        let group_id = group.id.clone();
        // insert group information and members information to db
        let invitation = self.group.create_group_with_members(group).await?;
        let members_id = invitation.members.iter().fold(
            Vec::with_capacity(invitation.members.len()),
            |mut acc, member| {
                acc.push(member.user_id.clone());
                acc
            },
        );
        //todo save the information to cache
        // save members id
        self.cache
            .save_group_members_id(&group_id, members_id)
            .await?;
        // save invitation to cache or there is not necessary to cache the group info

        let response = GroupCreateResponse {
            invitation: Some(invitation),
        };
        Ok(Response::new(response))
    }

    async fn group_update(
        &self,
        request: Request<GroupUpdateRequest>,
    ) -> Result<Response<GroupUpdateResponse>, Status> {
        let group = request.into_inner().group;
        if group.is_none() {
            return Err(Status::invalid_argument("group information is empty"));
        }
        let group = group.unwrap();
        // update db
        let group = self.group.update_group(&group).await?;

        // todo update cache
        let response = GroupUpdateResponse { group: Some(group) };
        Ok(Response::new(response))
    }

    async fn group_delete(
        &self,
        request: Request<GroupDeleteRequest>,
    ) -> Result<Response<GroupDeleteResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id;
        let group_id = req.group_id;
        if user_id.is_empty() || group_id.is_empty() {
            return Err(Status::invalid_argument("user_id or group_id is empty"));
        }

        let _group = self.group.delete_group(&group_id, &user_id).await?;

        // query the members id before delete it
        let members_id = self.cache.query_group_members_id(&group_id).await?;

        // delete from cache, and return the members id, this is performance than db
        self.cache.del_group_members(&group_id).await?;

        let response = GroupDeleteResponse { members_id };
        Ok(Response::new(response))
    }

    async fn group_member_exit(
        &self,
        request: Request<UserAndGroupId>,
    ) -> Result<Response<GroupMemberExitResponse>, Status> {
        let req = request.into_inner();
        if req.user_id.is_empty() || req.group_id.is_empty() {
            return Err(Status::invalid_argument("user_id or group_id is empty"));
        }
        self.group.exit_group(&req.user_id, &req.group_id).await?;

        // todo delete from cache, also get the members id
        let members_id = Vec::new();
        let response = GroupMemberExitResponse { members_id };
        Ok(Response::new(response))
    }

    async fn group_members_id(
        &self,
        request: Request<GroupMembersIdRequest>,
    ) -> Result<Response<GroupMembersIdResponse>, Status> {
        let group_id = request.into_inner().group_id;
        if group_id.is_empty() {
            return Err(Status::invalid_argument("group_id is empty"));
        }

        // todo query from cache, and if not exist, query from db
        let members_id = self.group.query_group_members_id(&group_id).await?;
        let response = GroupMembersIdResponse { members_id };
        Ok(Response::new(response))
    }
}

/// implement the Stream for tokio::sync::mpsc::Receiver
pub struct TonicReceiverStream<T> {
    inner: Receiver<Result<T, Error>>,
}

impl<T> TonicReceiverStream<T> {
    pub fn new(inner: Receiver<Result<T, Error>>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for TonicReceiverStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_recv(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
