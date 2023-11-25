use std::{sync::Arc, time::Duration};

use bitvec::{prelude::Msb0, vec::BitVec};
use tokio::{
    sync::{mpsc, watch, Mutex},
    time,
};

use crate::{
    builder::file_builder,
    parser::{metadata::Metadata, trackerinfo::PeerInfo},
};

use super::{
    admin_message::{AdminMessage, PeerBitfield, PeerDisconnect, PieceDownload, PieceIndexRequest},
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
    endgame_mode: bool,
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
            endgame_mode: false,
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
                    match admin_message {
                        AdminMessage::PeerBitfield(req) => {
                            let _ = piece_strategy.update_bitfield(req.addr, req.peer_bitfield);

                            let _ = req.ack.send(());
                        },
                        AdminMessage::PieceIndexRequest(req) => {
                            let res = piece_strategy.get_piece_index(req.addr, self.endgame_mode);

                            let _ = req.chan.send(res);
                        },
                        AdminMessage::PieceDownload(req) => {
                            let index: usize = req.index.try_into().unwrap();

                            let mut prog = in_progress.lock().await;
                            let mut down = downloaded.lock().await;

                            prog[index] = false;
                            down[index] = true;

                            // Test if all pieces have been downloaded or are in-progress:
                            if !self.endgame_mode {
                                self.endgame_mode = prog.iter().zip(down.iter()).all(|(&a, &b)| a || b);
                            }
                        }
                        AdminMessage::PeerDisconnect(_req) => {
                            // TODO: update piece strategy when peer disconnects
                        },
                    }
                }
                _ = ui_refresh_interval.tick() => {
                    let _ = self.tx_progress_bar.send((
                        Self::count_ones(&downloaded.lock().await.to_vec()),
                        self.md.num_pieces().try_into().unwrap(),
                    ));

                    let _ = self.tx_in_progress.send(in_progress.lock().await.clone());
                    let _ = self.tx_downloaded.send(downloaded.lock().await.clone());

                    let speed = (self.download_history.average() / 0.1) * (self.md.info.piece_length as f32 / 1_000.0);

                    let _ = self.tx_speed.send(speed);
                }
                _ = download_speed_interval.tick() => {
                    self.download_history.push(Self::count_ones(&downloaded.lock().await.to_vec()));
                }
            }
        }
    }

    // TODO: move this to utils
    fn count_ones(v: &Vec<bool>) -> u32 {
        return v.iter().filter(|&&x| x).count().try_into().unwrap();
    }
}

pub(crate) async fn run_peer_manager_task(mut peer_manager: Manager) {
    peer_manager.run().await;
}

// TODO: move to separate file
struct RingBuffer {
    buf: Vec<u32>,
    capacity: usize,
}

impl RingBuffer {
    pub(crate) fn new(capacity: usize) -> Self {
        RingBuffer {
            buf: Vec::new(),
            capacity,
        }
    }

    pub(crate) fn push(&mut self, val: u32) {
        if self.buf.len() == self.capacity {
            self.buf.pop();
        }
        self.buf.insert(0, val);
    }

    pub(crate) fn average(&self) -> f32 {
        let len = self.buf.len();
        if len == 0 {
            return 0.0;
        }

        let diff_total: u32 = self.buf[..len - 1]
            .iter()
            .zip(self.buf[1..].iter())
            .map(|(x, y)| x - y)
            .sum();

        return (diff_total as f32) / (self.capacity as f32);
    }
}
