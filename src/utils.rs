pub mod ring_buffer;


pub(crate) fn count_ones(v: &Vec<bool>) -> u32 {
    return v.iter().filter(|&&x| x).count().try_into().unwrap();
}