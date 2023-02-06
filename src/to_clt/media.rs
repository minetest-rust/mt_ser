use super::*;

#[mt_derive(to = "clt")]
pub struct MediaAnnounce {
    pub name: String,
    pub base64_sha1: String,
}

#[mt_derive(to = "clt")]
pub struct MediaPayload {
    pub name: String,
    #[mt(len32)]
    pub data: Vec<u8>,
}

#[mt_derive(to = "clt")]
pub struct TileAnim; // TODO

#[mt_derive(to = "clt")]
pub struct ItemDef; // TODO

#[mt_derive(to = "clt")]
pub struct NodeDef; // TODO

#[mt_derive(to = "clt")]
pub struct NodeMeta; // TODO

#[mt_derive(to = "clt", repr = "u16")]
pub enum SoundSrcType {
    Nowhere = 0,
    Pos,
    Obj,
}
