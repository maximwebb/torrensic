use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

pub(crate) struct PieceStrategy {
    peer_bitfield_map: HashMap<String, Vec<bool>>,
    num_pieces: usize,
    piece_multiplicities: Vec<u32>,
    in_progress: Arc<Mutex<Vec<bool>>>,
    downloaded: Arc<Mutex<Vec<bool>>>,
    endgame_mode: bool
}

impl PieceStrategy {
    pub fn new(
        num_pieces: usize,
        in_progress: Arc<Mutex<Vec<bool>>>,
        downloaded: Arc<Mutex<Vec<bool>>>,
    ) -> Self {
        return PieceStrategy {
            peer_bitfield_map: HashMap::new(),
            num_pieces,
            piece_multiplicities: vec![0; num_pieces],
            in_progress,
            downloaded,
            endgame_mode: false,
        };
    }

    pub fn update_bitfield(&mut self, addr: Arc<str>, bitfield: Vec<bool>) -> Result<(), ()> {
        if bitfield.len() != self.num_pieces.try_into().unwrap() {
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
    pub fn get_piece_index(&mut self, addr: Arc<str>, endgame_mode: bool) -> Option<u32> {

        let peer_bitfield = self
            .peer_bitfield_map
            .get(&*addr).expect("Invalid peer address");

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
                if !b1 || (!endgame_mode && b2) || b3 {
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
            return Some(i.try_into().unwrap())
        }
        else {
            return None
        }
    }

    // TODO: check this function
    fn update_multiplicities(&mut self) {
        self.piece_multiplicities = self.peer_bitfield_map.values().fold(vec![0; self.num_pieces], |acc, v| {
            return acc
                .iter()
                .zip(v.iter())
                .map(|(a, b)| a + if *b { 1 } else { 0 })
                .collect();
        })
    }
}
