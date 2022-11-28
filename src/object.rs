
use core::num::NonZeroU64;

use bytemuck::{Pod,Zeroable, PodInOption, ZeroableInOption, TransparentWrapper};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, TransparentWrapper)]
pub struct ObjectId(pub NonZeroU64);

unsafe impl ZeroableInOption for ObjectId{}
unsafe impl PodInOption for ObjectId{}


#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, TransparentWrapper, Pod, Zeroable)]
pub struct StreamId(pub u64);

impl StreamId{
    pub const STREAMS: Self = Self(0);
}

pub mod consts{
    pub const STREAMS_STREAM: &str = "Streams";
    pub const STRINGS_STREAM: &str = "Strings";
    pub const FILEDATA_STREAM: &str = "FileData";
    pub const DIRECTORYCONTENT_STREAM: &str = "DirectoryContent";
    pub const SYMLINKTARGET_STREAM: &str = "SymlinkTarget";
    pub const DEVICEID_STREAM: &str = "DeviceId";
    pub const LEGACYDEVICENUMBER_STREAM: &str = "LegacyDeviceNumber";
    pub const CUSTOMOBJECTINFO_STREAM: &str = "CustomObjectInfo";
    pub const SECURITYDESCRIPTOR_STREAM: &str = "SecurityDescriptor";
    pub const LEGACYSECURITYDESCRIPTOR_STREAM: &str = "LegacySecurityDescriptor";
    
    
    pub const OBJECTOWNER_PERMISSION: &str = "ObjectOwner";
    pub const READ_PERMISSION: &str = "Read";
    pub const WRITE_PERMISSION: &str = "Write";
    pub const EXECUTE_PERMISION: &str = "Execute";
    pub const ACCESSDIRECTORY_PERMISSION: &str = "AccessDirectory";
    pub const TAKEOWNERSHIP_PERMISSION: &str = "TakeOwnership";
    pub const CREATEOBJECT_PERMISION: &str = "CreateObject";
    pub const REMOVEOBJECT_PERMISSION: &str = "RemoveObject";
    pub const ALL_PERMISSIONS: &str = "*";

    pub const DEFAULT_PRINCIPAL: u128 = !0;
    pub const SYSTEM_PRINCIPAL: u128 = 0;

}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(TransparentWrapper, Pod, Zeroable)]
    pub struct StreamFlags : u64 {
        const REQUIRED = 0x00000001;
        const WRITE_REQUIRED = 0x00000002;
        const ENUMERATE_REQUIRED = 0x00000004;
        const PRESERVED = 0x00000008;
        const INDIRECTION_MASK = 0x000000F0;
        const IMPL_USE_MASK = 0xFFF0000000000000;
    }
}

impl StreamFlags{
    pub fn get_indirection(&self) -> u64{
        ((*self)&Self::INDIRECTION_MASK).bits()>>4
    }
}

#[repr(C,align(128))]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
pub struct StreamListing{
    pub name: [u8;32],
    pub name_ref: Option<NonZeroU64>,
    pub flags: StreamFlags,
    pub content_ref: u128,
    pub size: u64, 
    pub reserved: [u64; 3],
    pub inline_data: [u8;32]
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(TransparentWrapper, Pod, Zeroable)]
    pub struct ObjectFlags : u32{

    }
}

fake_enum::fake_enum! {
    #[repr(u16)]
    #[derive(Hash, TransparentWrapper, Pod, Zeroable)]
    pub enum struct ObjectType{
        RegularFile = 0,
        Directory = 1,
        Symlink = 2,
        PosixFifo = 3,
        UnixSocket = 4,
        BlockDevice = 5,
        CharDevice = 6,
        CustomType = 65535
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(64))]
pub struct Object{
    pub strong_ref: u32,
    pub weak_ref: u32, 
    pub streams_size: u64,
    pub streams_ref: u128,
    pub streams_indirection: u8,
    #[doc(hidden)]
    pub __reserved33: [u8; 5],
    pub ty: ObjectType,
    pub flags: ObjectFlags,
    #[doc(hidden)]
    pub __reserved44: [u8; 20],
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(TransparentWrapper, Pod, Zeroable)]
    pub struct DirectoryElementFlags : u64{
        const WEAK = 0x00000001;
        const HIDDEN = 0x00000002;
    } 
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(64))]
pub struct DirectoryElement{
    pub objidx: Option<ObjectId>,
    pub name_index: Option<NonZeroU64>,
    pub flags: DirectoryElementFlags,
    pub name: [u8;40]
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(16))]
pub struct DeviceId{
    pub devid_lo: u64,
    pub devid_hi: u64,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(8))]
pub struct LegacyDeviceNumber{
    pub major: u32,
    pub minor: u32
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(TransparentWrapper, Pod, Zeroable)]
    pub struct SecurityDescRowFlags : u64{
        const MODE_MASK = 0xFF;
        const REQUIRED = 0x100;
        const IMPL_BITS_MASK = 0xFF00000000000000;
    }
}

fake_enum::fake_enum! {
    #[repr(u64)]
    #[derive(Hash, TransparentWrapper, Pod, Zeroable)]
    pub enum struct SecurityDescRowMode {
        Permit = 0,
        Deny = 1,
        Forbid = 2,
        Inherit = 3,
    }
}

impl From<SecurityDescRowMode> for SecurityDescRowFlags{
    fn from(mode: SecurityDescRowMode) -> Self{
        bytemuck::cast(mode)
    }
}

impl SecurityDescRowFlags{
    pub fn mode(&self) -> SecurityDescRowMode{
        SecurityDescRowMode(((*self)&Self::MODE_MASK).bits())
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(64))]
pub struct SecurityDescriptorRow{
    pub principal: u128,
    pub stream_id: StreamId,
    pub flags_and_mode: SecurityDescRowFlags,
    pub permission_name_ref: Option<NonZeroU64>,
    pub permission_name: [u8;24]
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(16))]
pub struct LegacySecurityDescriptor{
    pub sd_uid: u32,
    pub sd_gid: u32,
    pub sd_mode: u16,
    #[doc(hidden)]
    pub __sd_reserved: [u8;6]
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(4))]
pub struct PhantomFSMagic([u8;4]);

impl PhantomFSMagic{
    pub const MAGIC: Self = Self([0xF3,0x50,0x48,0x53]);
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(TransparentWrapper, Pod, Zeroable)]
    pub struct FSRequiredFeatures : u32{

    }
}


bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(TransparentWrapper, Pod, Zeroable)]
    pub struct FSOptionalFeatures : u32{

    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C,align(128))]
pub struct RootDescriptor{
    pub magic: PhantomFSMagic,
    pub version_major: u16,
    pub version_minor: u16,
    pub required_features: FSRequiredFeatures,
    pub optional_features: FSOptionalFeatures,
    pub volume_id_lo: u64,
    pub volume_id_hi: u64,
    pub root_object_id: Option<ObjectId>,
    pub objtab_size: u64,
    pub objtab_end: u128,
    pub alloc_tab_size: u64,
    pub alloc_tab_begin: u64,
    pub label_ref: Option<NonZeroU64>,
    pub label: [u8; 32],
    pub header_size: u32,
    pub crc: u32
}