#[derive(Debug, Copy, Clone, Serialize, Deserialize, EnumIter)]
pub enum BlockFlag {
    // Bit index of each block flag in human readable form
    Collidable,
    Opaque,
    Flag3,
    Flag4,
    Flag5,
    Flag6,
    Flag7,
    Flag8,
}

#[test]
fn test_macro_print() {
    println!("All macro variants: ");
    for v in BlockFlag::iter() {
        println!("    - {:?}", v);
    }
    println!("FlagBank test:");
    let mut bank = FlagBank::default();
    println!("{:?}", bank);
    bank.set(BlockFlag::Collidable);
    println!("{:?}", bank);
    bank.set(BlockFlag::Opaque);
    println!("{:?}", bank);
    let flags: Vec<BlockFlag> = bank.into();
    println!("{:?}", flags);
    assert_eq!(2, 3);
}
