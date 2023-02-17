use core::{num::NonZeroU64, mem::size_of, cmp::Ordering};

use bytemuck::Zeroable;
use nonzero_ext::nonzero;

use crate::{object::{RootDescriptor,consts, PhantomFSMagic, FSRequiredFeatures, FSOptionalFeatures, ObjectId, Object, SectorPos, AbsPos, ObjectType, ObjectFlags, StreamListing, StreamFlags, VolumeSpan, StreamId, DirectoryElement}, io::{Read, Seek, Write, SeekPos}, uuid::Uuid};
use crate::helpers::extend_str;

pub struct FilesystemAccess<S>{
    stream: S,
    root_desc: Option<RootDescriptor>,
    label: Option<String>
}

impl<S> FilesystemAccess<S>{
    pub const fn new(stream: S) -> Self{
        Self { stream, root_desc: None, label: None }
    }
}

impl<S: Write + Seek> FilesystemAccess<S>{
    pub fn sync(&mut self) -> crate::io::Result<()>{
        let pos = self.stream.stream_position()?;
        if let Some(desc) = self.root_desc.as_ref(){
            self.stream.seek(crate::io::SeekPos::StartSector(1))?;
            self.stream.write_all(bytemuck::bytes_of(desc))?;
        }
        self.root_desc = None;
        self.label = None;
        Ok(())
    }
}

impl<S: Read + Write + Seek> FilesystemAccess<S>{
    pub fn create_object(&mut self, init_size: u64, ty: ObjectType, init_string_tab: &str,owner_uuid: Uuid) -> crate::io::Result<ObjectId>{
        let desc = self.get_or_read_descriptor()?;

        let objtab_pos = desc.objtab_end;
        let objtab_size = desc.objtab_size;

        self.stream.seek(SeekPos::StartSector(objtab_pos.0))?;

        for i in 1..=(objtab_size/(size_of::<Object>() as u64)){
            self.stream.seek(SeekPos::Curr(-(size_of::<Object>() as i64)))?;
            let mut obj = Object{..Zeroable::zeroed()};
            self.stream.read_fully(bytemuck::bytes_of_mut(&mut obj))?;
            self.stream.seek(SeekPos::Curr(-(size_of::<Object>() as i64)))?;

            if obj.weak_ref==0{
                // new object
                obj.weak_ref = 1;
                obj.strong_ref = 1;
                
                let mut streams: [StreamListing;16] = Zeroable::zeroed();

                [consts::STREAMS_STREAM, consts::STRINGS_STREAM, consts::SECURITYDESCRIPTOR_STREAM]
                .into_iter()
                .zip(&mut streams)
                .for_each(|(stream, slot)| *slot = StreamListing{name: extend_str(stream), name_ref: None, flags: StreamFlags::REQUIRED, ..Zeroable::zeroed()});

                let pos = self.stream.stream_position()?;
                let streams_base = self.allocate_contiguous_space(2048)?;

                streams[0].content_ref = streams_base.0;
                streams[0].size = 2048;
                streams[0].flags |= StreamFlags::indirection(1);

                obj.streams_indirection = 1;
                obj.streams_ref = streams_base.0;
                obj.streams_size = 2048;

                self.stream.seek(SeekPos::StartSector(streams_base.0))?;
                // safety: This is known to never overflow, and i starts at `1`
                return Ok(ObjectId(unsafe{NonZeroU64::new_unchecked(i)}))
            }
        }

        todo!("grow object table");
    }

    pub fn allocate_contiguous_space(&mut self, size: u64) -> crate::io::Result<SectorPos>{
        todo!("allocate_contiguous_space")
    }
    pub fn create_filesystem(&mut self, label: &str, id: Uuid, volsize: u128) -> crate::io::Result<()>{
        let mut desc = RootDescriptor{
            magic: PhantomFSMagic::MAGIC,
            version_major: consts::VERSION_MAJOR,
            version_minor: consts::VERSION_MAJOR,
            required_features: FSRequiredFeatures::empty(),
            optional_features: FSOptionalFeatures::empty(),
            volume_id_hi: id.hi,
            volume_id_lo: id.lo,
            root_object_id: Some(ObjectId(nonzero!(1u64))),
            objtab_end: SectorPos(volsize),
            objtab_size: 1024,
            alloc_tab_begin: AbsPos(2048),
            alloc_tab_size: 1024, // for now
            label_ref: None,
            label: Zeroable::zeroed(),
            header_size: core::mem::size_of::<RootDescriptor>() as u32,
            crc: 0,
        };


        self.stream.seek(crate::io::SeekPos::StartSector(volsize))?;
        self.stream.seek(crate::io::SeekPos::Curr(-1024))?;

        self.stream.write_zeroes(1024)?;

        self.stream.seek(crate::io::SeekPos::StartSector(2))?;
        let mut init_reserve = VolumeSpan{
            base_sector: 0,
            extent: 8,
            ..Zeroable::zeroed()
        };
        self.stream.write_all(bytemuck::bytes_of(&init_reserve))?;
        self.stream.write_zeroes(1024-32)?;
        
        
        self.root_desc = Some(desc);
        

        Ok(())
    }
}

