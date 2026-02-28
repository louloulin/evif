// EVIF gRPC 服务

mod server;
mod client;
mod error;

pub use server::{EvifServer, ServerConfig};
pub use client::{EvifClient, ClientConfig};
pub use error::{GrpcError, GrpcResult};

// 重新导出生成的 proto 代码
pub mod evif {
    tonic::include_proto!("evif");
}

// 重新导出常用的消息类型
pub use evif::{GetNodeRequest, GetNodeResponse, PutNodeRequest, PutNodeResponse,
               DeleteNodeRequest, DeleteNodeResponse, BatchGetNodesRequest, BatchPutNodesResponse,
               QueryRequest, NodeResponse, ReadFileRequest, WriteFileResponse,
               StatsRequest, StatsResponse, HealthRequest, HealthResponse,
               Node, DataChunk, Value, value};

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
