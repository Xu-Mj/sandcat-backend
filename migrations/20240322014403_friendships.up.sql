CREATE TYPE friend_request_status AS ENUM ('Pending', 'Accepted', 'Rejected', 'Blacked', 'Deleted');
create table friendships
(
    id          VARCHAR primary key,
    user_id     VARCHAR               NOT NULL,
    friend_id   VARCHAR               NOT NULL,
    status      friend_request_status NOT NULL DEFAULT 'Pending',
    apply_msg   VARCHAR,
    req_remark  VARCHAR,
    resp_msg    VARCHAR,
    resp_remark VARCHAR,
    source      VARCHAR,
    create_time BIGINT                NOT NULL,
    update_time BIGINT,
    Unique (user_id, friend_id)
)

-- A 申请 B 为好友，
-- 1. 向数据库插入一条数据；
-- 2. 生成一条好友请求消息存入B的收件箱（mongodb）

-- 2.1 B用户在线，通过websocket实时收到A好友请求消息，
-- 2.2 B收到该消息，入库成功后向服务器返回已经收到该消息，并且弹出好友请求消息。
-- 2.3 服务器收到B的收到消息，删除收件箱中的消息。
--      （防止其他设备登陆时此次好友请求已经被处理，然后再次拉取到该消息，造成好友关系状态不一致；
--          其他设备同步好友关系不采取这种方式，存入收件箱的目的是为了方便主力设备能够第一时间对齐服务器消息）
--
-- B同意该请求
-- 1. 如果A拉黑B或者删除B，那么更新该数据的状态为已拉黑或者已删除。没有问题
-- 2. 如果B拉黑A或者删除A，如何标记？
--  添加operator字段，用来标记操作者？

--  如果处于拉黑状态：A拉黑B
-- 1. 恢复正常好友关系：
--      A：只需取消拉黑
--          新增接口，恢复好友关系：因为可能存在B再次申请了好友请求，而此时A在B的好友信息页面操作恢复好友关系。
--      B：需要重新申请好友关系，
--          更新好友请求状态为Pending，
--  问题： 如果在此期间B删除了A，该怎么办？
--  方案：1. 无法恢复好友关系，通知用户A，B已经删除了A，需要重新申请好友关系。
--             2.
-- 2. 删除好友关系：A删除B。
--      修改该好友关系数据状态为已删除。并且更新update_time,用来其他设备上线时拉取该修改动作
--      B收到删除操作的通知，标记为被删除，如果发送消息那么需要重新申请好友关系。
--      A直接删除B，同时清理A客户端中的B好友信息以及聊天记录。
--

-- 每个用户在操作已经达成好友关系的好友信息时，除了同步修改对方的好友关系状态，都只能修改自己的好友关系数据。
-- 例如，A拉黑B，需要修改两条好友记录的状态。
-- 但是如果是删除操作，那么需要修改B的好友关系数据状态为已删除同时删除A自己的好友关系数据。

-- 其他设备登录，对齐好友关系（包括主力设备，因为用户可能交替使用不同的设备，当发生消息互发时，在线设备便是主力设备）
-- 登录设备会保存离线时间，客户端在拉取好友关系数据时，会携带两个参数：
--      { user_id , last_offline_time}服务器根据用户id以及上次离线时间获取好友关系数据
-- 当该设备第一次登录时，last_offline_time为0，此时服务器会返回所有好友关系数据。
-- 当该设备再次登录时，last_offline_time不为0，此时服务器会返回该设备上次离线时，好友关系数据更新后的数据。
-- 数据库只保存最新的好友关系数据，并且根据update_time与last_offline_time进行对比，
-- 如果update_time大于last_offline_time，则返回该数据。
-- 同时，客户端需要对这些数据进行分类处理
-- 1. 如果好友请求状态为Pending，并且本地状态为Pending，但是本地数据中的update_time与服务器返回的最新数据中的update_time不一致，那么更新本地数据并弹出消息通知；
-- 1.2 如果本地没有数据，那么直接入库并弹出消息通知；
-- 1.3 不存在本地数据与服务器返回的数据一致的情况
-- 2. 如果好友请求状态为Accepted，并且本地数据状态为Pending，那么更新本地数据并弹出消息通知；
-- 2.1 如果本地没有数据，那么直接入库，不需要其他处理（不需要拉取好友信息，会有单独的好友对齐操作）
-- 3. 如果好友请求状态为Blacked，put数据库
-- 4. 如果好友请求状态为Deleted，put数据库
-- 》》总结：好像除了Pending状态，其他状态都可以直接put数据库，
-- ***拉取好友列表***
-- 同样根据用户id以及上次离线时间拉取好友列表
