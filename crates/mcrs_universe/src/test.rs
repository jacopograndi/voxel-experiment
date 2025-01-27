use crate::block::{BlockFlag, FlagBank};

#[test]
fn smoke_test_flag_bank() {
    assert_eq!(
        BlockFlag::iter().count(),
        8,
        "BlockFlags hasn't exactly 8 variants: {:?}",
        BlockFlag::iter()
            .map(|f| f.to_string())
            .collect::<Vec<String>>()
    );

    let mut bank = FlagBank::default();
    dbg!(bank);
    bank.set(BlockFlag::Collidable);
    dbg!(bank);
    bank.set(BlockFlag::Opaque);
    dbg!(bank);
    let flags: Vec<BlockFlag> = bank.into();
    println!("{:?}", flags);
    assert_eq!(2, 3);
}
