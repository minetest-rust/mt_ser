use super::*;

#[mt_derive(to = "clt")]
pub struct ObjAdd; // TODO

#[mt_derive(to = "clt")]
pub struct ObjMsg; // TODO

#[mt_derive(to = "clt", repr = "u8", enumset)]
pub enum MapBlockFlag {
    IsUnderground = 0,
    DayNightDiff,
    LightExpired,
    NotGenerated,
}

pub const ALWAYS_LIT_FROM: u16 = 0xf000;

#[mt_derive(to = "clt")]
pub struct MapBlock {
    pub flags: EnumSet<MapBlockFlag>,
    pub lit_from: u16,

    #[mt(const8 = 2)]
    #[serde(skip)]
    pub param0_size: (),

    #[mt(const8 = 2)]
    #[serde(skip)]
    pub param12_size: (),

    #[serde(with = "serde_arrays")]
    pub param_0: [u16; 4096],
    #[serde(with = "serde_arrays")]
    pub param_1: [u8; 4096],
    #[serde(with = "serde_arrays")]
    pub param_2: [u8; 4096],

    pub node_metas: HashMap<u16, NodeMeta>,

    #[mt(const8 = 2)]
    #[serde(skip)]
    pub version: (),
}
