use proto::riftline::consumer_server::{Consumer, ConsumerServer};
use proto::riftline::offset_commit_server::{OffsetCommit, OffsetCommitServer};
use proto::riftline::producer_server::{Producer, ProducerServer};
use proto::riftline::*;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct MockProducer;

#[tonic::async_trait]
impl Producer for MockProducer {
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
struct MockConsumer;

#[tonic::async_trait]
impl Consumer for MockConsumer {
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
struct MockOffsetCommit;

#[tonic::async_trait]
impl OffsetCommit for MockOffsetCommit {
    async fn commit(
        &self,
        _request: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        Ok(Response::new(CommitResponse { success: true }))
    }
}

#[tokio::test]
async fn servers_build() {
    let _p = ProducerServer::new(MockProducer::default());
    let _c = ConsumerServer::new(MockConsumer::default());
    let _o = OffsetCommitServer::new(MockOffsetCommit::default());
}
