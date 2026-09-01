; ModuleID = 'v2_corpus.c'
source_filename = "v2_corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @w01_count_hits_i16(ptr nocapture noundef readonly %0, i64 noundef %1, i16 noundef signext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i16, ptr %0, i64 %8
  %11 = load i16, ptr %10, align 2, !tbaa !5
  %12 = icmp eq i16 %11, 42
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !9
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @w02_sum_u32(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i32, ptr %0, i64 %7
  %10 = load i32, ptr %9, align 4, !tbaa !12
  %11 = zext i32 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @w03_sum_plus_one(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !15
  %11 = zext i8 %10 to i64
  %12 = add i64 %8, 1
  %13 = add i64 %12, %11
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !16
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @w04_all_equal_i8(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef signext %2) local_unnamed_addr #0 {
  %4 = icmp sgt i64 %1, 0
  br i1 %4, label %7, label %5

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 1, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 1, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !15
  %12 = icmp eq i8 %11, %2
  %13 = select i1 %12, i64 %9, i64 0
  %14 = add nuw nsw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !17
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @w05_count_positive(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i32, ptr %0, i64 %7
  %10 = load i32, ptr %9, align 4, !tbaa !12
  %11 = icmp sgt i32 %10, 0
  %12 = zext i1 %11 to i64
  %13 = add i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !18
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @w06_count_mismatch_const(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %11

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !15
  %9 = icmp ne i8 %8, %1
  %10 = zext i1 %9 to i64
  %11 = add i64 %6, %10
  %12 = add nuw nsw i64 %5, 1
  %13 = icmp eq i64 %12, 256
  br i1 %13, label %3, label %4, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext range(i8 0, 2) i8 @w07_all_min(ptr nocapture noundef readonly %0, i32 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i32 %1, 0
  br i1 %4, label %7, label %5

5:                                                ; preds = %3
  %6 = zext i32 %1 to i64
  br label %9

7:                                                ; preds = %9, %3
  %8 = phi i8 [ 1, %3 ], [ %15, %9 ]
  ret i8 %8

9:                                                ; preds = %5, %9
  %10 = phi i64 [ 0, %5 ], [ %16, %9 ]
  %11 = phi i8 [ 1, %5 ], [ %15, %9 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !15
  %14 = icmp ult i8 %13, %2
  %15 = select i1 %14, i8 0, i8 %11
  %16 = add nuw nsw i64 %10, 1
  %17 = icmp eq i64 %16, %6
  br i1 %17, label %7, label %9, !llvm.loop !20
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @w08_two_reductions(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
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
  %12 = load i8, ptr %11, align 1, !tbaa !15
  %13 = icmp eq i8 %12, %2
  %14 = zext i1 %13 to i64
  %15 = add i64 %10, %14
  %16 = add nuw i64 %9, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %5, label %8, !llvm.loop !21

18:                                               ; preds = %21, %5
  %19 = phi i64 [ 0, %5 ], [ %27, %21 ]
  %20 = add i64 %19, %6
  ret i64 %20

21:                                               ; preds = %5, %21
  %22 = phi i64 [ %28, %21 ], [ 0, %5 ]
  %23 = phi i64 [ %27, %21 ], [ 0, %5 ]
  %24 = getelementptr inbounds i8, ptr %0, i64 %22
  %25 = load i8, ptr %24, align 1, !tbaa !15
  %26 = zext i8 %25 to i64
  %27 = add i64 %23, %26
  %28 = add nuw i64 %22, 1
  %29 = icmp eq i64 %28, %1
  br i1 %29, label %18, label %21, !llvm.loop !22
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define dso_local noundef i64 @x01_volatile_store(ptr noundef %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #1 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %1, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %10, %7 ], [ 0, %3 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  store volatile i8 %2, ptr %9, align 1, !tbaa !15
  %10 = add nuw i64 %8, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %5, label %7, !llvm.loop !23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @x02_sum_i32_nsw(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %2
  %5 = zext nneg i32 %1 to i64
  br label %8

6:                                                ; preds = %8, %2
  %7 = phi i32 [ 0, %2 ], [ %13, %8 ]
  ret i32 %7

8:                                                ; preds = %4, %8
  %9 = phi i64 [ 0, %4 ], [ %14, %8 ]
  %10 = phi i32 [ 0, %4 ], [ %13, %8 ]
  %11 = getelementptr inbounds i32, ptr %0, i64 %9
  %12 = load i32, ptr %11, align 4, !tbaa !12
  %13 = add nsw i32 %12, %10
  %14 = add nuw nsw i64 %9, 1
  %15 = icmp eq i64 %14, %5
  br i1 %15, label %6, label %8, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @x03_sliding_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp sgt i64 %1, 1
  br i1 %3, label %6, label %4

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %16, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %17, %6 ], [ 1, %2 ]
  %8 = phi i64 [ %16, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !15
  %11 = zext i8 %10 to i64
  %12 = getelementptr i8, ptr %9, i64 -1
  %13 = load i8, ptr %12, align 1, !tbaa !15
  %14 = zext i8 %13 to i64
  %15 = add nuw nsw i64 %8, %11
  %16 = add nuw nsw i64 %15, %14
  %17 = add nuw nsw i64 %7, 1
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %4, label %6, !llvm.loop !25
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @x04_stride4(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !15
  %11 = zext i8 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 4
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %6, label %4, !llvm.loop !26
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite) uwtable
define dso_local i64 @x05_atomic_flags(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load atomic i8, ptr %9 acquire, align 1
  %11 = icmp ne i8 %10, 0
  %12 = zext i1 %11 to i64
  %13 = add i64 %7, %12
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !27
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @x06_running_max(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ 0, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !15
  %11 = tail call i8 @llvm.umax.i8(i8 %10, i8 %8)
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !28
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @x07_self_recurrence(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 1, %2 ], [ %14, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %15, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %14, %6 ], [ 1, %2 ]
  %9 = urem i64 %8, 3
  %10 = add i64 %9, %8
  %11 = getelementptr inbounds i8, ptr %0, i64 %7
  %12 = load i8, ptr %11, align 1, !tbaa !15
  %13 = zext i8 %12 to i64
  %14 = add i64 %10, %13
  %15 = add nuw i64 %7, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %6, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @x08_early_exit_write(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3, %10
  %6 = phi i64 [ %11, %10 ], [ 0, %3 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %6
  %8 = load i8, ptr %7, align 1, !tbaa !15
  %9 = icmp eq i8 %8, %2
  br i1 %9, label %13, label %10

10:                                               ; preds = %5
  %11 = add nuw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5, !llvm.loop !30

13:                                               ; preds = %10, %5, %3
  %14 = phi i64 [ 0, %3 ], [ %1, %10 ], [ %6, %5 ]
  %15 = tail call i64 @llvm.umin.i64(i64 %14, i64 %1)
  ret i64 %15
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.umax.i8(i8, i8) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #3

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"PIE Level", i32 2}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{!"Debian clang version 19.1.7 (3+b1)"}
!5 = !{!6, !6, i64 0}
!6 = !{!"short", !7, i64 0}
!7 = !{!"omnipotent char", !8, i64 0}
!8 = !{!"Simple C/C++ TBAA"}
!9 = distinct !{!9, !10, !11}
!10 = !{!"llvm.loop.mustprogress"}
!11 = !{!"llvm.loop.unroll.disable"}
!12 = !{!13, !13, i64 0}
!13 = !{!"int", !7, i64 0}
!14 = distinct !{!14, !10, !11}
!15 = !{!7, !7, i64 0}
!16 = distinct !{!16, !10, !11}
!17 = distinct !{!17, !10, !11}
!18 = distinct !{!18, !10, !11}
!19 = distinct !{!19, !10, !11}
!20 = distinct !{!20, !10, !11}
!21 = distinct !{!21, !10, !11}
!22 = distinct !{!22, !10, !11}
!23 = distinct !{!23, !10, !11}
!24 = distinct !{!24, !10, !11}
!25 = distinct !{!25, !10, !11}
!26 = distinct !{!26, !10, !11}
!27 = distinct !{!27, !10, !11}
!28 = distinct !{!28, !10, !11}
!29 = distinct !{!29, !10, !11}
!30 = distinct !{!30, !10, !11}
