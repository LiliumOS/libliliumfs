
pub const fn extend_str<const N: usize>(s: &str) -> [u8;N]{
    let mut val = [0u8;N];

    if s.len()>N{
        panic!("Cannot extend a string to larger length")
    }

    let mut i = 0;

    while i<N{
        val[i] = s.as_bytes()[i];
        i+=1;
    }
    val
}