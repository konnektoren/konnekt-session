use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

pub trait ConnectionHandler {
    type InternMessageType: Clone + Debug;
    type ExternMessageType: Clone + Debug;
    type CallbackType: Sized;
    type ExternSenderType;
    type ExternReceiverType;

    fn receiver(&self) -> Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>;

    fn sender(&self) -> UnboundedSender<Self::InternMessageType>;

    async fn next_message(
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    ) -> Option<Self::InternMessageType>;

    fn spawn_send_task(
        &self,
        sender: Self::ExternSenderType,
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    );

    fn spawn_receive_task(
        &self,
        receiver: Self::ExternReceiverType,
        callback: Arc<Self::CallbackType>,
    );
}
