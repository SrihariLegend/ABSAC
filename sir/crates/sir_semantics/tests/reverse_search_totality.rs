//! Reverse position-search totality gate.
//!
//! A structurally reverse search (`successor = i - 1`) may only be
//! authorized when its iteration domain is PROVEN total: the carried
//! index starts at `extent - 1`, the scanned access uses that index,
//! and the termination contains an underflow guard
//! (`successor < carry`), so the scan visits `extent - 1 .. 0` and
//! stops at index 0.
//!
//! The PS002 shape (`i >= 0` on an UNSIGNED induction) has no such
//! guard: with no match, `i = 0` is followed by `i - 1 = u64::MAX` and
//! the guard stays true forever. It must never receive a PositionSearch
//! authorization — this is the gate that keeps the BitScanReverse lift
//! sound.

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_semantics::authorization::{derive_authorizations, DomainCertificate};
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};

fn u64_type() -> Type {
    Type::u64()
}
fn bool_type() -> Type {
    Type::Bool
}
fn bool_array(len: usize) -> Type {
    Type::Array {
        element: Box::new(Type::Bool),
        length: len,
    }
}
fn unknown() -> Span {
    Span::unknown()
}

/// Sound reverse search: `i = len - 1 .. 0` with the underflow guard
/// `successor < carry` (and a found flag for early exit).
fn build_sound_reverse(len: usize) -> sir_nodes::Function {
    let mut b = Builder::new("last_set_bit_sound", &[("board", bool_array(len))], u64_type());
    let board = b.parameter_index(0).unwrap();
    let start = b.constant(ConstantData::u64(len as u64 - 1), u64_type(), unknown());
    let one = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let sentinel = b.constant(ConstantData::u64(len as u64), u64_type(), unknown());
    let found_init = b.constant(ConstantData::boolean(false), bool_type(), unknown());

    let elem = b
        .array_access(board, start, bool_type(), unknown())
        .unwrap();
    let new_found = b.bool_or(found_init, elem, unknown()).unwrap();
    let not_found_yet = b.bool_not(found_init, unknown()).unwrap();
    let is_last = b.bool_and(elem, not_found_yet, unknown()).unwrap();
    let new_index = b.select(is_last, start, sentinel, unknown()).unwrap();

    let successor = b.sub(start, one, unknown()).unwrap();
    let not_found = b.bool_not(found_init, unknown()).unwrap();
    // Sound termination: continue while no match AND no underflow.
    let no_underflow = b.lt(successor, start, unknown()).unwrap();
    let cond = b.bool_and(not_found, no_underflow, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[
                elem,
                new_found,
                not_found_yet,
                is_last,
                new_index,
                successor,
                not_found,
                no_underflow,
                cond,
            ],
            cond,
            &[new_found, new_index, successor],
            &[found_init, sentinel, start],
            Type::Tuple {
                elements: vec![bool_type(), u64_type(), u64_type()],
            },
            unknown(),
        )
        .unwrap();
    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    b.build()
}

/// Unsound PS002 shape: unsigned `i >= 0` never stops the reverse scan.
fn build_unsound_reverse(len: usize) -> sir_nodes::Function {
    let mut b = Builder::new("last_set_bit_unsound", &[("board", bool_array(len))], u64_type());
    let board = b.parameter_index(0).unwrap();
    let start = b.constant(ConstantData::u64(len as u64 - 1), u64_type(), unknown());
    let one = b.constant(ConstantData::u64(1), u64_type(), unknown());
    let zero = b.constant(ConstantData::u64(0), u64_type(), unknown());
    let sentinel = b.constant(ConstantData::u64(len as u64), u64_type(), unknown());
    let found_init = b.constant(ConstantData::boolean(false), bool_type(), unknown());

    let elem = b
        .array_access(board, start, bool_type(), unknown())
        .unwrap();
    let new_found = b.bool_or(found_init, elem, unknown()).unwrap();
    let not_found_yet = b.bool_not(found_init, unknown()).unwrap();
    let is_last = b.bool_and(elem, not_found_yet, unknown()).unwrap();
    let new_index = b.select(is_last, start, sentinel, unknown()).unwrap();

    let successor = b.sub(start, one, unknown()).unwrap();
    let not_found = b.bool_not(found_init, unknown()).unwrap();
    // UNSOUND: `i >= 0` is always true for unsigned.
    let in_bounds = b.ge(start, zero, unknown()).unwrap();
    let cond = b.bool_and(not_found, in_bounds, unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[
                elem,
                new_found,
                not_found_yet,
                is_last,
                new_index,
                successor,
                not_found,
                in_bounds,
                cond,
            ],
            cond,
            &[new_found, new_index, successor],
            &[found_init, sentinel, start],
            Type::Tuple {
                elements: vec![bool_type(), u64_type(), u64_type()],
            },
            unknown(),
        )
        .unwrap();
    let res = b
        .field_access(loop_node, "1", u64_type(), unknown())
        .unwrap();
    b.return_value(res, unknown()).unwrap();
    b.build()
}

/// `(saw LastOccurrence concept, saw a PositionSearch certificate)`.
fn position_authorization(func: &sir_nodes::Function) -> (bool, bool) {
    let mut analysis = AnalysisManager::new();
    analysis.run_all(func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(func, analysis.database());

    let saw_last = semantics
        .database()
        .regions()
        .any(|(_, region)| region.concepts().contains(&SemanticConcept::LastOccurrence));

    let authorizations =
        derive_authorizations(func, analysis.database(), semantics.database());
    let saw_position = semantics
        .database()
        .regions()
        .any(|(region_id, _)| {
            authorizations.for_region(region_id).iter().any(|auth| {
                matches!(
                    auth.domain,
                    DomainCertificate::PositionSearch { .. }
                )
            })
        });
    (saw_last, saw_position)
}

#[test]
fn sound_reverse_search_is_authorized() {
    let (saw_last, saw_position) = position_authorization(&build_sound_reverse(64));
    assert!(
        saw_last,
        "the underflow-guarded reverse scan must still be recognized"
    );
    assert!(
        saw_position,
        "a total reverse search (start = extent-1, successor = i-1, \
         underflow guard) must receive a PositionSearch certificate"
    );
}

#[test]
fn unsound_reverse_search_is_not_authorized() {
    let (saw_last, saw_position) = position_authorization(&build_unsound_reverse(64));
    assert!(
        saw_last,
        "the PS002 shape is still recognized as a reverse search"
    );
    assert!(
        !saw_position,
        "an unsigned `i >= 0` reverse scan never terminates without a \
         match; it must NOT receive a PositionSearch certificate"
    );
}
