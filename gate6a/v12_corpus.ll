; Fresh (post-fix) arithmetic/mask corpus: independently written loop
; variants of the occurrence-recovery cases. Used for native differential
; verification, never as a recognition fixture.

; Four mask-algebra patterns feeding a counted accumulation loop.
define dso_local i32 @m01_merged_mask_u32(i32 %x, i32 %y) {
entry:
  %shared = add i32 %x, %y
  br label %loop

loop:
  %i = phi i32 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i32 [ 0, %entry ], [ %accnext, %loop ]
  %minus1 = sub i32 %shared, 1
  %clear = and i32 %shared, %minus1
  %neg = sub i32 0, %shared
  %isolate = and i32 %shared, %neg
  %plus1 = add i32 %shared, 1
  %nots = xor i32 %shared, -1
  %lowclear = and i32 %nots, %plus1
  %setclear = or i32 %shared, %plus1
  %a = add i32 %clear, %isolate
  %b = add i32 %lowclear, %setclear
  %val = add i32 %a, %b
  %accnext = add i32 %acc, %val
  %inext = add i32 %i, 1
  %cond = icmp eq i32 %inext, 4
  br i1 %cond, label %exit, label %loop

exit:
  ret i32 %accnext
}

; Remainder, division, and shift-mask over one loop-invariant input.
define dso_local i32 @m02_merged_pow2_u32(i32 %x, i32 %y) {
entry:
  %shared = add i32 %x, %y
  br label %loop

loop:
  %i = phi i32 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i32 [ 0, %entry ], [ %accnext, %loop ]
  %rem = urem i32 %shared, 16
  %div = udiv i32 %shared, 8
  %shl = shl i32 %shared, 4
  %sm = lshr i32 %shl, 4
  %a = add i32 %rem, %div
  %b = add i32 %a, %sm
  %accnext = add i32 %acc, %b
  %inext = add i32 %i, 1
  %cond = icmp eq i32 %inext, 4
  br i1 %cond, label %exit, label %loop

exit:
  ret i32 %accnext
}

; Chained power-of-two operations on a loop-varying value.
define dso_local i32 @m03_chained_pow2_u32(i32 %x) {
entry:
  br label %loop

loop:
  %i = phi i32 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i32 [ 0, %entry ], [ %accnext, %loop ]
  %t = add i32 %x, %i
  %a = urem i32 %t, 8
  %b = urem i32 %a, 4
  %c = udiv i32 %b, 2
  %accnext = add i32 %acc, %c
  %inext = add i32 %i, 1
  %cond = icmp eq i32 %inext, 4
  br i1 %cond, label %exit, label %loop

exit:
  ret i32 %accnext
}

; Mask, remainder, and shift-mask sharing one loop-invariant input.
define dso_local i32 @m04_mixed_families_u32(i32 %x, i32 %y) {
entry:
  %shared = add i32 %x, %y
  br label %loop

loop:
  %i = phi i32 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i32 [ 0, %entry ], [ %accnext, %loop ]
  %minus1 = sub i32 %shared, 1
  %clear = and i32 %shared, %minus1
  %rem = urem i32 %shared, 16
  %shl = shl i32 %shared, 3
  %sm = lshr i32 %shl, 3
  %a = add i32 %clear, %rem
  %b = add i32 %a, %sm
  %accnext = add i32 %acc, %b
  %inext = add i32 %i, 1
  %cond = icmp eq i32 %inext, 4
  br i1 %cond, label %exit, label %loop

exit:
  ret i32 %accnext
}

; 64-bit merged mask patterns.
define dso_local i64 @m05_merged_mask_u64(i64 %x, i64 %y) {
entry:
  %shared = add i64 %x, %y
  br label %loop

loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %minus1 = sub i64 %shared, 1
  %clear = and i64 %shared, %minus1
  %neg = sub i64 0, %shared
  %isolate = and i64 %shared, %neg
  %plus1 = add i64 %shared, 1
  %nots = xor i64 %shared, -1
  %lowclear = and i64 %nots, %plus1
  %setclear = or i64 %shared, %plus1
  %a = add i64 %clear, %isolate
  %b = add i64 %lowclear, %setclear
  %val = add i64 %a, %b
  %accnext = add i64 %acc, %val
  %inext = add i64 %i, 1
  %cond = icmp eq i64 %inext, 4
  br i1 %cond, label %exit, label %loop

exit:
  ret i64 %accnext
}

; 16-bit merged mask and remainder.
define dso_local i16 @m06_merged_mask_rem_u16(i16 %x, i16 %y) {
entry:
  %shared = add i16 %x, %y
  br label %loop

loop:
  %i = phi i16 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i16 [ 0, %entry ], [ %accnext, %loop ]
  %minus1 = sub i16 %shared, 1
  %clear = and i16 %shared, %minus1
  %rem = urem i16 %shared, 8
  %val = add i16 %clear, %rem
  %accnext = add i16 %acc, %val
  %inext = add i16 %i, 1
  %cond = icmp eq i16 %inext, 4
  br i1 %cond, label %exit, label %loop

exit:
  ret i16 %accnext
}
