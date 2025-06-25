use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

use common::object_store::LocalFileSystemStore;
use proto::riftline::consumer_client::ConsumerClient;
use proto::riftline::offset_commit_client::OffsetCommitClient;
use proto::riftline::producer_client::ProducerClient;
use proto::riftline::*;
use server::serve_with_listener;

#[tokio::test]
async fn persistence_roundtrip() {
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

    let endpoint = format!("http://{}", addr);
    let mut producer = ProducerClient::connect(endpoint.clone()).await.unwrap();
    producer
        .produce(ProduceRequest {
            topic: "topic".into(),
            messages: vec![b"m1".to_vec(), b"m2".to_vec()],
        })
        .await
        .unwrap();

    let mut consumer = ConsumerClient::connect(endpoint.clone()).await.unwrap();
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
    assert_eq!(messages, vec![b"m1".to_vec(), b"m2".to_vec()]);

    let mut offset = OffsetCommitClient::connect(endpoint).await.unwrap();
    offset
        .commit(CommitRequest {
            topic: "topic".into(),
            partition: 0,
            offset: messages.len() as i64,
            consumer_group: "group".into(),
        })
        .await
        .unwrap();

    let resp = offset
        .fetch(FetchRequest {
            topic: "topic".into(),
            partition: 0,
            consumer_group: "group".into(),
        })
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.offset, messages.len() as i64);

    tx.send(()).unwrap();
}
