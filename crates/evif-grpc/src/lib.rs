// EVIF gRPC 服务

mod client;
mod error;
mod server;

pub use client::{ClientConfig, EvifClient};
pub use error::{GrpcError, GrpcResult};
pub use server::{EvifServer, ServerConfig};

// 重新导出生成的 proto 代码
pub mod evif {
    tonic::include_proto!("evif");
}

// 重新导出常用的消息类型
pub use evif::{
    value, BatchGetNodesRequest, BatchPutNodesResponse, DataChunk, DeleteNodeRequest,
    DeleteNodeResponse, GetNodeRequest, GetNodeResponse, HealthRequest, HealthResponse, Node,
    NodeResponse, PutNodeRequest, PutNodeResponse, QueryRequest, ReadFileRequest, StatsRequest,
    StatsResponse, Value, WriteFileResponse,
};

// 重新导出服务端的 trait 和结构
pub use evif::evif_service_server::{EvifService as EvifServerTrait, EvifServiceServer};

use tonic::Status;

impl From<GrpcError> for Status {
    fn from(err: GrpcError) -> Self {
        match err {
            GrpcError::NotFound(msg) => Status::not_found(msg),
            GrpcError::Vfs(err) => Status::internal(err),
            GrpcError::Io(err) => Status::internal(err.to_string()),
            GrpcError::Protocol(msg) => Status::invalid_argument(msg),
            GrpcError::Internal(msg) => Status::internal(msg),
            GrpcError::AddrParse(err) => Status::internal(err.to_string()),
        }
    }
}
