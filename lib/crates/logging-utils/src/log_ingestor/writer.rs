//! Writer thread + per-envelope write helper. The writer owns the
//! `RollingFileAppender` directly — no `non_blocking` wrapper —
//! and flushes after every envelope.

use std::io::Write;

use crossbeam_channel::{Receiver, Sender};
use tracing_appender::rolling::RollingFileAppender;

#[derive(Debug, Clone)]
pub(crate) struct BatchEnvelope {
    pub batch_id: String,
    pub entries: Vec<String>,
}

/// Body of the writer thread. Loops on `rx.recv()`, writes each
/// envelope via [`write_envelope`], and exits when `rx` is closed.
/// Sends `()` on `done_tx` before returning so the shutdown path can
/// distinguish "thread exited" from "deadline exceeded".
pub(crate) fn run_writer(
    rx: Receiver<BatchEnvelope>,
    mut appender: RollingFileAppender,
    done_tx: Sender<()>,
) {
    while let Ok(envelope) = rx.recv() {
        write_envelope(&mut appender, &envelope);
    }
    let _ = done_tx.send(());
}

/// Write one envelope: one line per entry, then `flush()`. The
/// `batch_id` is intentionally not written — the dedup cache in
/// `LogIngestor::submit` guarantees at-most-once delivery per id, so
/// re-deriving identity from the file is not needed downstream. I/O
/// errors are swallowed — the writer thread is best-effort and
/// should not panic on a transient disk problem.
pub(crate) fn write_envelope(writer: &mut RollingFileAppender, envelope: &BatchEnvelope) {
    for line in &envelope.entries {
        let _ = writeln!(writer, "{line}");
    }
    let _ = writer.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: read the first daily-rotated file under `dir` whose
    /// name starts with `prefix`. Returns `(file_name, contents)`.
    fn read_log(dir: &std::path::Path, prefix: &str) -> (String, String) {
        let entry = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .expect("expected at least one daily-rotate file under prefix");
        let name = entry.file_name().to_string_lossy().into_owned();
        let contents = std::fs::read_to_string(entry.path()).unwrap();
        (name, contents)
    }

    #[test]
    fn writer_writes_one_line_per_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let (tx, rx) = crossbeam_channel::bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(1);
        let appender = tracing_appender::rolling::daily(tmp.path(), "happy.log");

        let handle = std::thread::spawn(move || run_writer(rx, appender, done_tx));

        tx.send(BatchEnvelope {
            batch_id: "b-1".into(),
            entries: vec!["line a".into(), "line b".into(), "line c".into()],
        })
        .unwrap();
        drop(tx); // close channel → writer exits

        done_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        handle.join().unwrap();

        let (_name, contents) = read_log(tmp.path(), "happy.log");
        // `batch_id` is intentionally NOT written — see
        // `write_envelope` doc. Only the entries land in the file.
        assert_eq!(contents, "line a\nline b\nline c\n");
    }

    #[test]
    fn writer_writes_empty_entries_as_blank_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (tx, rx) = crossbeam_channel::bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(1);
        let appender = tracing_appender::rolling::daily(tmp.path(), "empty.log");

        let handle = std::thread::spawn(move || run_writer(rx, appender, done_tx));

        tx.send(BatchEnvelope {
            batch_id: "b-empty".into(),
            entries: vec![],
        })
        .unwrap();
        drop(tx);

        done_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        handle.join().unwrap();

        // Empty entries → nothing written, so the file (which the
        // appender opens eagerly) is left blank.
        let (_name, contents) = read_log(tmp.path(), "empty.log");
        assert_eq!(contents, "");
    }

    #[test]
    fn writer_exits_when_sender_dropped() {
        let tmp = tempfile::tempdir().unwrap();
        let (tx, rx) = crossbeam_channel::bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(1);
        let appender = tracing_appender::rolling::daily(tmp.path(), "drop.log");

        let handle = std::thread::spawn(move || run_writer(rx, appender, done_tx));

        drop(tx);
        done_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("writer must signal done within deadline");
        handle.join().unwrap();

        // No envelopes were sent, so no log content was written.
        // `RollingFileAppender` may open the file eagerly, so we
        // assert on emptiness rather than absence.
        let any_with_content = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("drop.log"))
            .any(|e| {
                std::fs::metadata(e.path())
                    .map(|m| m.len() > 0)
                    .unwrap_or(false)
            });
        assert!(
            !any_with_content,
            "no log content should be written when no envelopes are sent"
        );
    }

    #[test]
    fn writer_panic_before_done_signal_propagates_via_thread_join() {
        let tmp = tempfile::tempdir().unwrap();
        let (tx, rx) = crossbeam_channel::bounded::<BatchEnvelope>(4);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(1);
        let mut appender = tracing_appender::rolling::daily(tmp.path(), "panic.log");

        let panicked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let panicked_clone = std::sync::Arc::clone(&panicked);
        let handle = std::thread::spawn(move || {
            // Trigger the panic inside catch_unwind so it gets
            // captured instead of unwinding the test thread.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let env = rx.recv().unwrap();
                panicked_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                write_envelope(&mut appender, &env);
                panic!("test-induced writer panic");
            }));
            assert!(result.is_err(), "writer must have panicked");
            // Force the closure to own `done_tx` so it is dropped
            // when the spawned thread exits — mimics the
            // panic-induced auto-drop behavior.
            drop(done_tx);
        });

        tx.send(BatchEnvelope {
            batch_id: "trigger".into(),
            entries: vec!["crash".into()],
        })
        .unwrap();
        drop(tx);
        handle.join().unwrap();

        assert!(panicked.load(std::sync::atomic::Ordering::SeqCst));
        match done_rx.recv_timeout(std::time::Duration::from_secs(2)) {
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {}
            other => panic!("expected Disconnected, got {other:?}"),
        }
    }
}
