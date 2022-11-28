
#[non_exhaustive]
pub enum Error{
    Unknown,
    UnexpectedEof,
    Interrupted
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

impl<R: Read> Read for &mut R{
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
    
}

pub enum SeekPos{
    Start(u64),
    StartSector(u128),
    Curr(i64),
    End(i64),
    EndSector(i128)
}

