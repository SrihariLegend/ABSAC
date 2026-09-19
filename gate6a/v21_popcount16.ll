; Gate 6A-v21 — Kernighan popcount slice (width 16).
; The loop-to-popcount identity is machine-checked by bounded unrolling
; for widths <= 16; wider widths await the kernel induction proof.

define dso_local i32 @k01_kernighan_popcount_u16(i16 noundef %0) local_unnamed_addr {
  %2 = icmp eq i16 %0, 0
  br i1 %2, label %10, label %3

3:
  %4 = phi i32 [ %8, %3 ], [ 0, %1 ]
  %5 = phi i16 [ %7, %3 ], [ %0, %1 ]
  %6 = add i16 %5, -1
  %7 = and i16 %6, %5
  %8 = add i32 %4, 1
  %9 = icmp eq i16 %7, 0
  br i1 %9, label %10, label %3

10:
  %11 = phi i32 [ 0, %1 ], [ %8, %3 ]
  ret i32 %11
}
