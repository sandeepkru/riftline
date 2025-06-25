use common::object_store::{LocalFileSystemStore, ObjectStore};
use proto::riftline::consumer_server::{Consumer, ConsumerServer};
use proto::riftline::offset_commit_server::{OffsetCommit, OffsetCommitServer};
use proto::riftline::producer_server::{Producer, ProducerServer};
use proto::riftline::*;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn serve(
    addr: SocketAddr,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = Arc::new(LocalFileSystemStore::new("data"));

    Server::builder()
        .add_service(ProducerServer::new(BasicProducer::new(store)))
        .add_service(ConsumerServer::new(BasicConsumer))
        .add_service(OffsetCommitServer::new(BasicOffsetCommit))
        .serve_with_shutdown(addr, shutdown)
        .await?;
    Ok(())
}

#[derive(Clone)]
struct BasicProducer {
    store: Arc<dyn ObjectStore + Send + Sync>,
}

impl BasicProducer {
    fn new(store: Arc<dyn ObjectStore + Send + Sync>) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl Producer for BasicProducer {
    async fn produce(
        &self,
        request: Request<ProduceRequest>,
    ) -> Result<Response<ProduceResponse>, Status> {
        let req = request.into_inner();
        let mut bytes = Vec::new();
        for msg in req.messages {
            bytes.extend_from_slice(&msg);
            bytes.push(b'\n');
        }

        let key = format!("segments/{}/segment_0.log", req.topic);

        self.store
            .put(&key, &bytes)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

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
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    #[derive(Default, Clone)]
    struct MockStore {
        data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    #[tonic::async_trait]
    impl ObjectStore for MockStore {
        async fn put(&self, key: &str, bytes: &[u8]) -> std::io::Result<()> {
            self.data
                .lock()
                .unwrap()
                .insert(key.to_string(), bytes.to_vec());
            Ok(())
        }

        async fn get(&self, key: &str) -> std::io::Result<Vec<u8>> {
            self.data
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
        }
    }

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

    #[tokio::test]
    async fn producer_writes_to_store() {
        let store = Arc::new(MockStore::default());
        let producer = BasicProducer::new(store.clone());

        let request = Request::new(ProduceRequest {
            topic: "test".into(),
            messages: vec![b"one".to_vec(), b"two".to_vec()],
        });

        producer.produce(request).await.unwrap();

        let data = store
            .data
            .lock()
            .unwrap()
            .get("segments/test/segment_0.log")
            .cloned()
            .unwrap();

        assert_eq!(data, b"one\ntwo\n".to_vec());
    }

    #[tokio::test]
    async fn producer_persists_to_files() {
        let dir = tempdir().unwrap();
        let store = Arc::new(LocalFileSystemStore::new(dir.path()));
        let producer = BasicProducer::new(store);

        let request = Request::new(ProduceRequest {
            topic: "topic".into(),
            messages: vec![b"hello".to_vec()],
        });

        producer.produce(request).await.unwrap();

        let path = dir.path().join("segments/topic/segment_0.log");
        let bytes = tokio::fs::read(path).await.unwrap();
        assert_eq!(bytes, b"hello\n");
    }
}
