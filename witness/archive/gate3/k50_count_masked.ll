define dso_local i64 @k50_count_masked(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %6, label %8

6:                                                ; preds = %8, %4
  %7 = phi i64 [ 0, %4 ], [ %16, %8 ]
  ret i64 %7

8:                                                ; preds = %4, %8
  %9 = phi i64 [ %17, %8 ], [ 0, %4 ]
  %10 = phi i64 [ %16, %8 ], [ 0, %4 ]
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = and i8 %12, %2
  %14 = icmp eq i8 %13, %3
  %15 = zext i1 %14 to i64
  %16 = add i64 %10, %15
  %17 = add nuw i64 %9, 1
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %6, label %8, !llvm.loop !63
}
