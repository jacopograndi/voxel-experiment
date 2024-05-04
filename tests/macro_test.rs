use mcrs_universe::block::BlockFlag;

#[test]
fn test_macro_print() {
    for v in BlockFlag::iter() {
        println!("{:?}", v);
    }
    assert_eq!(2, 3);
}