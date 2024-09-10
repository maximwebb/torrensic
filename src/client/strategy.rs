use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use super::admin_message::AdminMessage;

pub(crate) struct Strategy {
    peer_bitfield_map: HashMap<String, Vec<bool>>,
    num_pieces: usize,
    piece_multiplicities: Vec<u32>,
    in_progress: Arc<Mutex<Vec<bool>>>,
    downloaded: Arc<Mutex<Vec<bool>>>,
    endgame_mode: bool,
}

impl Strategy {
    pub fn new(
        num_pieces: usize,
        in_progress: Arc<Mutex<Vec<bool>>>,
        downloaded: Arc<Mutex<Vec<bool>>>,
    ) -> Self {
        return Strategy {
            peer_bitfield_map: HashMap::new(),
            num_pieces,
            piece_multiplicities: vec![0; num_pieces],
            in_progress,
            downloaded,
            endgame_mode: false,
        };
    }

    pub async fn handle_message(&mut self, admin_message: AdminMessage) -> Result<(), ()> {
        match admin_message {
            AdminMessage::PeerBitfield(req) => {
                let _ = self.update_bitfield(req.addr, req.peer_bitfield)?;
                let _ = req.ack.send(());
            }
            AdminMessage::PieceIndexRequest(req) => {
                let res = self.get_piece_index(req.addr);
                let _ = req.chan.send(res);
            }
            AdminMessage::PieceDownload(req) => {
                let index: usize = req.index.try_into().unwrap();

                let mut prog = self.in_progress.lock().await;
                let mut down = self.downloaded.lock().await;

                prog[index] = false;
                down[index] = true;

                // Test if all pieces have been downloaded or are in-progress:
                if !self.endgame_mode {
                    self.endgame_mode = prog.iter().zip(down.iter()).all(|(&a, &b)| a || b);
                }
            }
            AdminMessage::PeerDisconnect(_req) => {
                //println!("{0} disconnected", req.addr);
            }
        }
        return Ok(());
    }

    pub fn update_bitfield(&mut self, addr: Arc<str>, bitfield: Vec<bool>) -> Result<(), ()> {
        if bitfield.len() != self.num_pieces {
            return Err(());
        }

        if let Some(v) = self.peer_bitfield_map.get_mut(&*addr) {
            v.iter_mut()
                .zip(bitfield.iter())
                .for_each(|(x, &y)| *x = *x | y);
        } else {
            self.peer_bitfield_map.insert(addr.to_string(), bitfield);
        }

        self.update_multiplicities();

        Ok(())
    }

    /*
       Find the piece index satisfying the following criteria, if it exists:
       - Owned by the relevant peer
       - Not already downloaded
       - If not in endgame mode, not currently in progress
       - Owned by the fewest number of other peers
    */
    pub fn get_piece_index(&mut self, addr: Arc<str>) -> Option<u32> {
        // return None;
        let peer_bitfield = self
            .peer_bitfield_map
            .get(&*addr)
            .expect("Invalid peer address");

        let mut in_progress = self.in_progress.try_lock().expect("Error acquiring mutex");
        let downloaded = self.downloaded.try_lock().expect("Error acquiring mutex");

        let res = self
            .piece_multiplicities
            .iter()
            .enumerate()
            .zip(peer_bitfield.iter())
            .zip(in_progress.iter())
            .zip(downloaded.iter())
            .map(|((((i, n), &b1), &b2), &b3)| (i, n, b1, b2, b3))
            .fold(None, |acc: Option<(usize, u32)>, (i, n, b1, b2, b3)| {
                if !b1 || (!self.endgame_mode && b2) || b3 {
                    acc
                } else {
                    acc.map_or_else(
                        || Some((i, *n)),
                        |(j, m)| {
                            if n < &m {
                                Some((i, *n))
                            } else {
                                Some((j, m))
                            }
                        },
                    )
                }
            });

        if let Some((i, _)) = res {
            in_progress[i] = true;
            return Some(i.try_into().unwrap());
        } else {
            return None;
        }
    }

    // TODO: check this function
    fn update_multiplicities(&mut self) {
        self.piece_multiplicities =
            self.peer_bitfield_map
                .values()
                .fold(vec![0; self.num_pieces], |acc, v| {
                    return acc
                        .iter()
                        .zip(v.iter())
                        .map(|(a, b)| a + if *b { 1 } else { 0 })
                        .collect();
                })
    }
}
