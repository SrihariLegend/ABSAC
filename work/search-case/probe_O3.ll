; ModuleID = 'probe.c'
source_filename = "probe.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @find_first(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp sgt i64 %1, 0
  br i1 %4, label %5, label %13

5:                                                ; preds = %3, %10
  %6 = phi i64 [ %11, %10 ], [ 0, %3 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %6
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp eq i8 %8, %2
  br i1 %9, label %13, label %10

10:                                               ; preds = %5
  %11 = add nuw nsw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5, !llvm.loop !8

13:                                               ; preds = %10, %5, %3
  %14 = phi i64 [ -1, %3 ], [ %6, %5 ], [ -1, %10 ]
  ret i64 %14
}

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }

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
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
