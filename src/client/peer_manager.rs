use std::{collections::VecDeque, sync::Arc, time::Duration};

use bitvec::{prelude::Msb0, vec::BitVec};
use tokio::{
    sync::{watch, Mutex},
    time,
};

use crate::{
    builder::file_builder,
    parser::{metadata::Metadata, trackerinfo::PeerInfo},
};

use super::peer::Peer;

pub(crate) type BitVecMutex = Arc<Mutex<BitVec<u8, Msb0>>>;

//TODO: initialise tx_* in constructor
pub(crate) struct PeerManager {
    md: Arc<Metadata>,
    peers: Vec<Peer>,
    output_dir: Arc<str>,
    client_pieces: BitVecMutex,
    download_history: RingBuffer,
    tx_progress: watch::Sender<(u32, u32)>,
    tx_pieces: watch::Sender<BitVec<u8, Msb0>>,
    tx_speed: watch::Sender<f32>,
}

impl PeerManager {
    // Start all threads on creation
    pub(crate) fn new(
        md: Arc<Metadata>,
        peers: Vec<PeerInfo>,
        output_dir: &str,
        tx_progress: watch::Sender<(u32, u32)>,
        tx_pieces: watch::Sender<BitVec<u8, Msb0>>,
        tx_speed: watch::Sender<f32>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client_pieces: BitVec<u8, Msb0> = file_builder::load_bitfield(&md, &output_dir)?;
        let client_pieces_ref = Arc::new(Mutex::new(client_pieces));

        let dir_ref: Arc<str> = Arc::from(output_dir);

        let peers = peers[..]
            .iter()
            .map(|peer| {
                Peer::new(
                    &peer.to_string(),
                    md.clone(),
                    dir_ref.clone(),
                    client_pieces_ref.clone(),
                )
            })
            .collect();

        let download_history = RingBuffer::new(15);

        Ok(PeerManager {
            md,
            peers,
            output_dir: dir_ref,
            client_pieces: client_pieces_ref,
            download_history,
            tx_progress,
            tx_pieces,
            tx_speed,
        })
    }

    async fn run(&mut self) {
        let mut ui_refresh_interval = time::interval(Duration::from_millis(60));
        let mut download_speed_interval = time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = ui_refresh_interval.tick() => {
                    let pieces = self.client_pieces.lock().await;
                    let _ = self.tx_progress.send((
                        pieces.count_ones().try_into().unwrap(),
                        pieces.len().try_into().unwrap(),
                    ));

                    let _ = self.tx_pieces.send(pieces.to_owned());

                    let speed = (self.download_history.average() / 0.1) * (self.md.info.piece_length as f32 / 1_000.0);

                    let _ = self.tx_speed.send(speed);
                }
                _ = download_speed_interval.tick() => {
                    let pieces = self.client_pieces.lock().await;
                    self.download_history.push(pieces.count_ones().try_into().unwrap());
                }
            }
        }
    }
}

pub(crate) async fn run_peer_manager_task(mut peer_manager: PeerManager) {
    peer_manager.run().await;
}

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
