use tokio::sync::mpsc;

pub async fn try_recv<T>(recv: &mut mpsc::UnboundedReceiver<T>) -> Result<T, mpsc::error::TryRecvError> {
    recv.try_recv()
}

pub async fn recv<T>(recv: &mut mpsc::Receiver<T>) -> Option<T> {
    recv.recv().await
}