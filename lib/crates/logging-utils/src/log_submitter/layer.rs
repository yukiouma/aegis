//! The `tracing` layer that feeds the submitter.
//!
//! This deliberately does **not** hand-roll a JSON serializer. It
//! installs a second `tracing_subscriber::fmt::Layer` configured
//! exactly like the file layer and swaps the writer, so a shipped
//! entry is byte-identical to the corresponding line in the local
//! log file. Two formatters would drift; one formatter with two
//! writers cannot.

use std::io;

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::Registry;

use super::submitter::SubmitHandle;

// The submit layer's type is deliberately not named. `.json()`
// selects a `JsonFields` field formatter that is not part of the
// crate's public surface, and `fmt::Layer`'s generic order
// (`Layer<S, N, E, W>`) is easy to misread — so the concrete type
// is spelled as `impl Layer<Registry> + Send + Sync`, which is
// exactly the bound `init_tracing` takes.

/// Mints one [`EntryWriter`] per event, which is what makes each
/// entry exactly one formatted event rather than a coalesced blob.
///
/// Only `make_writer` is implemented; `MakeWriter`'s default
/// `make_writer_for` forwards to it, and that is the method
/// `fmt::Layer` calls per event.
#[derive(Clone, Debug)]
pub struct SubmitWriter {
    handle: SubmitHandle,
}

impl<'a> MakeWriter<'a> for SubmitWriter {
    type Writer = EntryWriter;
    fn make_writer(&'a self) -> Self::Writer {
        EntryWriter {
            handle: self.handle.clone(),
            buf: Vec::new(),
        }
    }
}

/// Accumulates one event's bytes, then submits them on drop.
#[derive(Debug)]
pub struct EntryWriter {
    handle: SubmitHandle,
    buf: Vec<u8>,
}

impl io::Write for EntryWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        // Always report success. Returning an error here would make
        // `fmt::Layer` treat a full submit buffer as a logging
        // failure; the honest response to backpressure is to drop
        // the line, which `SubmitHandle` counts.
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for EntryWriter {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.buf);
        let entry = text.trim_end_matches(['\r', '\n']);
        if entry.is_empty() {
            return;
        }
        let _ = self.handle.submit(entry.to_string());
    }
}

/// Build the layer. Configure it to match the file layer:
/// `.json().with_current_span(true).with_span_list(false)`.
pub fn submit_layer(handle: SubmitHandle) -> impl Layer<Registry> + Send + Sync {
    tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(SubmitWriter { handle })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_submitter::LogSubmitter;
    use crate::log_submitter::submitter::tests_support::{RecordingSender, make_config};
    use std::time::Duration;
    use tracing_subscriber::layer::SubscriberExt;

    fn flat_entries(sender: &RecordingSender) -> Vec<String> {
        sender
            .accepted()
            .into_iter()
            .flat_map(|(_, entries)| entries)
            .collect()
    }

    fn await_entries(sender: &RecordingSender, n: usize) -> Vec<String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let flat = flat_entries(sender);
            if flat.len() >= n {
                return flat;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {n} entries; got {flat:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn captured_entry_is_the_json_formatted_event() {
        let sender = RecordingSender::shared();
        let mut submitter = LogSubmitter::new(make_config(sender.clone(), 1, 256)).unwrap();
        let subscriber = tracing_subscriber::registry()
            .with(submit_layer(submitter.handle()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(request_id = "abc", "hello from the layer");
        });
        let flat = await_entries(&sender, 1);
        let value: serde_json::Value =
            serde_json::from_str(&flat[0]).expect("entry should be valid JSON");
        // `tracing_subscriber::fmt`'s json formatter nests the
        // event's own fields under "fields"; this is the default
        // json format, unchanged.
        assert_eq!(value["fields"]["message"], "hello from the layer");
        assert_eq!(value["fields"]["request_id"], "abc");
        assert_eq!(value["level"], "INFO");
        submitter.shutdown().unwrap();
    }

    #[test]
    fn two_events_produce_two_entries() {
        let sender = RecordingSender::shared();
        let mut submitter = LogSubmitter::new(make_config(sender.clone(), 1, 256)).unwrap();
        let subscriber = tracing_subscriber::registry()
            .with(submit_layer(submitter.handle()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("first");
            tracing::info!("second");
        });
        let flat = await_entries(&sender, 2);
        let messages: Vec<String> = flat
            .iter()
            .map(|e| {
                serde_json::from_str::<serde_json::Value>(e).unwrap()["fields"]["message"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert!(
            messages.contains(&"first".to_string()),
            "got {messages:?}"
        );
        assert!(
            messages.contains(&"second".to_string()),
            "got {messages:?}"
        );
        submitter.shutdown().unwrap();
    }

    #[test]
    fn captured_entries_carry_no_trailing_newline() {
        let sender = RecordingSender::shared();
        let mut submitter = LogSubmitter::new(make_config(sender.clone(), 1, 256)).unwrap();
        let subscriber = tracing_subscriber::registry()
            .with(submit_layer(submitter.handle()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("trimmed");
        });
        let flat = await_entries(&sender, 1);
        assert!(!flat[0].ends_with('\n'), "got {:?}", flat[0]);
        assert!(!flat[0].ends_with('\r'), "got {:?}", flat[0]);
        submitter.shutdown().unwrap();
    }
}
