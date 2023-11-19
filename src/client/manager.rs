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
    peer_handler::{PeerBitfield, PeerDisconnect, PeerHandler, PieceDownload, PieceIndexRequest},
    piece_strategy::PieceStrategy,
};

pub(crate) type BitVecMutex = Arc<Mutex<BitVec<u8, Msb0>>>;

/* TODO for next time:
    - The in-progress stats seem to be messing with the downloaded ones
    - The UI might be overlaying - though is unlikely, as downloaded: x/y is also wrong
    - Test if pieces are actually being downloaded - might be a bug in peer handler?
    - Inspect downloaded_pieces and in_progress_pieces as vectors.

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
    rx_peer_bitfield: mpsc::Receiver<PeerBitfield>,
    rx_piece_index_req: mpsc::Receiver<PieceIndexRequest>,
    rx_piece_download: mpsc::Receiver<PieceDownload>,
    rx_peer_disconnect: mpsc::Receiver<PeerDisconnect>,
}

impl Manager {
    // All peer handler threads are started on creation
    pub(crate) fn new(
        md: Arc<Metadata>,
        peers: Vec<PeerInfo>,
        output_dir: &str,
        tx_progress_bar: watch::Sender<(u32, u32)>,
        tx_in_progress: watch::Sender<Vec<bool>>,
        tx_downloaded: watch::Sender<Vec<bool>>,
        tx_speed: watch::Sender<f32>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: do these need to be shared with peer handlers?
        let client_pieces: BitVec<u8, Msb0> = file_builder::load_bitfield(&md, &output_dir)?;
        let client_pieces_ref = Arc::new(Mutex::new(client_pieces));

        // Peer handler channels
        let (tx_peer_bitfield, rx_peer_bitfield) = mpsc::channel(32);
        let (tx_piece_index_req, rx_piece_index_req) = mpsc::channel(128);
        let (tx_piece_download, rx_piece_download) = mpsc::channel(8);
        let (tx_peer_disconnect, rx_peer_disconnect) = mpsc::channel(8);

        let dir_ref: Arc<str> = Arc::from(output_dir);

        for peer in peers {
            PeerHandler::init(
                md.clone(),
                &peer.to_string(),
                dir_ref.clone(),
                client_pieces_ref.clone(),
                tx_peer_bitfield.clone(),
                tx_piece_index_req.clone(),
                tx_piece_download.clone(),
                tx_peer_disconnect.clone(),
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
            rx_peer_bitfield,
            rx_piece_index_req,
            rx_piece_download,
            rx_peer_disconnect,
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
                peer_bitfield = self.rx_peer_bitfield.recv() => {
                    let req = peer_bitfield.expect("Error receiving peer bitfield");

                    let _ = piece_strategy.update_bitfield(req.addr, req.peer_bitfield);

                    let _ = req.ack.send(());
                }
                piece_index_req = self.rx_piece_index_req.recv() => {
                    let req = piece_index_req.expect("Error receiving peer piece request");

                    let res = piece_strategy.get_piece_index(req.addr, self.endgame_mode);

                    // println!("Requesting {}", res.unwrap_or(6969));

                    let _ = req.chan.send(res);
                }
                piece_download = self.rx_piece_download.recv() => {
                    let index = piece_download.expect("Error receiving piece download update").index;
                    let index: usize = index.try_into().unwrap();

                    let mut prog = in_progress.lock().await;
                    let mut down = downloaded.lock().await;

                    prog[index] = false;
                    down[index] = true;

                    // Test if all pieces have been downloaded or are in-progress:
                    if !self.endgame_mode {
                        self.endgame_mode = prog.iter().zip(down.iter()).all(|(&a, &b)| a || b);
                        if self.endgame_mode {
                            println!("Endgame mode enabled");
                        }
                    }
                }
                peer_disconnect = self.rx_peer_disconnect.recv() => {
                    let payload = peer_disconnect.expect("Error: Peer disconnected too quickly");
                    println!("Peer disconnected (addr: {})", payload.addr);
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

    // fn respond_piece_index_req(
    //     req: PieceIndexRequest,
    //     in_progress: &BitVec<u8, Msb0>,
    //     downloaded: &BitVec<u8, Msb0>,
    // ) -> Option<u32> {
    //     {
    //         let (chan, peer_bitfield) = (req.chan, req.peer_bitfield);

    //         let valid_pieces = !(!peer_bitfield | in_progress | downloaded);
    //         let res = valid_pieces.first_one().map(|v| v as u32);

    //         match chan.send(res) {
    //             Ok(_) => {}
    //             Err(_v) => {
    //                 println!("PM send error");
    //             }
    //         }

    //         return res;
    //     }
    // }

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
