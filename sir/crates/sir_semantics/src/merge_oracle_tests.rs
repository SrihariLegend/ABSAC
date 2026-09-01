//! Region-merge oracle: the merged region layout must be exactly the
//! connected components of the "shares a non-parameter/non-constant
//! node" relation (P0A hardening, advisor checklist item 8).

use crate::region::Region;
use crate::semantics::SemanticDatabase;
use sir_types::{NodeId, RegionId};

/// Minimal SIR function whose arena carries `n` non-parameter,
/// non-constant nodes (a chain of Adds). Node ids 1..=n are usable as
/// shared-overlap anchors.
fn chain_function(n: u64) -> sir_nodes::Function {
    use sir_builder::Builder;
    use sir_types::{ConstantData, Span, Type};
    let mut b = Builder::new("merge_oracle", &[("x", Type::u64())], Type::u64());
    let x = b.parameter_index(0).unwrap();
    let one = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let mut cur = b.add(x, one, Span::unknown()).unwrap();
    for _ in 1..n {
        cur = b.add(cur, one, Span::unknown()).unwrap();
    }
    b.return_value(cur, Span::unknown()).unwrap();
    b.build()
}

/// Build a SemanticDatabase with regions (id, nodes) as given.
fn db_with_regions(specs: &[(u64, &[u64])]) -> SemanticDatabase {
    let mut db = SemanticDatabase::new();
    for (rid, nodes) in specs {
        let mut region = Region::new(RegionId::new(*rid));
        for &n in *nodes {
            region.nodes.insert(NodeId::new(n));
        }
        db.add_region(region);
    }
    db
}

/// Oracle: independently compute connected components over
/// "shares a node", returning for each surviving region its node set.
/// Root of each component is its minimum region id.
fn oracle_partition(specs: &[(u64, &[u64])]) -> Vec<(u64, Vec<u64>)> {
    let mut parent: Vec<usize> = (0..specs.len()).collect();
    fn find(p: &mut Vec<usize>, mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..specs.len() {
        for j in (i + 1)..specs.len() {
            let shared = specs[i]
                .1
                .iter()
                .any(|n| specs[j].1.contains(n));
            if shared {
                let ri = find(&mut parent, i);
                let rj = find(&mut parent, j);
                if ri != rj {
                    // Union by minimum region id, matching the
                    // implementation's min-root convention.
                    let (lo, hi) = if specs[ri].0 <= specs[rj].0 {
                        (ri, rj)
                    } else {
                        (rj, ri)
                    };
                    parent[hi] = lo;
                }
            }
        }
    }
    // Group by root; each component reports the min id and the union
    // of node sets.
    let mut out: Vec<(u64, Vec<u64>)> = Vec::new();
    for (i, (rid, nodes)) in specs.iter().enumerate() {
        let nodes: &[u64] = nodes;
        let root = specs[find(&mut parent, i)].0;
        match out.iter_mut().find(|(rid, _)| *rid == root) {
            Some((_, ns)) => {
                for &n in nodes.iter() {
                    if !ns.contains(&n) {
                        ns.push(n);
                    }
                }
            }
            None => out.push((root, nodes.to_vec())),
        }
    }
    for (root, ns) in out.iter_mut() {
        ns.sort();
    }
    out.sort();
    out
}

/// The previously-broken pattern: R0~R1 share node 1; R1~R3 share node
/// 3; R2~R3 share node 5. All four regions form ONE connected
/// component. The old merge_map implementation could lose the R1~R3
/// edge when region 3 was already claimed as a merge source, leaving
/// R2 disconnected — an under-merge (wrong, not just unstable).
#[test]
fn transitive_overlap_merges_into_one_component() {
    let func = chain_function(9);
    // Anchor ids 2..=8 are Add nodes (id 1 is the constant, excluded
    // from overlap by the Parameter/Constant filter).
    let specs: &[(u64, &[u64])] = &[
        (0, &[2]),
        (1, &[2, 4]),
        (2, &[3, 5]),
        (3, &[4, 5]),
        (4, &[7]),
        (5, &[8]),
    ];
    let expected = oracle_partition(specs);
    assert_eq!(expected.len(), 3, "oracle sanity: 3 components");

    let mut db = db_with_regions(specs);
    db.merge_overlapping_regions(&func);

    let mut actual: Vec<(u64, Vec<u64>)> = db
        .regions()
        .map(|(rid, r)| (rid.0, region_nodes(&r)))
        .collect();
    actual.sort();
    assert_eq!(actual, expected, "merged layout must equal the connected-components oracle");
}

fn region_nodes(r: &Region) -> Vec<u64> {
    r.nodes.iter().map(|n| n.0).collect()
}

/// Idempotency: merging twice must be a no-op (the merged layout is
/// already a partition — components share no nodes).
#[test]
fn merge_is_idempotent() {
    let func = chain_function(9);
    let specs: &[(u64, &[u64])] = &[
        (0, &[2]),
        (1, &[2, 4]),
        (2, &[3, 5]),
        (3, &[4, 5]),
    ];
    let mut db = db_with_regions(specs);
    db.merge_overlapping_regions(&func);
    let after_first: Vec<(u64, Vec<u64>)> = db
        .regions()
        .map(|(rid, r)| (rid.0, region_nodes(r)))
        .collect();
    db.merge_overlapping_regions(&func);
    let after_second: Vec<(u64, Vec<u64>)> = db
        .regions()
        .map(|(rid, r)| (rid.0, region_nodes(&r)))
        .collect();
    assert_eq!(after_first, after_second, "merge must be idempotent");
}
