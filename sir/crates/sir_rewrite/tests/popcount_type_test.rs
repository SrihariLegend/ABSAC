use sir_rewrite::local_id::LocalNodeId;
use sir_rewrite::subgraph_builder::SubgraphBuilder;
use sir_types::{Span, Type};

#[test]
fn test_popcount_type() {
    let mut builder = SubgraphBuilder::new();
    let packed = builder.pack(LocalNodeId::new(0), 8, Span::unknown());
    let pop = builder.popcount(packed, Span::unknown());
    let ty = builder.get_type(pop).unwrap();
    assert_eq!(ty, Type::i32());
}
