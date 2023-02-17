
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Error{
    Unknown,
    UnexpectedEof,
    Interrupted,
    Unsupported,
    InvalidInput,
    InvalidData,
    NotFound,
}

impl core::fmt::Display for Error{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result{
        match self{
            Error::Unknown => f.write_str("Unknown Error"),
            Error::UnexpectedEof => f.write_str("Unexpected Eof"),
            Error::Interrupted => f.write_str("Interrupted"),
            Error::Unsupported => f.write_str("Unsupported Operation"),
            Error::InvalidInput => f.write_str("Invalid Input"),
            Error::InvalidData => f.write_str("Invalid Data"),
            Error::NotFound => f.write_str("Object or stream not found"),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error{
    fn from(err: std::io::Error) -> Self{
        match err.kind(){
            std::io::ErrorKind::Interrupted => Error::Interrupted,
            std::io::ErrorKind::UnexpectedEof => Error::UnexpectedEof,
            std::io::ErrorKind::Unsupported => Error::Unsupported,
            std::io::ErrorKind::InvalidInput => Error::InvalidInput,
            std::io::ErrorKind::InvalidData => Error::InvalidData,
            std::io::ErrorKind::NotFound => Error::NotFound,
            _ => Error::Unknown,
        }
    }
}


pub type Result<T> = core::result::Result<T,Error>;

pub trait Read{
    fn read(&mut self, out: &mut [u8]) -> Result<usize>;
    fn read_fully(&mut self, mut out: &mut [u8]) -> Result<()>{
        loop{
            match self.read(out){
                Ok(0) => break Err(Error::Interrupted),
                Ok(n) => {
                    out = &mut out[n..];
                    if out.is_empty(){
                        break Ok(())
                    }
                }
                Err(Error::Interrupted) => continue,
                Err(e) => break Err(e)
            }
        }
    }
}

impl<R: Read + ?Sized> Read for &mut R{
    fn read(&mut self, out: &mut [u8]) -> Result<usize>{
        <R as Read>::read(self,out)
    }

    fn read_fully(&mut self, out: &mut [u8]) -> Result<()>{
        <R as Read>::read_fully(self,out)
    }
}

impl Read for &mut [u8]{
    fn read(&mut self, out: &mut [u8]) -> Result<usize>{
        let mlen = self.len().min(out.len());
        let base = unsafe{core::ptr::read(self)};
        let copy;
        
        (copy,*self) = base.split_at_mut(mlen);

        out[..mlen].copy_from_slice(copy);

        Ok(mlen)
    }
}


#[cfg(feature = "std")]
impl Read for std::fs::File{
    fn read(&mut self, out: &mut [u8]) -> Result<usize> {
        <std::fs::File as std::io::Read>::read(self, out)
            .map_err(Error::from)
    }
}

#[cfg(feature = "std")]
impl Read for &std::fs::File{
    fn read(&mut self, out: &mut [u8]) -> Result<usize> {
        <&std::fs::File as std::io::Read>::read(self, out)
            .map_err(Error::from)
    }
}

#[derive(Copy,Clone,Debug,Hash,PartialEq)]
pub enum SeekPos{
    Start(u64),
    StartSector(u128),
    AbsPos(VolLocation),
    Curr(i64),
    End(i64),
    EndSector(i128)
}

#[derive(Copy,Clone,Debug,Hash,PartialEq)]
pub struct VolLocation{
    pub sector: u128,
    pub offset: u64,
}


pub trait Seek{
    fn seek(&mut self, pos: SeekPos) -> Result<VolLocation>;

    fn stream_position(&mut self) -> Result<VolLocation>{
        self.seek(SeekPos::Curr(0))
    }

    fn stream_length(&mut self) -> Result<VolLocation>{
        let curpos = self.seek(SeekPos::Curr(0))?;
        let len = self.seek(SeekPos::End(0))?;
        self.seek(SeekPos::AbsPos(curpos))?;
        Ok(len)
    }
}

impl<S: Seek + ?Sized> Seek for &mut S{
    fn seek(&mut self, pos: SeekPos) -> Result<VolLocation> {
        <S as Seek>::seek(self, pos)
    }
}

#[cfg(feature = "std")]
impl Seek for std::fs::File{
    fn seek(&mut self, pos: SeekPos) -> Result<VolLocation> {
        let rpos = match pos{
            SeekPos::Start(n) => std::io::SeekFrom::Start(n),
            SeekPos::Curr(n) => std::io::SeekFrom::Current(n),
            SeekPos::End(n) => std::io::SeekFrom::End(n),
            SeekPos::StartSector(n) => {
                let n = n.checked_shl(10).ok_or(Error::Unsupported)?;
                std::io::SeekFrom::Start(n.try_into().map_err(|_| Error::Unsupported)?)
            },
            SeekPos::EndSector(n) => {
                let n = n.checked_mul(1024).ok_or(Error::Unsupported)?;
                std::io::SeekFrom::End(n.try_into().map_err(|_| Error::Unsupported)?)
            },
            SeekPos::AbsPos(pos) => {
                if pos.offset>=1024{
                    return Err(Error::InvalidInput);
                }
                let n: u128 = pos.sector.checked_shl(10).ok_or(Error::Unsupported)? | (pos.offset as u128);
                std::io::SeekFrom::Start(n.try_into().map_err(|_| Error::Unsupported)?)
            },
        };
        <_ as std::io::Seek>::seek(self, rpos).map_err(Error::from).map(|pos|{
            let sector = (pos>>10).into();
            let offset = pos&1023;
            VolLocation { sector, offset }
        })
    }
}

#[cfg(feature = "std")]
impl Seek for &std::fs::File{
    fn seek(&mut self, pos: SeekPos) -> Result<VolLocation> {
        let rpos = match pos{
            SeekPos::Start(n) => std::io::SeekFrom::Start(n),
            SeekPos::Curr(n) => std::io::SeekFrom::Current(n),
            SeekPos::End(n) => std::io::SeekFrom::End(n),
            SeekPos::StartSector(n) => {
                let n = n.checked_shl(10).ok_or(Error::Unsupported)?;
                std::io::SeekFrom::Start(n.try_into().map_err(|_| Error::Unsupported)?)
            },
            SeekPos::EndSector(n) => {
                let n = n.checked_mul(1024).ok_or(Error::Unsupported)?;
                std::io::SeekFrom::End(n.try_into().map_err(|_| Error::Unsupported)?)
            },
            SeekPos::AbsPos(pos) => {
                if pos.offset>=1024{
                    return Err(Error::InvalidInput);
                }
                let n: u128 = pos.sector.checked_shl(10).ok_or(Error::Unsupported)? | (pos.offset as u128);
                std::io::SeekFrom::Start(n.try_into().map_err(|_| Error::Unsupported)?)
            },
        };
        <_ as std::io::Seek>::seek(self, rpos).map_err(Error::from).map(|pos|{
            let sector = (pos>>10).into();
            let offset = pos&1023;
            VolLocation { sector, offset }
        })
    }
}


pub trait Write{
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()>{
        while !buf.is_empty(){
            match self.write(buf){
                Ok(0) => return Err(Error::UnexpectedEof),
                Ok(n) => {
                    buf = &buf[n..];
                }
                Err(Error::Interrupted) => continue,
                Err(e) => return Err(e)
            }
        }
        Ok(())
    }

    fn write_zeroes(&mut self, mut num: usize) -> Result<()>{
        let buf = [0u8;1024];

        while num > 0{
            match self.write(&buf[..(num.min(1024))]){
                Ok(0) => return Err(Error::UnexpectedEof),
                Ok(n) => {
                    num -= n;
                }
                Err(Error::Interrupted) => continue,
                Err(e) => return Err(e)
            }
        }
        Ok(())
    }
}

