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

use super::peer_handler::{PieceIndexRequest, PeerDisconnect, PeerHandler};


pub(crate) type BitVecMutex = Arc<Mutex<BitVec<u8, Msb0>>>;


pub(crate) struct Manager {
    md: Arc<Metadata>,
    downloaded_pieces: BitVecMutex,
    download_history: RingBuffer,
    tx_progress: watch::Sender<(u32, u32)>,
    tx_pieces: watch::Sender<BitVec<u8, Msb0>>,
    tx_speed: watch::Sender<f32>,
    rx_piece_index_req: mpsc::Receiver<PieceIndexRequest>,
    rx_peer_disconnect: mpsc::Receiver<PeerDisconnect>,
}

impl Manager {
    // All peer handler threads are started on creation
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

        let (tx_piece_index_req, rx_piece_index_req) = mpsc::channel(8);
        let (tx_peer_disconnect, rx_peer_disconnect) = mpsc::channel(8);

        let dir_ref: Arc<str> = Arc::from(output_dir);

        for peer in peers {
            PeerHandler::init(
                md.clone(),
                &peer.to_string(),
                dir_ref.clone(),
                client_pieces_ref.clone(),
                tx_piece_index_req.clone(),
                tx_peer_disconnect.clone(),
            )
        }

        let download_history = RingBuffer::new(15);

        Ok(Manager {
            md,
            downloaded_pieces: client_pieces_ref,
            download_history,
            tx_progress,
            tx_pieces,
            tx_speed,
            rx_piece_index_req,
            rx_peer_disconnect,
        })
    }

    async fn run(&mut self) {
        let mut ui_refresh_interval = time::interval(Duration::from_millis(60));
        let mut download_speed_interval = time::interval(Duration::from_millis(100));

        let mut in_progress_pieces = BitVec::<u8, Msb0>::repeat(false, self.md.num_pieces());

        loop {
            tokio::select! {
                piece_index_req = self.rx_piece_index_req.recv() => {
                    let req = piece_index_req.expect("Error receiving peer piece request");

                    {
                        // let downloaded_pieces = self.client_pieces.lock().await;
                        let res = Self::respond_piece_index_req(req, &in_progress_pieces);

                        if let Some(index) = res {
                            in_progress_pieces.set(index.try_into().unwrap(), true);
                        }
                    }
                }
                peer_disconnect = self.rx_peer_disconnect.recv() => {
                    let payload = peer_disconnect.expect("Error: Peer disconnected too quickly");
                    println!("Peer disconnected (addr: {})", payload.addr);
                }
                _ = ui_refresh_interval.tick() => {
                    // TODO: change from in progress to downloaded
                    // let pieces = self.client_pieces.lock().await;
                    let _ = self.tx_progress.send((
                        in_progress_pieces.count_ones().try_into().unwrap(),
                        in_progress_pieces.len().try_into().unwrap(),
                    ));

                    let _ = self.tx_pieces.send(in_progress_pieces.clone());

                    let speed = (self.download_history.average() / 0.1) * (self.md.info.piece_length as f32 / 1_000.0);

                    let _ = self.tx_speed.send(speed);
                }
                _ = download_speed_interval.tick() => {
                    // TODO: Again, switch from in progress to downloaded.
                    // let pieces = self.client_pieces.lock().await;
                    self.download_history.push(in_progress_pieces.count_ones().try_into().unwrap());
                }
            }
        }
    }


    // TODO add downloaded_pieces bitfield back
    fn respond_piece_index_req(
        req: PieceIndexRequest,
        in_progress: &BitVec<u8, Msb0>
    ) -> Option<u32> {
        {
            let (chan, peer_bitfield) = (req.chan, req.peer_bitfield);

            let valid_pieces = ! (!peer_bitfield | in_progress);
            let res = valid_pieces.first_one().map(|v| v as u32);

            match chan.send(res) {
                Ok(_) => {}
                Err(_v) => {
                    println!("PM send error");
                }
            }
            
            return res;
        }
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


// async fn get_piece_index(&self) -> Option<u32> {
//     let (tx, rx) = oneshot::channel();
//     let r = self.tx_piece_index_req.send(Message{respond_to: tx}).await;

//     match rx.await {
//         Ok(v) => {
//             return v
//         },
//         Err(e) => {
//             return None
//         },
//     }
// }