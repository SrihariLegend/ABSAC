use sir_types::Type;

#[test]
fn test_bitvector() {
    let ty = Type::BitVector { width: 64 };
    assert!(ty.is_integer_or_bitvector());
}
