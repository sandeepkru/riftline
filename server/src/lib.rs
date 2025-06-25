use common::object_store::{LocalFileSystemStore, ObjectStore};
use proto::riftline::consumer_server::{Consumer, ConsumerServer};
use proto::riftline::offset_commit_server::{OffsetCommit, OffsetCommitServer};
use proto::riftline::producer_server::{Producer, ProducerServer};
use proto::riftline::*;
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Request, Response, Status, transport::Server};

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

type OffsetKey = (String, i32, String);
type OffsetMap = HashMap<OffsetKey, i64>;
/// Shared map of committed offsets indexed by (topic, partition, group).
type SharedOffsetMap = Arc<Mutex<OffsetMap>>;

pub async fn serve(
    addr: SocketAddr,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = Arc::new(LocalFileSystemStore::new("data"));
    Server::builder()
        .add_service(ProducerServer::new(BasicProducer::new(store.clone())))
        .add_service(ConsumerServer::new(BasicConsumer::new(store)))
        .add_service(OffsetCommitServer::new(BasicOffsetCommit::default()))
        .serve_with_shutdown(addr, shutdown)
        .await?;
    Ok(())
}

pub async fn serve_with_listener(
    listener: tokio::net::TcpListener,
    store: Arc<dyn ObjectStore + Send + Sync>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Server::builder()
        .add_service(ProducerServer::new(BasicProducer::new(store.clone())))
        .add_service(ConsumerServer::new(BasicConsumer::new(store)))
        .add_service(OffsetCommitServer::new(BasicOffsetCommit::default()))
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
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

#[derive(Clone)]
struct BasicConsumer {
    store: Arc<dyn ObjectStore + Send + Sync>,
}

impl BasicConsumer {
    fn new(store: Arc<dyn ObjectStore + Send + Sync>) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl Consumer for BasicConsumer {
    type ConsumeStream = ReceiverStream<Result<ConsumeResponse, Status>>;

    async fn consume(
        &self,
        request: Request<ConsumeRequest>,
    ) -> Result<Response<Self::ConsumeStream>, Status> {
        let req = request.into_inner();
        let key = format!("segments/{}/segment_{}.log", req.topic, req.partition);
        let data = self
            .store
            .get(&key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let messages: Vec<_> = data
            .split(|b| *b == b'\n')
            .filter(|m| !m.is_empty())
            .map(|m| m.to_vec())
            .collect();

        let start = req.offset as usize;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            let mut offset = req.offset;
            for msg in messages.into_iter().skip(start) {
                if tx
                    .send(Ok(ConsumeResponse {
                        offset,
                        message: msg,
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
                offset += 1;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[derive(Clone)]
struct BasicOffsetCommit {
    offsets: SharedOffsetMap,
}

impl Default for BasicOffsetCommit {
    fn default() -> Self {
        Self {
            offsets: Arc::new(Mutex::new(OffsetMap::new())),
        }
    }
}

#[tonic::async_trait]
impl OffsetCommit for BasicOffsetCommit {
    async fn commit(
        &self,
        request: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        let req = request.into_inner();
        self.offsets
            .lock()
            .unwrap()
            .insert((req.topic, req.partition, req.consumer_group), req.offset);
        Ok(Response::new(CommitResponse { success: true }))
    }

    async fn fetch(
        &self,
        request: Request<FetchRequest>,
    ) -> Result<Response<FetchResponse>, Status> {
        let req = request.into_inner();
        let offset = *self
            .offsets
            .lock()
            .unwrap()
            .get(&(req.topic, req.partition, req.consumer_group))
            .unwrap_or(&0);
        Ok(Response::new(FetchResponse { offset }))
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::riftline::consumer_client::ConsumerClient;
    use proto::riftline::producer_client::ProducerClient;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;
    use tokio::net::TcpListener;

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

    async fn start_server_temp() -> (
        SocketAddr,
        tokio::sync::oneshot::Sender<()>,
        tempfile::TempDir,
    ) {
        let dir = tempdir().unwrap();
        let store = Arc::new(LocalFileSystemStore::new(dir.path()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let _ = serve_with_listener(listener, store, async {
                rx.await.ok();
            })
            .await;
        });
        (addr, tx, dir)
    }

    #[tokio::test]
    async fn produce_then_consume() {
        let (addr, shutdown, _dir) = start_server_temp().await;

        let endpoint = format!("http://{}", addr);
        let mut producer = ProducerClient::connect(endpoint.clone()).await.unwrap();
        producer
            .produce(ProduceRequest {
                topic: "topic".into(),
                messages: vec![b"one".to_vec(), b"two".to_vec()],
            })
            .await
            .unwrap();

        let mut consumer = ConsumerClient::connect(endpoint).await.unwrap();
        let response = consumer
            .consume(ConsumeRequest {
                topic: "topic".into(),
                partition: 0,
                offset: 0,
            })
            .await
            .unwrap();
        let mut stream = response.into_inner();
        let mut messages = Vec::new();
        while let Some(msg) = stream.message().await.unwrap() {
            messages.push(msg.message);
        }

        assert_eq!(messages, vec![b"one".to_vec(), b"two".to_vec()]);

        shutdown.send(()).unwrap();
    }

    #[tokio::test]
    async fn commit_and_fetch_roundtrip() {
        let service = BasicOffsetCommit::default();

        service
            .commit(Request::new(CommitRequest {
                topic: "topic".into(),
                partition: 0,
                offset: 5,
                consumer_group: "group".into(),
            }))
            .await
            .unwrap();

        let resp = service
            .fetch(Request::new(FetchRequest {
                topic: "topic".into(),
                partition: 0,
                consumer_group: "group".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.offset, 5);
    }

    #[tokio::test]
    async fn commit_overwrite() {
        let service = BasicOffsetCommit::default();

        service
            .commit(Request::new(CommitRequest {
                topic: "t".into(),
                partition: 0,
                offset: 1,
                consumer_group: "g".into(),
            }))
            .await
            .unwrap();

        service
            .commit(Request::new(CommitRequest {
                topic: "t".into(),
                partition: 0,
                offset: 2,
                consumer_group: "g".into(),
            }))
            .await
            .unwrap();

        let resp = service
            .fetch(Request::new(FetchRequest {
                topic: "t".into(),
                partition: 0,
                consumer_group: "g".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.offset, 2);
    }

    #[tokio::test]
    async fn fetch_defaults_to_zero() {
        let service = BasicOffsetCommit::default();

        let resp = service
            .fetch(Request::new(FetchRequest {
                topic: "t".into(),
                partition: 0,
                consumer_group: "g".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.offset, 0);
    }
}
