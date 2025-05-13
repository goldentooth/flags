use super::container::ShutdownContainer;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::Mutex, task::JoinHandle, time};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

#[derive(Clone)]
pub struct ShutdownManager {
  cancel_token: CancellationToken,
  tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl ShutdownManager {
  pub fn new() -> Self {
    Self {
      cancel_token: CancellationToken::new(),
      tasks: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub fn cancel_token(&self) -> CancellationToken {
    self.cancel_token.clone()
  }

  pub fn cancel(&self) {
    self.cancel_token.cancel();
  }

  pub async fn spawn<F>(&self, name: &str, fut: F)
  where
    F: Future<Output = ()> + Send + 'static,
  {
    let task = tokio::spawn(fut);
    let mut tasks = self.tasks.lock().await;
    let name = name.to_string();
    tasks.insert(name.to_string(), task);
  }

  pub async fn spawn_guarded<F, Fut>(
    &self,
    name: &'static str,
    container: &ShutdownContainer,
    fut: F,
  ) where
    F: FnOnce(CancellationToken, ShutdownContainer) -> Fut + Send + 'static,
    Fut: Future<Output = eyre::Result<()>> + Send + 'static,
  {
    let cancel_token = self.cancel_token();
    let cancel_token2 = self.cancel_token();
    let container = container.clone();
    self
      .spawn(name, async move {
        if let Err(error) = fut(cancel_token, container).await {
          error!("{} failed: {}", name, error);
          cancel_token2.cancel();
        }
      })
      .await;
  }

  pub async fn shutdown(self) {
    tokio::select! {
      _ = self.cancel_token.cancelled() => {
        info!("Cancel token triggered.");
      }
      _ = tokio::signal::ctrl_c() => {
        info!("Received Ctrl+C signal.");
      }
    }
    let mut tasks = self.tasks.lock().await;
    let duration = Duration::from_secs(5);
    info!("Waiting for tasks to complete...");
    for (name, mut task) in tasks.drain() {
      if let Err(error) = time::timeout(duration, &mut task).await {
        debug!("Task {} failed to complete in time: {:?}", name, error);
        debug!("Forcefully aborting task {}.", name);
        task.abort();
      } else {
        trace!("Task {} completed successfully.", name);
      }
    }
    info!("All tasks completed.");
  }
}
