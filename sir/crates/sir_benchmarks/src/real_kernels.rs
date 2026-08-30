//! Real kernels — hand-lowered SIR from real open-source code.
//!
//! Each function here corresponds to a corpus entry in /corpus/.
//! The `build_by_id` function maps a corpus ID to its builder.

use sir_builder::Builder;
use sir_nodes::Function;
use sir_types::{ConstantData, Span, Type};

fn span() -> Span {
    Span::unknown()
}

/// Corpus entry 001: Redis BITCOUNT tail loop.
///
/// ```text
/// long long bits = 0;
/// while (i < count) {
///     bits += bitsinbyte[buf[i]];
///     i++;
/// }
/// return bits;
/// ```
///
/// Provenance: redis/redis, src/bitops.c, redisPopcount() `remain:` tail loop.
fn build_redis_bitcount_tail() -> Function {
    let mut b = Builder::new(
        "redis_bitcount_tail",
        &[
            ("buf", Type::Array { element: Box::new(Type::u8()), length: 64 }),
            ("count", Type::u64()),
            ("bitsinbyte", Type::Array { element: Box::new(Type::u64()), length: 256 }),
        ],
        Type::u64(),
    );
    let buf = b.parameter_index(0).unwrap();
    let count = b.parameter_index(1).unwrap();
    let table = b.parameter_index(2).unwrap();

    let one = b.constant(ConstantData::u64(1), Type::u64(), span());
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), span());
    let bits_init = b.constant(ConstantData::u64(0), Type::u64(), span());

    let byte = b.array_access(buf, i_init, Type::u8(), span()).unwrap();
    let to_add = b.array_access(table, byte, Type::u64(), span()).unwrap();
    let next_bits = b.add(bits_init, to_add, span()).unwrap();
    let next_i = b.add(i_init, one, span()).unwrap();
    let cond = b.lt(next_i, count, span()).unwrap();

    let loop_node = b
        .r#loop(
            &[byte, to_add, next_bits, next_i, cond],
            cond,
            &[next_i, next_bits],
            &[i_init, bits_init],
            Type::Tuple { elements: vec![Type::u64(), Type::u64()] },
            span(),
        )
        .unwrap();

    let bits_result = b.tuple_extract(loop_node, 1, Type::u64(), span()).unwrap();
    b.return_value(bits_result, span()).unwrap();

    b.build()
}

/// Corpus entry 002: OpenSSL constant-time memcmp.
///
/// ```text
/// uint8_t x = 0;
/// for (i = 0; i < len; i++) {
///     x |= a[i] ^ b[i];
/// }
/// return x;
/// ```
///
/// Provenance: openssl/openssl, CRYPTO_memcmp — constant-time byte
/// comparison that prevents timing side-channels. -O3 cannot
/// short-circuit or aggressively vectorize this loop.
fn build_openssl_ct_memcmp() -> Function {
    let mut b = Builder::new(
        "openssl_ct_memcmp",
        &[
            ("a", Type::Array { element: Box::new(Type::u8()), length: 64 }),
            ("b", Type::Array { element: Box::new(Type::u8()), length: 64 }),
            ("len", Type::u64()),
        ],
        Type::u8(),
    );
    let a = b.parameter_index(0).unwrap();
    let bb = b.parameter_index(1).unwrap();
    let len = b.parameter_index(2).unwrap();

    let one = b.constant(ConstantData::u64(1), Type::u64(), span());
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), span());
    let x_init = b.constant(ConstantData::u8(0), Type::u8(), span());

    // Body: x |= a[i] ^ b[i]
    let a_byte = b.array_access(a, i_init, Type::u8(), span()).unwrap();
    let b_byte = b.array_access(bb, i_init, Type::u8(), span()).unwrap();
    let xor_val = b.bit_xor(a_byte, b_byte, span()).unwrap();
    let next_x = b.bit_or(x_init, xor_val, span()).unwrap();
    let next_i = b.add(i_init, one, span()).unwrap();
    let cond = b.lt(next_i, len, span()).unwrap();

    let loop_node = b
        .r#loop(
            &[a_byte, b_byte, xor_val, next_x, next_i, cond],
            cond,
            &[next_i, next_x],
            &[i_init, x_init],
            Type::Tuple { elements: vec![Type::u64(), Type::u8()] },
            span(),
        )
        .unwrap();

    // Extract x (second output)
    let x_result = b.tuple_extract(loop_node, 1, Type::u8(), span()).unwrap();
    b.return_value(x_result, span()).unwrap();

    b.build()
}

/// Build a real kernel by corpus ID. Returns None if the ID is unknown.
pub fn build_by_id(id: &str) -> Option<Function> {
    match id {
        "001_redis_bitcount" => Some(build_redis_bitcount_tail()),
        "002_openssl_ct_memcmp" => Some(build_openssl_ct_memcmp()),
        _ => None,
    }
}
