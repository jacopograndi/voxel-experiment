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
    bank.set(BlockFlag::Collidable);
    bank.set(BlockFlag::Opaque);
    let flags: Vec<BlockFlag> = bank.into();
    let mut flags_iter = flags.iter();
    assert_eq!(Some(&BlockFlag::Collidable), flags_iter.next());
    assert_eq!(Some(&BlockFlag::Opaque), flags_iter.next());
    assert_eq!(None, flags_iter.next());
}
