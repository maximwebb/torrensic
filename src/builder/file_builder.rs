use std::{
    cmp::{max, min},
    fs::{self, File},
    io::{self, Seek, SeekFrom, Write},
    path::Path,
};

use bitvec::{prelude::Msb0, vec::BitVec};

use crate::parser::{fileinfo::FileInfo, metadata::Metadata};

pub(crate) fn create(md: &Metadata, dir: &String, overwrite: bool) -> io::Result<()> {
    let files: &Vec<FileInfo> = &md.info.files;
    let remove_dir = &format!("{}/{}", dir, &md.info.name);
    if Path::new(remove_dir).is_dir() {
        if overwrite {
            // println!("Removing existing files in {remove_dir}.");
            fs::remove_dir_all(remove_dir)?;
        } else {
            return Ok(());
        }
    }

    for file in files {
        let path_str = &format!("{}/{}/{}", dir, &md.info.name, &file.path.join("/"));
        let path = Path::new(path_str);
        let prefix = path.parent().unwrap();

        fs::create_dir_all(prefix)?;
        let mut f = fs::File::create(path)?;
        f.seek(SeekFrom::Start((file.length - 1).into())).unwrap();
        f.write_all(&[0]).unwrap();
    }

    // Create empty bitfield to track piece progress.
    let path = &format!("{}/{}/bitfield", dir, &md.info.name);
    let bitfield_size = (md.num_pieces() + 7) / 8;
    let mut f = fs::File::create(path)?;
    f.seek(SeekFrom::Start((bitfield_size - 1).try_into().unwrap()))
        .unwrap();
    f.write_all(&[0]).unwrap();

    Ok(())
}

//TODO: decide whether to remove begin parameter.
pub(crate) fn write(
    md: &Metadata,
    dir: &str,
    index: u32,
    begin: u32,
    data: &Vec<u8>,
) -> io::Result<()> {
    let start_pos: u32 = md.info.piece_length * index + begin;
    let end_pos: u32 = start_pos + u32::try_from(data.len()).unwrap();
    let mut cur_pos: u32 = 0;

    for file in &md.info.files {
        if cur_pos >= end_pos {
            break;
        }
        if cur_pos + file.length >= start_pos {
            let path_str = &format!("{}/{}/{}", dir, &md.info.name, &file.path.join("/"));
            let mut f = File::options().write(true).open(path_str)?;

            // Determines slice of data being written to file
            let start = max(start_pos, cur_pos) - start_pos;
            let end = min(end_pos, cur_pos + file.length) - start_pos;

            // If performing the first write, move cursor to required position
            if cur_pos < start_pos {
                f.seek(SeekFrom::Start((start_pos - cur_pos).into()))?;
            }
            f.write_all(&data[start as usize..end as usize])?;
        }
        cur_pos += file.length;
    }

    // Update bitfield
    let mut bitfield = load_bitfield(md, dir)?;
    bitfield.set(index.try_into().unwrap(), true);
    let path = &format!("{}/{}/bitfield", dir, &md.info.name);
    let mut f = File::options().write(true).open(path)?;
    f.seek(SeekFrom::Start(0))?;
    f.write_all(bitfield.as_raw_slice())?;

    Ok(())
}

pub(crate) fn load_bitfield(md: &Metadata, dir: &str) -> io::Result<BitVec<u8, Msb0>> {
    let path_str = &format!("{}/{}/bitfield", dir, &md.info.name);
    let raw = match fs::read(path_str) {
        Ok(file) => file,
        Err(e) => {
            println!("Error reading bitfield.");
            return Err(e);
        }
    };

    let mut bitfield: BitVec<u8, Msb0> = BitVec::from_iter(raw.iter());
    bitfield.truncate(md.num_pieces());
    Ok(bitfield)
}
