use std::{sync::Arc, time::Duration};

use bitvec::{prelude::Msb0, vec::BitVec};
use tokio::{
    sync::{mpsc, watch, Mutex},
    time,
};

use crate::{
    builder::file_builder,
    parser::{metadata::Metadata, tracker_info::PeerInfo},
    utils::{self, ring_buffer::RingBuffer}
};

use super::{
    admin_message::AdminMessage,
    peer_handler::PeerHandler,
    piece_strategy::PieceStrategy,
};

pub(crate) type BitVecMutex = Arc<Mutex<BitVec<u8, Msb0>>>;

/* TODO for next time:
    - Combine peer handler / manager comms into one channel
    - Start looking at magnet links
*/
pub(crate) struct Manager {
    md: Arc<Metadata>,
    download_history: RingBuffer,
    // TODO: distinguish UI from peer handler channels
    tx_progress_bar: watch::Sender<(u32, u32)>,
    tx_in_progress: watch::Sender<Vec<bool>>,
    tx_downloaded: watch::Sender<Vec<bool>>,
    tx_speed: watch::Sender<f32>,
    rx_admin_message: mpsc::Receiver<AdminMessage>,
}

impl Manager {
    // All peer handler threads are started on creation
    pub(crate) fn new(
        md: Arc<Metadata>,
        peers: Arc<Vec<PeerInfo>>,
        output_dir: &str,
        tx_progress_bar: watch::Sender<(u32, u32)>,
        tx_in_progress: watch::Sender<Vec<bool>>,
        tx_downloaded: watch::Sender<Vec<bool>>,
        tx_speed: watch::Sender<f32>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: do these need to be shared with peer handlers?
        let client_pieces: BitVec<u8, Msb0> = file_builder::load_bitfield(&md, &output_dir)?;
        let client_pieces_ref = Arc::new(Mutex::new(client_pieces));

        // Peer handler channel
        let (tx_admin_message, rx_admin_message) = mpsc::channel(128);

        let dir_ref: Arc<str> = Arc::from(output_dir);

        for peer in peers.iter() {
            PeerHandler::init(
                md.clone(),
                &peer.to_string(),
                dir_ref.clone(),
                client_pieces_ref.clone(),
                tx_admin_message.clone(),
            )
        }

        Ok(Manager {
            md,
            download_history: RingBuffer::new(15),
            tx_progress_bar,
            tx_in_progress,
            tx_downloaded,
            tx_speed,
            rx_admin_message,
        })
    }

    async fn run(&mut self) {
        let mut ui_refresh_interval = time::interval(Duration::from_millis(60));
        let mut download_speed_interval = time::interval(Duration::from_millis(100));

        let in_progress = Arc::new(Mutex::new(vec![false; self.md.num_pieces()]));
        let downloaded = Arc::new(Mutex::new(vec![false; self.md.num_pieces()]));

        let mut piece_strategy = PieceStrategy::new(
            self.md.num_pieces(),
            Arc::clone(&in_progress),
            Arc::clone(&downloaded),
        );

        loop {
            tokio::select! {
                admin_message = self.rx_admin_message.recv() => {
                    let admin_message = admin_message.expect("Error receiving message");
                    let _ = piece_strategy.handle_message(admin_message).await;
                }
                _ = ui_refresh_interval.tick() => {
                    let _ = self.tx_progress_bar.send((
                        utils::count_ones(&downloaded.lock().await.to_vec()),
                        self.md.num_pieces().try_into().unwrap(),
                    ));

                    let _ = self.tx_in_progress.send(in_progress.lock().await.clone());
                    let _ = self.tx_downloaded.send(downloaded.lock().await.clone());

                    let speed = (self.download_history.average() / 0.1) * (self.md.info.piece_length as f32 / 1_000.0);

                    let _ = self.tx_speed.send(speed);
                }
                _ = download_speed_interval.tick() => {
                    self.download_history.push(utils::count_ones(&downloaded.lock().await.to_vec()));
                }
            }
        }
    }

}

pub(crate) async fn run_peer_manager_task(mut peer_manager: Manager) {
    peer_manager.run().await;
}

