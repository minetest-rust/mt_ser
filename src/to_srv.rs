use crate::*;

#[mt_derive(to = "srv")]
pub struct PlayerPos {
    #[mt(const_u16 = 1)] // supported compression
    pub pos_100: [i32; 3],
    pub vel_100: [i32; 3],
    pub pitch_100: i32,
    pub yaw_100: i32,
    pub fov_80: u8,
    pub wanted_range: u8,
}

#[mt_derive(to = "srv", repr = "u8")]
pub enum Interaction {
    Dig = 0,
    StopDigging,
    Dug,
    Place,
    Use,
    Activate,
}

#[mt_derive(to = "srv")]
pub struct PointedThing; // TODO

#[mt_derive(to = "srv", repr = "u16", tag = "type", content = "data")]
pub enum ToSrvPkt {
    Nil = 0,
    Init {
        serialize_version: u8,
        #[mt(const_u16 = 1)] // supported compression
        min_proto_version: u16,
        max_proto_version: u16,
        player_name: String,
        #[mt(default)]
        send_full_item_meta: bool,
    } = 2,
    Init2 {
        lang: String,
    } = 17,
    JoinModChan {
        channel: String,
    } = 23,
    LeaveModChan {
        channel: String,
    } = 24,
    MsgModChan {
        channel: String,
        msg: String,
    } = 25,
    PlayerPos(PlayerPos) = 35,
    GotBlocks {
        #[mt(len8)]
        blocks: Vec<[i16; 3]>,
    } = 36,
    DeletedBlocks {
        #[mt(len8)]
        blocks: Vec<[i16; 3]>,
    } = 37,
    InvAction {
        #[mt(len0)]
        action: String,
    } = 49,
    ChatMsg {
        #[mt(utf16)]
        msg: String,
    } = 50,
    FallDmg {
        amount: u16,
    } = 53,
    SelectItem {
        select_item: u16,
    } = 55,
    Respawn = 56,
    Interact {
        action: Interaction,
        item_slot: u16,
        #[mt(size_u32)]
        pointed: PointedThing,
        pos: PlayerPos,
    } = 57,
    RemovedSounds {
        ids: Vec<i32>,
    } = 58,
    NodeMetaFields {
        pos: [i16; 3],
        formname: String,
        fields: HashMap<String, String>,
    } = 59,
    InvFields {
        formname: String,
        fields: HashMap<String, String>,
    } = 60,
    ReqMedia {
        filenames: Vec<String>,
    } = 64,
    CltReady {
        major: u8,
        minor: u8,
        patch: u8,
        reserved: u8,
        version: String,
        formspec: u16,
    } = 67,
    FirstSrp {
        salt: Vec<u8>,
        verifier: Vec<u8>,
        empty_passwd: bool,
    } = 80,
    SrpBytesA {
        a: Vec<u8>,
        no_sha1: bool,
    } = 81,
    SrpBytesM {
        m: Vec<u8>,
    } = 82,
    Disco = 0xffff,
}
