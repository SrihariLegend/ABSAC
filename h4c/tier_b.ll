; ModuleID = 'h4c/tier_b.c'
source_filename = "h4c/tier_b.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@h4c_g_a = external local_unnamed_addr global [96 x i8], align 16
@h4c_g_b = external local_unnamed_addr global [160 x i8], align 16
@h4c_g_c = external local_unnamed_addr global [24 x i8], align 16
@h4c_g_s = external local_unnamed_addr global [96 x i8], align 16
@h4c_g_d = external local_unnamed_addr global [96 x i8], align 16

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c01_any_or_global96() local_unnamed_addr #0 {
  br label %4

1:                                                ; preds = %4
  %2 = icmp ne i8 %9, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:                                                ; preds = %0, %4
  %5 = phi i64 [ 0, %0 ], [ %10, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %9, %4 ]
  %7 = getelementptr inbounds [96 x i8], ptr @h4c_g_a, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = or i8 %8, %6
  %10 = add nuw nsw i64 %5, 1
  %11 = icmp eq i64 %10, 96
  br i1 %11, label %1, label %4, !llvm.loop !8
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c02_pred_lt_global160(i8 noundef zeroext %0) local_unnamed_addr #0 {
  br label %5

2:                                                ; preds = %5
  %3 = icmp ne i8 %12, 0
  %4 = zext i1 %3 to i64
  ret i64 %4

5:                                                ; preds = %1, %5
  %6 = phi i64 [ 0, %1 ], [ %13, %5 ]
  %7 = phi i8 [ 0, %1 ], [ %12, %5 ]
  %8 = getelementptr inbounds [160 x i8], ptr @h4c_g_b, i64 0, i64 %6
  %9 = load i8, ptr %8, align 1, !tbaa !5
  %10 = icmp ult i8 %9, %0
  %11 = zext i1 %10 to i8
  %12 = or i8 %7, %11
  %13 = add nuw nsw i64 %6, 1
  %14 = icmp eq i64 %13, 160
  br i1 %14, label %2, label %5, !llvm.loop !11
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c03_pred_gt_const_global24() local_unnamed_addr #0 {
  br label %4