impl<S: Read + Seek> FilesystemAccess<S>{
    pub fn get_or_read_descriptor(&mut self) -> crate::io::Result<&mut RootDescriptor>{
        if let Some(desc) = self.root_desc.as_mut(){
            Ok(unsafe{&mut *(desc as *mut RootDescriptor)}) // Hecking NLL get_or_insert_with
        }else{

            let mut root_desc: RootDescriptor = Zeroable::zeroed();

            let pos = self.stream.stream_position()?;

            self.stream.seek(crate::io::SeekPos::StartSector(1))?;

            self.stream.read_fully(bytemuck::bytes_of_mut(&mut root_desc))?;


            if root_desc.magic!=PhantomFSMagic::MAGIC{
                return Err(crate::io::Error::InvalidData);
            }

            if root_desc.version_major!=consts::VERSION_MAJOR{
                return Err(crate::io::Error::InvalidData);
            }

            if root_desc.header_size<(size_of::<RootDescriptor>() as u32){
                return Err(crate::io::Error::InvalidData);
            }

            if root_desc.crc != crc::Crc::<u32>::new(&crc::CRC_32_CKSUM).checksum(bytemuck::bytes_of(&root_desc)){
                return Err(crate::io::Error::InvalidData);
            }

            self.root_desc = Some(root_desc);

            Ok(self.root_desc.as_mut().unwrap())
        }
    }


    pub fn get_obj_by_id(&mut self, id: ObjectId) -> crate::io::Result<Object>{
        let desc = self.get_or_read_descriptor()?;
        let objtab_end = desc.objtab_end;
        let objtabsize = desc.objtab_size;

        let pos = id.0.get()*(size_of::<Object>() as u64);

        if pos>objtabsize{
            return Err(crate::io::Error::NotFound);
        }

        self.stream.seek(SeekPos::StartSector(objtab_end.0))?;
        self.stream.seek(SeekPos::Curr(-(pos as i64)))?;

        let mut obj: Object = Zeroable::zeroed();

        self.stream.read_fully(bytemuck::bytes_of_mut(&mut obj))?;

        if obj.weak_ref==0{
            // This is an invalid object, so for all intents and purposes, it does not exist
            return Err(crate::io::Error::NotFound)
        }

        Ok(obj)
    }

    fn read_by_indirection(&mut self, offset: u64,buf: &mut [u8], baseref: u128, indirection: u8,len: u64) -> crate::io::Result<usize>{
        if indirection==1{
            self.stream.seek(SeekPos::StartSector(baseref))?;
            self.stream.seek(SeekPos::Curr(offset as i64))?;
            self.stream.read(buf)
        }else{
            let indirection = indirection as usize;
            let mut stack: [VolumeSpan;16] = [Zeroable::zeroed();16];
            stack[0] = VolumeSpan{base_sector: baseref, extent: !0, __reserved:0};
            let mut stackpos = 1;
            let mut cursizesector = 0u64;
            let mut offset_at_level = 0u64;

            loop{
                if cursizesector*1024 > len{
                    return Ok(0)
                }
                
                if stackpos==indirection{
                    if (cursizesector+1)*1024 > offset{
                        let abspos = offset-(cursizesector*1024);
                        self.stream.seek(SeekPos::StartSector(stack[stackpos].base_sector))?;
                        self.stream.seek(SeekPos::Curr(abspos as i64))?;

                        let len = ((buf.len() as u64).min(len-offset).min(stack[stackpos].extent*1024-abspos)) as usize;

                        return self.stream.read(&mut buf[..len]);
                    }else{
                        cursizesector += stack[stackpos].extent;
                        offset_at_level = stack[stackpos].__reserved + 1;
                        stackpos -= 1;
                        continue;
                    }
                }

                if offset_at_level*32 >= stack[stackpos-1].extent*1024{
                    cursizesector += stack[stackpos].extent;
                    offset_at_level = stack[stackpos].__reserved + 1;
                    stackpos -= 1;
                    continue;
                }

                self.stream.seek(SeekPos::StartSector(stack[stackpos].base_sector))?;
                self.stream.seek(SeekPos::Curr((stack[stackpos].__reserved as i64)*32))?;

                self.stream.read_fully(bytemuck::bytes_of_mut(&mut stack[stackpos+1]));
                stackpos += 1;
                stack[stackpos].__reserved = offset_at_level;
                offset_at_level = 0;
            }
        }

    }

