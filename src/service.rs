use anyhow::Result;
use async_nats::Subscriber;
use futures_util::stream::StreamExt;
use infrastructure::messaging;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::Workspace;

#[derive(Clone)]
pub struct ServiceController {
    tracker: TaskTracker,
    cancel_token: CancellationToken,
}

impl ServiceController {
    pub fn new() -> Self {
        let cancel_token = CancellationToken::new();
        let tracker = TaskTracker::new();
        Self {
            tracker,
            cancel_token,
        }
    }

    pub async fn stop(&self) {
        self.cancel_token.cancel();
        self.tracker.close();
        self.tracker.wait().await
    }
}

pub struct WorkspaceService {
    controller: ServiceController,
    subject: String,
}

struct WorkspaceServiceContext {
    workspace: Workspace,
    channel: messaging::MessagingChannel,
}

impl WorkspaceService {
    pub async fn start(workspace: Workspace) -> Result<Self> {
        let (channel, subscriber) =
            messaging::MessagingChannel::establish("workspace".to_string()).await?;
        let subject = channel.channel_instance_subject.clone();
        let controller = WorkspaceServiceContext::run(channel, subscriber, workspace);

        Ok(Self {
            controller,
            subject,
        })
    }

    pub async fn stop(&self) {
        self.controller.stop().await;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CommandMessage {
    command: String,
    arguments: serde_json::Value,
}

type ResponseMessage = Result<serde_json::Value>;

impl WorkspaceServiceContext {
    fn run(
        channel: messaging::MessagingChannel,
        subscriber: Subscriber,
        workspace: Workspace,
    ) -> ServiceController {
        let controller = ServiceController::new();
        let tracker = controller.tracker.clone();
        let cancel_token = controller.cancel_token.clone();

        let context = WorkspaceServiceContext { workspace, channel };

        context.handle_messages(subscriber, tracker, cancel_token);

        controller
    }

    fn handle_messages(
        self,
        mut subscriber: Subscriber,
        tracker: TaskTracker,
        cancel_token: CancellationToken,
    ) {
        tracker.spawn(async move {
            loop {
                tokio::select! {
                    Some(message) = subscriber.next() => {
                        let content_bytes = message.payload;
                        let content = std::str::from_utf8(&content_bytes).unwrap();
												self.handle_command(serde_json::from_str(content).unwrap());
                    }
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    fn handle_command(&self, message: CommandMessage) {
        println!("{:?}", message);
    }
}
