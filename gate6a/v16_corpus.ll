; ModuleID = 'gate6a/v16_corpus.c'
source_filename = "gate6a/v16_corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @r01_runtime_first_zero_guard(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %15, label %4

4:                                                ; preds = %2, %9
  %5 = phi i64 [ %10, %9 ], [ 0, %2 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, 0
  br i1 %8, label %12, label %9

9:                                                ; preds = %4
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %12, label %4, !llvm.loop !8

12:                                               ; preds = %4, %9
  %13 = phi i64 [ %5, %4 ], [ %1, %9 ]
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %1)
  br label %15

15:                                               ; preds = %12, %2
  %16 = phi i64 [ 0, %2 ], [ %14, %12 ]
  ret i64 %16
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @r02_runtime_first_ne_key_u32(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3, %10
  %6 = phi i64 [ %11, %10 ], [ 0, %3 ]
  %7 = getelementptr inbounds i32, ptr %0, i64 %6
  %8 = load i32, ptr %7, align 4, !tbaa !11
  %9 = icmp eq i32 %8, %2
  br i1 %9, label %10, label %13

10:                                               ; preds = %5
  %11 = add nuw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5, !llvm.loop !13

13:                                               ; preds = %10, %5, %3
  %14 = phi i64 [ 0, %3 ], [ %1, %10 ], [ %6, %5 ]
  %15 = tail call i64 @llvm.umin.i64(i64 %14, i64 %1)
  ret i64 %15
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @r03_runtime_last_eq_key_u16(ptr nocapture noundef readonly %0, i64 noundef %1, i16 noundef zeroext %2) local_unnamed_addr #0 {
  br label %4

4:                                                ; preds = %7, %3
  %5 = phi i64 [ %1, %3 ], [ %8, %7 ]
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %12, label %7

7:                                                ; preds = %4
  %8 = add i64 %5, -1
  %9 = getelementptr inbounds i16, ptr %0, i64 %8
  %10 = load i16, ptr %9, align 2, !tbaa !14
  %11 = icmp eq i16 %10, %2
  br i1 %11, label %12, label %4, !llvm.loop !16

12:                                               ; preds = %7, %4
  %13 = phi i64 [ %1, %4 ], [ %8, %7 ]
  ret i64 %13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @r04_runtime_first_ge_key_u16(ptr nocapture noundef readonly %0, i64 noundef %1, i16 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3, %10
  %6 = phi i64 [ %11, %10 ], [ 0, %3 ]
  %7 = getelementptr inbounds i16, ptr %0, i64 %6
  %8 = load i16, ptr %7, align 2, !tbaa !14
  %9 = icmp ult i16 %8, %2
  br i1 %9, label %10, label %13

10:                                               ; preds = %5
  %11 = add nuw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5, !llvm.loop !17

13:                                               ; preds = %10, %5, %3
  %14 = phi i64 [ 0, %3 ], [ %1, %10 ], [ %6, %5 ]
  %15 = tail call i64 @llvm.umin.i64(i64 %14, i64 %1)
  ret i64 %15
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @s01_sum_u16_64(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %6 = getelementptr inbounds i16, ptr %0, i64 %4
  %7 = load i16, ptr %6, align 2, !tbaa !14
  %8 = zext i16 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 1
  %11 = icmp eq i64 %10, 64
  br i1 %11, label %2, label %3, !llvm.loop !18
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @s02_xor_map_sum_i16_64(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %10

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %11, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %6 = getelementptr inbounds i16, ptr %0, i64 %4
  %7 = load i16, ptr %6, align 2, !tbaa !14
  %8 = xor i16 %7, 90
  %9 = zext i16 %8 to i64
  %10 = add i64 %5, %9
  %11 = add nuw nsw i64 %4, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %2, label %3, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @s03_cond_sum_u8_64(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %12

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %13, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %12, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = and i8 %7, 1
  %9 = icmp eq i8 %8, 0
  %10 = select i1 %9, i8 0, i8 %7
  %11 = zext i8 %10 to i64
  %12 = add i64 %5, %11
  %13 = add nuw nsw i64 %4, 1
  %14 = icmp eq i64 %13, 64
  br i1 %14, label %2, label %3, !llvm.loop !20
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @s04_sum_u32_32(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %6 = getelementptr inbounds i32, ptr %0, i64 %4
  %7 = load i32, ptr %6, align 4, !tbaa !11
  %8 = zext i32 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 1
  %11 = icmp eq i64 %10, 32
  br i1 %11, label %2, label %3, !llvm.loop !21
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i32 @p01_rotl13_u32(i32 noundef %0) local_unnamed_addr #1 {
  %2 = tail call i32 @llvm.fshl.i32(i32 %0, i32 %0, i32 13)
  ret i32 %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i32 @p02_bswap_u32(i32 noundef %0) local_unnamed_addr #1 {
  %2 = tail call i32 @llvm.bswap.i32(i32 %0)
  ret i32 %2
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.bswap.i32(i32) #2

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i32 @p03_bitrev_u32(i32 noundef %0) local_unnamed_addr #1 {
  %2 = tail call i32 @llvm.bitreverse.i32(i32 %0)
  ret i32 %2
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @c01_search_then_sum(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %3

3:                                                ; preds = %2, %11
  %4 = phi i64 [ 0, %2 ], [ %12, %11 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !5
  %7 = icmp eq i8 %6, %1
  br i1 %7, label %8, label %11

8:                                                ; preds = %3
  %9 = mul nuw nsw i64 %4, 1000
  %10 = add nuw nsw i64 %9, 1000
  br label %14

11:                                               ; preds = %3
  %12 = add nuw nsw i64 %4, 1
  %13 = icmp eq i64 %12, 48
  br i1 %13, label %14, label %3, !llvm.loop !22

14:                                               ; preds = %11, %8
  %15 = phi i64 [ %10, %8 ], [ 0, %11 ]
  br label %18

16:                                               ; preds = %18
  %17 = add i64 %24, %15
  ret i64 %17

18:                                               ; preds = %14, %18
  %19 = phi i64 [ 0, %14 ], [ %25, %18 ]
  %20 = phi i64 [ 0, %14 ], [ %24, %18 ]
  %21 = getelementptr inbounds i8, ptr %0, i64 %19
  %22 = load i8, ptr %21, align 1, !tbaa !5
  %23 = zext i8 %22 to i64
  %24 = add i64 %20, %23
  %25 = add nuw nsw i64 %19, 1
  %26 = icmp eq i64 %25, 48
  br i1 %26, label %16, label %18, !llvm.loop !23
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.fshl.i32(i32, i32, i32) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.bitreverse.i32(i32) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #3

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #3 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

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
!12 = !{!"int", !6, i64 0}
!13 = distinct !{!13, !9, !10}
!14 = !{!15, !15, i64 0}
!15 = !{!"short", !6, i64 0}
!16 = distinct !{!16, !9, !10}
!17 = distinct !{!17, !9, !10}
!18 = distinct !{!18, !9, !10}
!19 = distinct !{!19, !9, !10}
!20 = distinct !{!20, !9, !10}
!21 = distinct !{!21, !9, !10}
!22 = distinct !{!22, !9, !10}
!23 = distinct !{!23, !9, !10}
