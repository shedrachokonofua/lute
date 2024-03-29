use super::{
  crawler_interactor::{CrawlerInteractor, CrawlerMonitor},
  crawler_state_repository::CrawlerStatus,
  priority_queue::{ClaimedQueueItem, Priority, QueueItem, QueuePushParameters},
};
use crate::{
  files::file_metadata::file_name::FileName,
  proto::{self, EnqueueRequest, GetCrawlerMonitorReply, SetCrawlerStatusReply, SetStatusRequest},
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;

impl From<CrawlerMonitor> for proto::CrawlerMonitor {
  fn from(val: CrawlerMonitor) -> Self {
    proto::CrawlerMonitor {
      status: proto::CrawlerStatus::from(val.status).into(),
      size: val.size,
      claimed_item_count: val.claimed_item_count,
      claimed_items: val
        .claimed_items
        .into_iter()
        .map(|item| item.into())
        .collect(),
      remaining_window_requests: val.remaining_window_requests,
      window_request_count: val.window_request_count,
    }
  }
}

impl From<CrawlerStatus> for proto::CrawlerStatus {
  fn from(val: CrawlerStatus) -> Self {
    match val {
      CrawlerStatus::Running => proto::CrawlerStatus::CrawlerRunning,
      CrawlerStatus::Paused => proto::CrawlerStatus::CrawlerPaused,
      CrawlerStatus::Draining => proto::CrawlerStatus::CrawlerDraining,
      CrawlerStatus::Throttled => proto::CrawlerStatus::CrawlerThrottled,
    }
  }
}

impl From<proto::CrawlerStatus> for CrawlerStatus {
  fn from(val: proto::CrawlerStatus) -> Self {
    match val {
      proto::CrawlerStatus::CrawlerRunning => CrawlerStatus::Running,
      proto::CrawlerStatus::CrawlerPaused => CrawlerStatus::Paused,
      proto::CrawlerStatus::CrawlerDraining => CrawlerStatus::Draining,
      proto::CrawlerStatus::CrawlerThrottled => CrawlerStatus::Throttled,
    }
  }
}

impl From<Priority> for proto::CrawlerItemPriority {
  fn from(val: Priority) -> Self {
    match val {
      Priority::Express => proto::CrawlerItemPriority::Express,
      Priority::High => proto::CrawlerItemPriority::High,
      Priority::Standard => proto::CrawlerItemPriority::Standard,
      Priority::Low => proto::CrawlerItemPriority::Low,
    }
  }
}

impl From<proto::CrawlerItemPriority> for Priority {
  fn from(val: proto::CrawlerItemPriority) -> Self {
    match val {
      proto::CrawlerItemPriority::Express => Priority::Express,
      proto::CrawlerItemPriority::High => Priority::High,
      proto::CrawlerItemPriority::Standard => Priority::Standard,
      proto::CrawlerItemPriority::Low => Priority::Low,
    }
  }
}

impl From<QueueItem> for proto::CrawlerQueueItem {
  fn from(val: QueueItem) -> Self {
    proto::CrawlerQueueItem {
      item_key: val.item_key.to_string(),
      enqueue_time: val.enqueue_time.to_string(),
      deduplication_key: val.deduplication_key,
      file_name: val.file_name.0,
      priority: proto::CrawlerItemPriority::from(val.priority).into(),
      correlation_id: val.correlation_id,
      metadata: val.metadata.unwrap_or_default(),
    }
  }
}

impl From<ClaimedQueueItem> for proto::ClaimedCrawlerQueueItem {
  fn from(val: ClaimedQueueItem) -> Self {
    proto::ClaimedCrawlerQueueItem {
      item: Some(val.item.into()),
      claim_ttl_seconds: val.claim_ttl_seconds,
    }
  }
}

impl TryFrom<EnqueueRequest> for QueuePushParameters {
  type Error = anyhow::Error;

  fn try_from(val: EnqueueRequest) -> Result<Self, Self::Error> {
    Ok(QueuePushParameters {
      file_name: FileName::try_from(val.file_name.clone())?,
      priority: Some(Priority::from(val.priority())),
      deduplication_key: Some(val.deduplication_key),
      correlation_id: val.correlation_id,
      metadata: Some(val.metadata),
    })
  }
}

pub struct CrawlerService {
  pub crawler_interactor: Arc<CrawlerInteractor>,
}

#[tonic::async_trait]
impl proto::CrawlerService for CrawlerService {
  async fn get_monitor(
    &self,
    request: Request<()>,
  ) -> Result<Response<GetCrawlerMonitorReply>, Status> {
    let _ = request.into_inner();
    let monitor = self.crawler_interactor.get_monitor().await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Internal server error")
    })?;
    let reply = GetCrawlerMonitorReply {
      monitor: Some(monitor.into()),
    };
    Ok(Response::new(reply))
  }

  async fn set_status(
    &self,
    request: Request<SetStatusRequest>,
  ) -> Result<Response<SetCrawlerStatusReply>, Status> {
    let status = CrawlerStatus::from(request.into_inner().status());
    self
      .crawler_interactor
      .set_status(status)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;
    let reply = SetCrawlerStatusReply {
      status: proto::CrawlerStatus::from(status).into(),
    };
    Ok(Response::new(reply))
  }

  async fn enqueue(&self, request: Request<EnqueueRequest>) -> Result<Response<()>, Status> {
    self
      .crawler_interactor
      .enqueue(request.into_inner().try_into().map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;

    Ok(Response::new(()))
  }

  async fn empty(&self, _request: Request<()>) -> Result<Response<()>, Status> {
    self.crawler_interactor.empty_queue().await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Internal server error")
    })?;

    Ok(Response::new(()))
  }

  async fn reset_limiter(&self, _request: Request<()>) -> Result<Response<()>, Status> {
    self
      .crawler_interactor
      .reset_window_request_count()
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;

    Ok(Response::new(()))
  }

  async fn remove_throttle(&self, _request: Request<()>) -> Result<Response<()>, Status> {
    self
      .crawler_interactor
      .remove_throttle()
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;

    Ok(Response::new(()))
  }
}