    fn read_fully_by_indirection(&mut self, mut offset: u64,mut buf: &mut [u8], baseref: u128, indirection: u8,len: u64) -> crate::io::Result<()>{
        if indirection==0{
            panic!("read_by_indirection does not support inline data, handle via appropriate top-level type instead")
        }

        while !buf.is_empty(){
            match self.read_by_indirection(offset, buf, baseref, indirection,len){
                Ok(0) => return Err(crate::io::Error::UnexpectedEof),
                Ok(n) => {
                    buf = &mut buf[n..];
                    offset += n as u64;
                }
                Err(crate::io::Error::Interrupted) => {}
                Err(e) => return Err(e)
             }
        }
        Ok(())
    }

    fn read_nullstr_by_indirection(&mut self, mut offset: u64, baseref: u128, indirection: u8, len: u64) -> crate::io::Result<String>{
        let mut str = Vec::new();
        loop{
            let mut buf = [0;1024];
            match self.read_by_indirection(offset, &mut buf, baseref, indirection, len){
                Ok(0) => return Err(crate::io::Error::UnexpectedEof),
                Ok(n) => {
                    let buf = &buf[..n];

                    for b in buf{
                        if *b==0{
                            return String::from_utf8(str).map_err(|_|crate::io::Error::InvalidData)
                        }
                        str.push(*b);
                    }

                    offset += n as u64;
                }
                Err(crate::io::Error::Interrupted) => continue,
                Err(e) => return Err(e)
            }
        }
    }

    fn cmp_nullstr_by_indirection(&mut self, name: &str, mut offset: u64, baseref: u128, indirection: u8, len: u64) -> crate::io::Result<Ordering>{
        let mut name = name.as_bytes();
        loop{
            let mut buf = [0;1024];
            match self.read_by_indirection(offset, &mut buf, baseref, indirection, len){
                Ok(0) => return Err(crate::io::Error::UnexpectedEof),
                Ok(n) => {
                    let buf = &buf[..n];

                    for (idx,b) in buf.iter().enumerate(){
                        if *b==0{
                            return Ok(idx.cmp(&name.len()))
                        }
                        else if name.len()==idx{
                            return Ok(Ordering::Greater);
                        }else{
                            match buf[idx].cmp(&b){
                                Ordering::Equal => continue,
                                o => return Ok(o)
                            }
                        }
                    }
                    name = &name[n..];
                    offset += n as u64;
                }
                Err(crate::io::Error::Interrupted) => continue,
                Err(e) => return Err(e)
            }
        }
    }

    fn cmp_nullstr_from_stream(&mut self, str: &str, pos: u64, stream: &StreamListing) -> crate::io::Result<Ordering>{
        

        if pos>stream.size{
            return Err(crate::io::Error::UnexpectedEof)
        }

        let indirection = stream.flags.get_indirection() as u8;


        if indirection==0{
            let str = str.as_bytes();
            let base = &stream.inline_data[..(pos as usize)];

            let content = base.split(|n|*n==0).next().unwrap();

            Ok(str.cmp(content))
        }else{
            self.cmp_nullstr_by_indirection(str, pos, stream.content_ref, indirection, stream.size)
        }
    }

