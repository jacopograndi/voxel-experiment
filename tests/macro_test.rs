use mcrs_universe::block::{BlockFlag, FlagBank};

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
