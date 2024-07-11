use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use nanoid::nanoid;
use tokio::sync::mpsc::Receiver;
use tonic::{async_trait, Request, Response, Status};
use tracing::debug;

use abi::errors::Error;
use abi::message::db_service_server::DbService;
use abi::message::{
    CreateUserRequest, CreateUserResponse, DelMsgRequest, DelMsgResp, DeleteFriendRequest,
    DeleteFriendResponse, FriendInfo, FriendListRequest, FriendListResponse, FsAgreeRequest,
    FsAgreeResponse, FsCreateRequest, FsCreateResponse, FsListRequest, FsListResponse,
    GetDbMessagesRequest, GetDbMsgRequest, GetMsgResp, GetUserByEmailRequest, GetUserRequest,
    GetUserResponse, GroupCreateRequest, GroupCreateResponse, GroupDeleteRequest,
    GroupDeleteResponse, GroupInviteNewRequest, GroupInviteNewResp, GroupMemberExitResponse,
    GroupMembersIdRequest, GroupMembersIdResponse, GroupUpdateRequest, GroupUpdateResponse, Msg,
    MsgReadReq, MsgReadResp, QueryFriendInfoRequest, QueryFriendInfoResponse, SaveGroupMsgRequest,
    SaveGroupMsgResponse, SaveMaxSeqBatchRequest, SaveMaxSeqRequest, SaveMaxSeqResponse,
    SaveMessageRequest, SaveMessageResponse, SearchUserRequest, SearchUserResponse,
    UpdateRegionRequest, UpdateRegionResponse, UpdateRemarkRequest, UpdateRemarkResponse,
    UpdateUserRequest, UpdateUserResponse, UserAndGroupId, VerifyPwdRequest, VerifyPwdResponse,
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
        let inner = request.into_inner();
        let message = inner
            .message
            .ok_or(Status::invalid_argument("message is empty"))?;
        let need_to_history = inner.need_to_history;
        debug!("save message: {:?}", message);
        self.handle_message(message, need_to_history).await?;
        return Ok(Response::new(SaveMessageResponse {}));
    }

    async fn save_group_message(
        &self,
        request: Request<SaveGroupMsgRequest>,
    ) -> Result<Response<SaveGroupMsgResponse>, Status> {
        let inner = request.into_inner();
        let message = inner
            .message
            .ok_or(Status::invalid_argument("message is empty"))?;
        let need_to_history = inner.need_to_history;
        let members_id = inner.members;
        debug!("save group message: {:?}", message);
        self.handle_group_message(message, need_to_history, members_id)
            .await?;
        return Ok(Response::new(SaveGroupMsgResponse {}));
    }

    type GetMsgStreamStream = Pin<Box<dyn Stream<Item = Result<Msg, Status>> + Send>>;

    async fn get_msg_stream(
        &self,
        request: Request<GetDbMsgRequest>,
    ) -> Result<Response<Self::GetMsgStreamStream>, Status> {
        let req = request.into_inner();
        let result = self
            .msg_rec_box
            .get_messages_stream(&req.user_id, req.start, req.end)
            .await?;
        Ok(Response::new(Box::pin(TonicReceiverStream::new(result))))
    }

    #[allow(deprecated)]
    async fn get_messages(
        &self,
        request: Request<GetDbMsgRequest>,
    ) -> Result<Response<GetMsgResp>, Status> {
        let req = request.into_inner();
        let result = self
            .msg_rec_box
            .get_messages(&req.user_id, req.start, req.end)
            .await?;
        Ok(Response::new(GetMsgResp { messages: result }))
    }

    async fn get_msgs(
        &self,
        request: Request<GetDbMessagesRequest>,
    ) -> Result<Response<GetMsgResp>, Status> {
        let req = request.into_inner();
        let result = self
            .msg_rec_box
            .get_msgs(
                &req.user_id,
                req.send_start,
                req.send_end,
                req.start,
                req.end,
            )
            .await?;
        Ok(Response::new(GetMsgResp { messages: result }))
    }

    async fn read_msg(
        &self,
        request: Request<MsgReadReq>,
    ) -> Result<Response<MsgReadResp>, Status> {
        let req = request
            .into_inner()
            .msg_read
            .ok_or(Status::invalid_argument("message is empty"))?;

        // update database
        self.msg_rec_box
            .msg_read(&req.user_id, &req.msg_seq)
            .await?;
        Ok(Response::new(MsgReadResp {}))
    }

    async fn del_messages(
        &self,
        request: Request<DelMsgRequest>,
    ) -> Result<Response<DelMsgResp>, Status> {
        let req = request.into_inner();
        self.msg_rec_box
            .delete_messages(&req.user_id, req.msg_id)
            .await?;
        Ok(Response::new(DelMsgResp {}))
    }

    async fn save_send_max_seq(
        &self,
        request: Request<SaveMaxSeqRequest>,
    ) -> Result<Response<SaveMaxSeqResponse>, Status> {
        let req = request.into_inner();
        if let Err(e) = self.db.seq.save_max_seq(&req.user_id).await {
            return Err(Status::internal(e.to_string()));
        };
        Ok(Response::new(SaveMaxSeqResponse {}))
    }

    async fn save_max_seq(
        &self,
        request: Request<SaveMaxSeqRequest>,
    ) -> Result<Response<SaveMaxSeqResponse>, Status> {
        let req = request.into_inner();
        if let Err(e) = self.db.seq.save_max_seq(&req.user_id).await {
            return Err(Status::internal(e.to_string()));
        };
        Ok(Response::new(SaveMaxSeqResponse {}))
    }

    async fn save_max_seq_batch(
        &self,
        request: Request<SaveMaxSeqBatchRequest>,
    ) -> Result<Response<SaveMaxSeqResponse>, Status> {
        let req = request.into_inner();
        if let Err(e) = self.db.seq.save_max_seq_batch(&req.user_ids).await {
            return Err(Status::internal(e.to_string()));
        };
        Ok(Response::new(SaveMaxSeqResponse {}))
    }

    async fn group_create(
        &self,
        request: Request<GroupCreateRequest>,
    ) -> Result<Response<GroupCreateResponse>, Status> {
        let group = request
            .into_inner()
            .group
            .ok_or(Status::invalid_argument("group is empty"))?;

        let group_id = group.id.clone();
        // insert group information and members information to db
        let invitation = self.db.group.create_group_with_members(&group).await?;
        let members_id = invitation.members.iter().fold(
            Vec::with_capacity(invitation.members.len()),
            |mut acc, member| {
                acc.push(member.user_id.clone());
                acc
            },
        );
        // todo save the information to message receive box

        //todo save the information to cache, is it necessary to save the group info to cache?

        // save members id
        self.cache
            .save_group_members_id(&group_id, members_id)
            .await?;
        // save invitation to cache or there is no necessary to cache the group info

        let response = GroupCreateResponse {
            invitation: Some(invitation),
        };
        Ok(Response::new(response))
    }

    async fn group_invite_new(
        &self,
        request: Request<GroupInviteNewRequest>,
    ) -> Result<Response<GroupInviteNewResp>, Status> {
        let invitation = request
            .into_inner()
            .group_invite
            .ok_or(Status::invalid_argument("group_invite is empty"))?;

        let members = self.db.group.invite_new_members(&invitation).await?;

        // update cache
        self.cache
            .save_group_members_id(&invitation.group_id, invitation.members)
            .await?;

        let response = GroupInviteNewResp { members };
        Ok(Response::new(response))
    }

    async fn group_update(
        &self,
        request: Request<GroupUpdateRequest>,
    ) -> Result<Response<GroupUpdateResponse>, Status> {
        let group = request
            .into_inner()
            .group
            .ok_or(Status::invalid_argument("group information is empty"))?;

        // update db
        let group = self.db.group.update_group(&group).await?;

        // todo update cache, is it necessary to update the group info in cache?

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

        let group = self.db.group.delete_group(&group_id, &user_id).await?;

        // WE CAN NOT DELETE THE CACHE HERE, CONSUMER NEED IT TO SEND MESSAGE TO USERS
        // delete from cache, and return the members id, this is performance than db
        // self.cache.del_group_members(&group_id).await?;

        let response = GroupDeleteResponse { group: Some(group) };
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
        self.db
            .group
            .exit_group(&req.user_id, &req.group_id)
            .await?;

        // delete from cache, also get the members id
        self.cache
            .remove_group_member_id(&req.group_id, &req.user_id)
            .await?;
        let response = GroupMemberExitResponse {};
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
        let members_id = self.db.group.query_group_members_id(&group_id).await?;
        let response = GroupMembersIdResponse { members_id };
        Ok(Response::new(response))
    }

    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        debug!("receive create user request: {:?}", request);
        let mut user = request
            .into_inner()
            .user
            .ok_or(Status::invalid_argument("user is empty"))?;
        user.id = nanoid!();

        let user = self.db.user.create_user(user).await?;
        Ok(Response::new(CreateUserResponse { user: Some(user) }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let user_id = request.into_inner().user_id;
        if user_id.is_empty() {
            return Err(Status::invalid_argument("user_id is empty"));
        }
        let user = self.db.user.get_user_by_id(&user_id).await?;
        Ok(Response::new(GetUserResponse { user }))
    }

    async fn get_user_by_email(
        &self,
        request: Request<GetUserByEmailRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let email = request.into_inner().email;
        if email.is_empty() {
            return Err(Status::invalid_argument("email is empty"));
        }
        let user = self.db.user.get_user_by_email(&email).await?;
        Ok(Response::new(GetUserResponse { user }))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResponse>, Status> {
        let user = request
            .into_inner()
            .user
            .ok_or(Status::invalid_argument("user is empty"))?;

        let user = self.db.user.update_user(user).await?;
        Ok(Response::new(UpdateUserResponse { user: Some(user) }))
    }

    async fn update_user_region(
        &self,
        request: Request<UpdateRegionRequest>,
    ) -> Result<Response<UpdateRegionResponse>, Status> {
        let inner = request.into_inner();

        self.db
            .user
            .update_region(&inner.user_id, &inner.region)
            .await?;
        Ok(Response::new(UpdateRegionResponse {}))
    }

    async fn search_user(
        &self,
        request: Request<SearchUserRequest>,
    ) -> Result<Response<SearchUserResponse>, Status> {
        let SearchUserRequest { user_id, pattern } = request.into_inner();
        if pattern.is_empty() || pattern.chars().count() > 32 {
            return Err(Status::invalid_argument("keyword is empty or too long"));
        }
        let user = self.db.user.search_user(&user_id, &pattern).await?;
        Ok(Response::new(SearchUserResponse { user }))
    }

    async fn verify_password(
        &self,
        request: Request<VerifyPwdRequest>,
    ) -> Result<Response<VerifyPwdResponse>, Status> {
        let VerifyPwdRequest { account, password } = request.into_inner();
        if account.is_empty() || password.is_empty() {
            return Err(Status::invalid_argument("account or password is empty"));
        }

        let user = self.db.user.verify_pwd(&account, &password).await?;
        let response = VerifyPwdResponse { user };
        Ok(Response::new(response))
    }

    async fn create_friendship(
        &self,
        request: Request<FsCreateRequest>,
    ) -> Result<Response<FsCreateResponse>, Status> {
        let fs = request
            .into_inner()
            .fs_create
            .ok_or(Status::invalid_argument("fs_create is empty"))?;
        let (fs_req, fs_send) = self.db.friend.create_fs(fs).await?;
        Ok(Response::new(FsCreateResponse {
            fs_req: Some(fs_req),
            fs_send: Some(fs_send),
        }))
    }

    async fn agree_friendship(
        &self,
        request: Request<FsAgreeRequest>,
    ) -> Result<Response<FsAgreeResponse>, Status> {
        let fs = request
            .into_inner()
            .fs_reply
            .ok_or(Status::invalid_argument("fs_reply is empty"))?;
        let (req, send) = self.db.friend.agree_friend_apply_request(fs).await?;
        Ok(Response::new(FsAgreeResponse {
            req: Some(req),
            send: Some(send),
        }))
    }

    async fn get_friendship_list(
        &self,
        request: Request<FsListRequest>,
    ) -> Result<Response<FsListResponse>, Status> {
        let user_id = request.into_inner().user_id;
        if user_id.is_empty() {
            return Err(Status::invalid_argument("user_id is empty"));
        }
        let list = self.db.friend.get_fs_list(&user_id).await?;
        Ok(Response::new(FsListResponse { friendships: list }))
    }

    // type GetFriendListStream = Pin<Box<dyn Stream<Item = Result<Friend, Status>> + Send>>;

    async fn get_friend_list(
        &self,
        request: Request<FriendListRequest>,
    ) -> Result<Response<FriendListResponse>, Status> {
        let req = request.into_inner();
        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id is empty"));
        }
        let friends = self
            .db
            .friend
            .get_friend_list(&req.user_id, req.offline_time)
            .await?;
        Ok(Response::new(FriendListResponse { friends }))
        // Ok(Response::new(Box::pin(TonicReceiverStream::new(receiver))))
    }

    async fn update_friend_remark(
        &self,
        request: Request<UpdateRemarkRequest>,
    ) -> Result<Response<UpdateRemarkResponse>, Status> {
        let req = request.into_inner();
        self.db
            .friend
            .update_friend_remark(&req.user_id, &req.friend_id, &req.remark)
            .await?;
        Ok(Response::new(UpdateRemarkResponse {}))
    }

    async fn delete_friend(
        &self,
        request: Request<DeleteFriendRequest>,
    ) -> Result<Response<DeleteFriendResponse>, Status> {
        let inner = request.into_inner();
        self.db
            .friend
            .delete_friend(&inner.user_id, inner.id)
            .await?;
        Ok(Response::new(DeleteFriendResponse {}))
    }

    async fn query_friend_info(
        &self,
        request: Request<QueryFriendInfoRequest>,
    ) -> Result<Response<QueryFriendInfoResponse>, Status> {
        let inner = request.into_inner();
        debug!("query friend info: {:?}", inner);
        let user = self
            .db
            .user
            .get_user_by_id(&inner.user_id)
            .await?
            .ok_or(Status::not_found("user not found"))?;
        let friend = FriendInfo {
            id: user.id,
            name: user.name,
            region: user.region,
            gender: user.gender,
            avatar: user.avatar,
            account: user.account,
            signature: user.signature,
            email: user.email,
            age: user.age,
        };
        Ok(Response::new(QueryFriendInfoResponse {
            friend: Some(friend),
        }))
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
