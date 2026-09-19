; Gate 6A-v22 — Kernighan popcount slice (width 64, contract-backed).

define dso_local i32 @k01_kernighan_popcount_u64(i64 noundef %0) local_unnamed_addr {
  %2 = icmp eq i64 %0, 0
  br i1 %2, label %10, label %3

3:
  %4 = phi i32 [ %8, %3 ], [ 0, %1 ]
  %5 = phi i64 [ %7, %3 ], [ %0, %1 ]
  %6 = add i64 %5, -1
  %7 = and i64 %6, %5
  %8 = add i32 %4, 1
  %9 = icmp eq i64 %7, 0
  br i1 %9, label %10, label %3

10:
  %11 = phi i32 [ 0, %1 ], [ %8, %3 ]
  ret i32 %11
}
