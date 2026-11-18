//! A small dedicated pool keeps blocking decoders and thumbnailer processes
//! away from GPUI's executor and render thread.
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use futures_channel::oneshot;
use super::cache::Source;

type Job = (Source, oneshot::Sender<Option<std::path::PathBuf>>);

fn sender() -> &'static mpsc::Sender<Job> {
    static SENDER: OnceLock<mpsc::Sender<Job>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        for index in 0..2 {
            let receiver = receiver.clone();
            std::thread::Builder::new().name(format!("veyra-thumbnail-{index}"))
                .spawn(move || loop {
                    let job = receiver.lock().unwrap().recv();
                    let Ok((source, reply)) = job else { break; };
                    // A request may disappear when navigating away. Skip work
                    // that no view is waiting for anymore.
                    if !reply.is_canceled() {
                        let _ = reply.send(source.generate());
                    }
                }).expect("failed to start thumbnail worker");
        }
        sender
    })
}

pub async fn generate(source: Source) -> Option<std::path::PathBuf> {
    let (reply, result) = oneshot::channel();
    sender().send((source, reply)).ok()?;
    result.await.ok().flatten()
}
