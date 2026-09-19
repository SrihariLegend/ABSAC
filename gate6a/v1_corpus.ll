; ModuleID = 'v1_corpus.c'
source_filename = "v1_corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @v01_count_matches_do_while(ptr nocapture noundef readonly %0, i32 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = tail call i32 @llvm.smax.i32(i32 %1, i32 1)
  %5 = zext nneg i32 %4 to i64
  br label %6

6:                                                ; preds = %6, %3
  %7 = phi i32 [ 0, %3 ], [ %13, %6 ]
  %8 = phi i64 [ 0, %3 ], [ %14, %6 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, %2
  %12 = zext i1 %11 to i32
  %13 = add nuw nsw i32 %7, %12
  %14 = add nuw nsw i64 %8, 1
  %15 = icmp eq i64 %14, %5
  br i1 %15, label %16, label %6, !llvm.loop !8

16:                                               ; preds = %6
  ret i32 %13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @v02_sum_u16(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i32 %1, 0
  br i1 %3, label %6, label %4

4:                                                ; preds = %2
  %5 = zext i32 %1 to i64
  br label %8

6:                                                ; preds = %8, %2
  %7 = phi i32 [ 0, %2 ], [ %14, %8 ]
  ret i32 %7

8:                                                ; preds = %4, %8
  %9 = phi i64 [ 0, %4 ], [ %15, %8 ]
  %10 = phi i32 [ 0, %4 ], [ %14, %8 ]
  %11 = getelementptr inbounds i16, ptr %0, i64 %9
  %12 = load i16, ptr %11, align 2, !tbaa !11
  %13 = zext i16 %12 to i32
  %14 = add i32 %10, %13
  %15 = add nuw nsw i64 %9, 1
  %16 = icmp eq i64 %15, %5
  br i1 %16, label %6, label %8, !llvm.loop !13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @v03_all_same_i64(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 1, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 1, %3 ]
  %10 = getelementptr inbounds i64, ptr %0, i64 %8
  %11 = load i64, ptr %10, align 8, !tbaa !14
  %12 = icmp eq i64 %11, %2
  %13 = select i1 %12, i64 %9, i64 0
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !16
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @v04_count_above_u32(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i32, ptr %0, i64 %8
  %11 = load i32, ptr %10, align 4, !tbaa !17
  %12 = icmp ugt i32 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @v05_sum_signed(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp sgt i64 %1, 0
  br i1 %3, label %6, label %4

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = sext i8 %10 to i64
  %12 = add nsw i64 %8, %11
  %13 = add nuw nsw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !20
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @v06_count_not_equal(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %15, label %5

5:                                                ; preds = %3, %5
  %6 = phi i64 [ %13, %5 ], [ 0, %3 ]
  %7 = phi i64 [ %12, %5 ], [ 0, %3 ]
  %8 = getelementptr inbounds i8, ptr %0, i64 %6
  %9 = load i8, ptr %8, align 1, !tbaa !5
  %10 = icmp ne i8 %9, %2
  %11 = zext i1 %10 to i64
  %12 = add i64 %7, %11
  %13 = add nuw i64 %6, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %15, label %5, !llvm.loop !21

15:                                               ; preds = %5, %3
  %16 = phi i64 [ 0, %3 ], [ %12, %5 ]
  ret i64 %16
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @v07_count_then_sum(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %8

5:                                                ; preds = %8, %3
  %6 = phi i64 [ 0, %3 ], [ %15, %8 ]
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %18, label %21

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %16, %8 ], [ 0, %3 ]
  %10 = phi i64 [ %15, %8 ], [ 0, %3 ]
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = icmp eq i8 %12, %2
  %14 = zext i1 %13 to i64
  %15 = add i64 %10, %14
  %16 = add nuw i64 %9, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %5, label %8, !llvm.loop !22

18:                                               ; preds = %21, %5
  %19 = phi i64 [ 0, %5 ], [ %27, %21 ]
  %20 = add i64 %19, %6
  ret i64 %20

21:                                               ; preds = %5, %21
  %22 = phi i64 [ %28, %21 ], [ 0, %5 ]
  %23 = phi i64 [ %27, %21 ], [ 0, %5 ]
  %24 = getelementptr inbounds i8, ptr %0, i64 %22
  %25 = load i8, ptr %24, align 1, !tbaa !5
  %26 = zext i8 %25 to i64
  %27 = add i64 %23, %26
  %28 = add nuw i64 %22, 1
  %29 = icmp eq i64 %28, %1
  br i1 %29, label %18, label %21, !llvm.loop !23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @v08_sum_bounded(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp ult i64 %2, %1
  br i1 %4, label %7, label %5

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ %2, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = zext i8 %11 to i64
  %13 = add i64 %9, %12
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !24
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define dso_local i64 @n09_atomic_load(ptr noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load volatile i8, ptr %9, align 1, !tbaa !5
  %11 = icmp ne i8 %10, 0
  %12 = zext i1 %11 to i64
  %13 = add i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !25
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: write) uwtable
define dso_local i64 @n10_store_alias(ptr nocapture noundef writeonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = trunc i64 %7 to i8
  %10 = getelementptr inbounds i8, ptr %0, i64 %7
  store i8 %9, ptr %10, align 1, !tbaa !5
  %11 = and i64 %7, 255
  %12 = add i64 %11, %8
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !26
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n11_reverse_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr i8, ptr %0, i64 -1
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %2
  %6 = phi i64 [ 0, %2 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %2, %7
  %8 = phi i64 [ %14, %7 ], [ %1, %2 ]
  %9 = phi i64 [ %13, %7 ], [ 0, %2 ]
  %10 = getelementptr i8, ptr %3, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = zext i8 %11 to i64
  %13 = add i64 %9, %12
  %14 = add i64 %8, -1
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %5, label %7, !llvm.loop !27
}

; Function Attrs: nounwind uwtable
define dso_local i64 @n12_count_with_call(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #3 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %14, %3
  %6 = phi i64 [ 0, %3 ], [ %18, %14 ]
  ret i64 %6

7:                                                ; preds = %3, %14
  %8 = phi i64 [ %19, %14 ], [ 0, %3 ]
  %9 = phi i64 [ %18, %14 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, %2
  br i1 %12, label %13, label %14

13:                                               ; preds = %7
  tail call void @record_hit() #6
  br label %14

14:                                               ; preds = %13, %7
  %15 = load i8, ptr %10, align 1, !tbaa !5
  %16 = icmp eq i8 %15, %2
  %17 = zext i1 %16 to i64
  %18 = add i64 %9, %17
  %19 = add nuw i64 %8, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %5, label %7, !llvm.loop !28
}

declare void @record_hit() local_unnamed_addr #4

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n13_dynamic_stride(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = zext i8 %11 to i64
  %13 = add i64 %9, %12
  %14 = add i64 %8, %2
  %15 = icmp ult i64 %14, %1
  br i1 %15, label %7, label %5, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n14_saturating_count(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %16, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %17, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %16, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, %2
  %13 = icmp ult i64 %9, 1000
  %14 = select i1 %12, i1 %13, i1 false
  %15 = zext i1 %14 to i64
  %16 = add nuw nsw i64 %9, %15
  %17 = add nuw i64 %8, 1
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %5, label %7, !llvm.loop !30
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n15_count_equal_pairs(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %16, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %17, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %16, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = getelementptr inbounds i8, ptr %1, i64 %8
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = icmp eq i8 %11, %13
  %15 = zext i1 %14 to i64
  %16 = add i64 %9, %15
  %17 = add nuw i64 %8, 1
  %18 = icmp eq i64 %17, %2
  br i1 %18, label %5, label %7, !llvm.loop !31
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @n16_sum_signed_nsw(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %2
  %5 = zext nneg i32 %1 to i64
  br label %8

6:                                                ; preds = %8, %2
  %7 = phi i32 [ 0, %2 ], [ %14, %8 ]
  ret i32 %7

8:                                                ; preds = %4, %8
  %9 = phi i64 [ 0, %4 ], [ %15, %8 ]
  %10 = phi i32 [ 0, %4 ], [ %14, %8 ]
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = sext i8 %12 to i32
  %14 = add nsw i32 %10, %13
  %15 = add nuw nsw i64 %9, 1
  %16 = icmp eq i64 %15, %5
  br i1 %16, label %6, label %8, !llvm.loop !32
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smax.i32(i32, i32) #5

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nosync nounwind memory(argmem: write) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #6 = { nounwind }

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
!11 = !{!12, !12, i64 0}
!12 = !{!"short", !6, i64 0}
!13 = distinct !{!13, !9, !10}
!14 = !{!15, !15, i64 0}
!15 = !{!"long", !6, i64 0}
!16 = distinct !{!16, !9, !10}
!17 = !{!18, !18, i64 0}
!18 = !{!"int", !6, i64 0}
!19 = distinct !{!19, !9, !10}
!20 = distinct !{!20, !9, !10}
!21 = distinct !{!21, !9, !10}
!22 = distinct !{!22, !9, !10}
!23 = distinct !{!23, !9, !10}
!24 = distinct !{!24, !9, !10}
!25 = distinct !{!25, !9, !10}
!26 = distinct !{!26, !9, !10}
!27 = distinct !{!27, !9, !10}
!28 = distinct !{!28, !9, !10}
!29 = distinct !{!29, !9, !10}
!30 = distinct !{!30, !9, !10}
!31 = distinct !{!31, !9, !10}
!32 = distinct !{!32, !9, !10}
