use super::{bootstrap_info::BootstrapInfo, file_info::FileInfo};

pub(crate) enum TorrentType {
    File(BootstrapInfo, FileInfo),
    Magnet(BootstrapInfo)
}

pub(crate) fn get_torrent_info(path: String) -> Result<TorrentType, Box<dyn std::error::Error>> {
    if path.eq("magnet") {

    } else {
        let file_info = FileInfo::get_from_file(&path);
    }

    todo!()
}