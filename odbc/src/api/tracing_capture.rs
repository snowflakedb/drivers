use std::sync::{Arc, Mutex};

use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};

#[derive(Clone)]
pub(crate) struct CaptureLayer {
    pub(crate) messages: Arc<Mutex<Vec<String>>>,
}

impl CaptureLayer {
    pub(crate) fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn snapshot(&self) -> Vec<String> {
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let normalized = sf_core::logging::normalize_event(event);
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(normalized.message);
    }
}

pub(crate) fn capture_messages(emit: impl FnOnce()) -> Vec<String> {
    let layer = CaptureLayer::new();
    let subscriber = tracing_subscriber::registry().with(layer.clone());
    tracing::subscriber::with_default(subscriber, emit);
    layer.snapshot()
}
