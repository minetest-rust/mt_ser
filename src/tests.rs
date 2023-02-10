use super::*;

fn reserialize<C: MtCfg, T: MtSerialize + MtDeserialize>(item: &T) -> T {
    let mut writer = Vec::new();
    item.mt_serialize::<C>(&mut writer).unwrap();

    let mut reader = std::io::Cursor::new(writer);
    T::mt_deserialize::<C>(&mut reader).unwrap()
}

#[test]
fn test_reserialize() {
    let vec = vec![1, 2, 3];
    // encoded with 8-bit length
    assert_eq!(vec, reserialize::<u8, _>(&vec));

    let vec2 = vec![1, 2, 3];
    // encoded without length - drains the Reader
    assert_eq!(vec2, reserialize::<(), _>(&vec2));

    let st: String = "µ ß 私 😀\n".into();
    // encoded as UTF-16 with 32-bit length (also works with () or other types)
    assert_eq!(st, reserialize::<Utf16<u32>, _>(&st));

    let long: Vec<_> = (0..=256).collect();
    assert!(matches!(
        long.mt_serialize::<u8>(&mut Vec::new()),
        Err(SerializeError::TooBig(_))
    ));
}
