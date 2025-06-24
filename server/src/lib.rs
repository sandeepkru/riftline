use proto::riftline::consumer_server::{Consumer, ConsumerServer};
use proto::riftline::offset_commit_server::{OffsetCommit, OffsetCommitServer};
use proto::riftline::producer_server::{Producer, ProducerServer};
use proto::riftline::*;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};

use std::future::Future;
use std::net::SocketAddr;

pub async fn serve(
    addr: SocketAddr,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Server::builder()
        .add_service(ProducerServer::new(BasicProducer))
        .add_service(ConsumerServer::new(BasicConsumer))
        .add_service(OffsetCommitServer::new(BasicOffsetCommit))
        .serve_with_shutdown(addr, shutdown)
        .await?;
    Ok(())
}

#[derive(Default)]
struct BasicProducer;

#[tonic::async_trait]
impl Producer for BasicProducer {
    async fn produce(
        &self,
        _request: Request<ProduceRequest>,
    ) -> Result<Response<ProduceResponse>, Status> {
        Ok(Response::new(ProduceResponse {
            partition: 0,
            offset: 0,
        }))
    }
}

#[derive(Default)]
struct BasicConsumer;

#[tonic::async_trait]
impl Consumer for BasicConsumer {
    type ConsumeStream = ReceiverStream<Result<ConsumeResponse, Status>>;

    async fn consume(
        &self,
        _request: Request<ConsumeRequest>,
    ) -> Result<Response<Self::ConsumeStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        drop(tx);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[derive(Default)]
struct BasicOffsetCommit;

#[tonic::async_trait]
impl OffsetCommit for BasicOffsetCommit {
    async fn commit(
        &self,
        _request: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        Ok(Response::new(CommitResponse { success: true }))
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn server_starts_ephemeral() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let fut = tokio::time::sleep(std::time::Duration::from_millis(10));
        assert!(serve(addr, fut).await.is_ok());
    }
}