    pub fn read_from_stream(&mut self, buf: &mut [u8], pos: u64, stream: &StreamListing) -> crate::io::Result<usize>{
        let max_len = buf.len().min(stream.size.saturating_sub(pos) as usize);

        let buf = &mut buf[..max_len];

        let indirection = stream.flags.get_indirection() as u8;

        if indirection ==0{
            buf.copy_from_slice(&stream.inline_data[(pos as usize)..][..max_len]);

            Ok(max_len)
        }else{
            self.read_by_indirection(pos, buf, stream.content_ref, indirection, stream.size)
        }
    }

    pub fn read_fully_from_stream(&mut self, buf: &mut [u8], pos: u64, stream: &StreamListing) -> crate::io::Result<()>{

        if (stream.size.saturating_sub(pos) as usize)<buf.len(){
            return Err(crate::io::Error::UnexpectedEof)
        }

        let indirection = stream.flags.get_indirection() as u8;
        if indirection==0{
            buf.copy_from_slice(&stream.inline_data[(pos as usize)..][..buf.len()]);

            Ok(())
        }else{
            self.read_fully_by_indirection(pos, buf, stream.content_ref, indirection, stream.size)
        }
    }

    pub fn get_stream_by_id(&mut self, objid: ObjectId, stream: StreamId) -> crate::io::Result<StreamListing>{
        let obj = self.get_obj_by_id(objid)?;

        if obj.strong_ref==0{
            // Note: weak_ref is tested by `get_obj_by_id`. Otherwise, condition should test weak_ref as well, since strong_ref has undefined content with weak_ref==0
            return Err(crate::io::Error::NotFound)
        }

        let pos = stream.0*(size_of::<StreamListing>() as u64);

        if pos>=obj.streams_size{
            return Err(crate::io::Error::NotFound)
        }

        let mut listing = Zeroable::zeroed();

        self.read_fully_by_indirection(pos, bytemuck::bytes_of_mut(&mut listing), obj.streams_ref,obj.streams_indirection,obj.streams_size)?;

        Ok(listing)
    }

    pub fn find_stream_by_id(&mut self, objid: ObjectId, stream: &str) -> crate::io::Result<(StreamId,StreamListing)>{
        let mut idx = 0u64;

        let obj = self.get_obj_by_id(objid)?;
        
        let strings_stream = obj.strings_stream.map(|id| self.get_stream_by_id(objid,StreamId(id.get()))).transpose()?;


        while (idx*(size_of::<StreamListing>() as u64))<obj.streams_size{
            let mut listing: StreamListing = Zeroable::zeroed();
            self.read_fully_by_indirection(idx*(size_of::<StreamListing>() as u64), bytemuck::bytes_of_mut(&mut listing), obj.streams_ref,obj.streams_indirection,obj.streams_size)?;

            if let Some(nref) = listing.name_ref{
                if let Some(strings) = &strings_stream{
                    if self.cmp_nullstr_from_stream(stream, nref.get(), strings)?==Ordering::Equal{
                        return Ok((StreamId(idx),listing));
                    }
                }
            }else{
                let name = listing.name.split(|f|*f==0).next().unwrap();

                if stream.as_bytes()==name{
                    return Ok((StreamId(idx),listing));
                }
            }
            idx+=1;
        }
        
        Err(crate::io::Error::NotFound)
    }


    pub fn search_directory(&mut self, objid: ObjectId, subfilename: &str) -> crate::io::Result<ObjectId>{
        let obj = self.get_obj_by_id(objid)?;

        let (_,stream) = self.find_stream_by_id(objid, consts::DIRECTORYCONTENT_STREAM)?;

        let strings_stream = obj.strings_stream.map(|id| self.get_stream_by_id(objid,StreamId(id.get()))).transpose()?;

        let len = stream.size/(size_of::<DirectoryElement>() as u64);
        for i in 0..len{
            let mut element: DirectoryElement = Zeroable::zeroed();

            self.read_from_stream(bytemuck::bytes_of_mut(&mut element), i*(size_of::<DirectoryElement>() as u64), &stream)?;

            if let Some(nameref) = element.name_index{
                if let Some(strings) = &strings_stream{
                    if self.cmp_nullstr_from_stream(subfilename, nameref.get(), strings)?.is_eq(){
                        return element.objidx.ok_or(crate::io::Error::NotFound);
                    }
                }
            }else{
                let name = element.name.split(|f|*f==0).next().unwrap();

                if subfilename.as_bytes()==name{
                    return element.objidx.ok_or(crate::io::Error::NotFound);
                }
            }
        }


        Err(crate::io::Error::NotFound)
    }

}