use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

struct VecWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn json_log_line() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = buffer.clone();
    let subscriber = fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_writer(move || VecWriter(writer_buf.clone()))
        .json()
        .flatten_event(true)
        .with_target(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        info!(module = "test_mod", "test message");
    });

    let bytes = buffer.lock().unwrap();
    let line = std::str::from_utf8(&bytes).unwrap().trim();
    serde_json::from_str::<serde_json::Value>(line).unwrap();
}