1:                                                ; preds = %4
  %2 = icmp ne i8 %11, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:                                                ; preds = %0, %4
  %5 = phi i64 [ 0, %0 ], [ %12, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %11, %4 ]
  %7 = getelementptr inbounds [24 x i8], ptr @h4c_g_c, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp ugt i8 %8, 64
  %10 = zext i1 %9 to i8
  %11 = or i8 %6, %10
  %12 = add nuw nsw i64 %5, 1
  %13 = icmp eq i64 %12, 24
  br i1 %13, label %1, label %4, !llvm.loop !12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @h4c04_ptr_scan(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %7, label %9

4:                                                ; preds = %9
  %5 = icmp ne i8 %14, 0
  %6 = zext i1 %5 to i64
  br label %7

7:                                                ; preds = %4, %2
  %8 = phi i64 [ 0, %2 ], [ %6, %4 ]
  ret i64 %8

9:                                                ; preds = %2, %9
  %10 = phi i64 [ %15, %9 ], [ 0, %2 ]
  %11 = phi i8 [ %14, %9 ], [ 0, %2 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = or i8 %13, %11
  %15 = add nuw i64 %10, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %9, !llvm.loop !13
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c05_reverse_scan96() local_unnamed_addr #0 {
  br label %4

1:                                                ; preds = %4
  %2 = icmp ne i8 %9, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:                                                ; preds = %0, %4
  %5 = phi i64 [ 95, %0 ], [ %10, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %9, %4 ]
  %7 = getelementptr inbounds [96 x i8], ptr @h4c_g_a, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = or i8 %8, %6
  %10 = add nsw i64 %5, -1
  %11 = icmp eq i64 %5, 0
  br i1 %11, label %1, label %4, !llvm.loop !14
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c06_partial_bound_global96() local_unnamed_addr #0 {
  br label %4

1:                                                ; preds = %4
  %2 = icmp ne i8 %9, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:                                                ; preds = %0, %4
  %5 = phi i64 [ 0, %0 ], [ %10, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %9, %4 ]
  %7 = getelementptr inbounds [96 x i8], ptr @h4c_g_a, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = or i8 %8, %6
  %10 = add nuw nsw i64 %5, 1
  %11 = icmp eq i64 %10, 32
  br i1 %11, label %1, label %4, !llvm.loop !15
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c07_two_loops_diff_extents() local_unnamed_addr #0 {
  br label %1

1:                                                ; preds = %0, %1
  %2 = phi i64 [ 0, %0 ], [ %7, %1 ]
  %3 = phi i8 [ 0, %0 ], [ %6, %1 ]
  %4 = getelementptr inbounds [96 x i8], ptr @h4c_g_a, i64 0, i64 %2
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = or i8 %5, %3
  %7 = add nuw nsw i64 %2, 1
  %8 = icmp eq i64 %7, 96
  br i1 %8, label %14, label %1, !llvm.loop !16

9:                                                ; preds = %14
  %10 = icmp ne i8 %6, 0
  %11 = icmp ne i8 %19, 0
  %12 = select i1 %10, i1 %11, i1 false
  %13 = zext i1 %12 to i64
  ret i64 %13

14:                                               ; preds = %1, %14
  %15 = phi i64 [ %20, %14 ], [ 0, %1 ]
  %16 = phi i8 [ %19, %14 ], [ 0, %1 ]
  %17 = getelementptr inbounds [24 x i8], ptr @h4c_g_c, i64 0, i64 %15
  %18 = load i8, ptr %17, align 1, !tbaa !5
  %19 = or i8 %18, %16
  %20 = add nuw nsw i64 %15, 1
  %21 = icmp eq i64 %20, 24
  br i1 %21, label %9, label %14, !llvm.loop !17
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c08_signed_lt_zero() local_unnamed_addr #0 {
  br label %4

1:                                                ; preds = %4
  %2 = icmp ne i8 %10, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:                                                ; preds = %0, %4
  %5 = phi i64 [ 0, %0 ], [ %11, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %10, %4 ]
  %7 = getelementptr inbounds [96 x i8], ptr @h4c_g_s, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = lshr i8 %8, 7
  %10 = or i8 %9, %6
  %11 = add nuw nsw i64 %5, 1
  %12 = icmp eq i64 %11, 96
  br i1 %12, label %1, label %4, !llvm.loop !18
}

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c09_index_dependent_predicate() local_unnamed_addr #0 {
  br label %4

1:                                                ; preds = %4
  %2 = icmp ne i8 %13, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:                                                ; preds = %0, %4
  %5 = phi i64 [ 0, %0 ], [ %14, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %13, %4 ]
  %7 = getelementptr inbounds [96 x i8], ptr @h4c_g_a, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = zext i8 %8 to i32
  %10 = trunc nuw i64 %5 to i32
  %11 = icmp sgt i32 %9, %10
  %12 = zext i1 %11 to i8
  %13 = or i8 %6, %12
  %14 = add nuw nsw i64 %5, 1
  %15 = icmp eq i64 %14, 96
  br i1 %15, label %1, label %4, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, argmem: none, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @h4c10_store_in_loop(i8 noundef zeroext %0) local_unnamed_addr #2 {
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) @h4c_g_d, i8 %0, i64 96, i1 false), !tbaa !5
  br label %5

2:                                                ; preds = %5
  %3 = icmp ne i8 %10, 0
  %4 = zext i1 %3 to i64
  ret i64 %4

5:                                                ; preds = %1, %5
  %6 = phi i64 [ 0, %1 ], [ %11, %5 ]
  %7 = phi i8 [ 0, %1 ], [ %10, %5 ]
  %8 = getelementptr inbounds [96 x i8], ptr @h4c_g_a, i64 0, i64 %6
  %9 = load i8, ptr %8, align 1, !tbaa !5
  %10 = or i8 %9, %7
  %11 = add nuw nsw i64 %6, 1
  %12 = icmp eq i64 %11, 96
  br i1 %12, label %2, label %5, !llvm.loop !20
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #3

attributes #0 = { nofree norecurse nosync nounwind memory(read, argmem: none, inaccessiblemem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nosync nounwind memory(readwrite, argmem: none, inaccessiblemem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: write) }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"PIE Level", i32 2}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{!"Debian clang version 19.1.7 (3+b1)"}
!5 = !{!6, !6, i64 0}
!6 = !{!"omnipotent char", !7, i64 0}
!7 = !{!"Simple C/C++ TBAA"}
!8 = distinct !{!8, !9, !10}
!9 = !{!"llvm.loop.mustprogress"}
!10 = !{!"llvm.loop.unroll.disable"}
!11 = distinct !{!11, !9, !10}
!12 = distinct !{!12, !9, !10}
!13 = distinct !{!13, !9, !10}
!14 = distinct !{!14, !9, !10}
!15 = distinct !{!15, !9, !10}
!16 = distinct !{!16, !9, !10}
!17 = distinct !{!17, !9, !10}
!18 = distinct !{!18, !9, !10}
!19 = distinct !{!19, !9, !10}
!20 = distinct !{!20, !9, !10}
