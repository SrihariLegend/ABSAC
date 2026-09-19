define dso_local i64 @k43_sum_ascii(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = zext i8 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !56
}
