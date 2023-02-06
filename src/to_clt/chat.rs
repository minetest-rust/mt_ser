use super::*;

#[mt_derive(to = "clt", repr = "u8")]
pub enum ChatMsgType {
    Raw = 0,
    Normal,
    Announce,
    System,
}

#[mt_derive(to = "clt", repr = "u8")]
pub enum PlayerListUpdateType {
    Init = 0,
    Add,
    Remove,
}
