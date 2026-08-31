; ModuleID = '/tmp/orig_kernels.c'
source_filename = "/tmp/orig_kernels.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k18_all_equal(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %47, label %5

5:                                                ; preds = %3
  %6 = icmp ult i64 %1, 16
  br i1 %6, label %7, label %10

7:                                                ; preds = %38, %5
  %8 = phi i64 [ 0, %5 ], [ %11, %38 ]
  %9 = phi i64 [ 1, %5 ], [ %45, %38 ]
  br label %49

10:                                               ; preds = %5
  %11 = and i64 %1, -16
  %12 = insertelement <4 x i8> poison, i8 %2, i64 0
  %13 = shufflevector <4 x i8> %12, <4 x i8> poison, <4 x i32> zeroinitializer
  br label %14

14:                                               ; preds = %14, %10
  %15 = phi i64 [ 0, %10 ], [ %36, %14 ]
  %16 = phi <4 x i1> [ zeroinitializer, %10 ], [ %32, %14 ]
  %17 = phi <4 x i1> [ zeroinitializer, %10 ], [ %33, %14 ]
  %18 = phi <4 x i1> [ zeroinitializer, %10 ], [ %34, %14 ]
  %19 = phi <4 x i1> [ zeroinitializer, %10 ], [ %35, %14 ]
  %20 = getelementptr inbounds i8, ptr %0, i64 %15
  %21 = getelementptr inbounds i8, ptr %20, i64 4
  %22 = getelementptr inbounds i8, ptr %20, i64 8
  %23 = getelementptr inbounds i8, ptr %20, i64 12
  %24 = load <4 x i8>, ptr %20, align 1, !tbaa !5
  %25 = load <4 x i8>, ptr %21, align 1, !tbaa !5
  %26 = load <4 x i8>, ptr %22, align 1, !tbaa !5
  %27 = load <4 x i8>, ptr %23, align 1, !tbaa !5
  %28 = icmp ne <4 x i8> %24, %13
  %29 = icmp ne <4 x i8> %25, %13
  %30 = icmp ne <4 x i8> %26, %13
  %31 = icmp ne <4 x i8> %27, %13
  %32 = or <4 x i1> %16, %28
  %33 = or <4 x i1> %17, %29
  %34 = or <4 x i1> %18, %30
  %35 = or <4 x i1> %19, %31
  %36 = add nuw i64 %15, 16
  %37 = icmp eq i64 %36, %11
  br i1 %37, label %38, label %14, !llvm.loop !8

38:                                               ; preds = %14
  %39 = or <4 x i1> %33, %32
  %40 = or <4 x i1> %34, %39
  %41 = or <4 x i1> %35, %40
  %42 = freeze <4 x i1> %41
  %43 = bitcast <4 x i1> %42 to i4
  %44 = icmp eq i4 %43, 0
  %45 = zext i1 %44 to i64
  %46 = icmp eq i64 %11, %1
  br i1 %46, label %47, label %7

47:                                               ; preds = %49, %38, %3
  %48 = phi i64 [ 1, %3 ], [ %45, %38 ], [ %55, %49 ]
  ret i64 %48

49:                                               ; preds = %7, %49
  %50 = phi i64 [ %56, %49 ], [ %8, %7 ]
  %51 = phi i64 [ %55, %49 ], [ %9, %7 ]
  %52 = getelementptr inbounds i8, ptr %0, i64 %50
  %53 = load i8, ptr %52, align 1, !tbaa !5
  %54 = icmp eq i8 %53, %2
  %55 = select i1 %54, i64 %51, i64 0
  %56 = add nuw i64 %50, 1
  %57 = icmp eq i64 %56, %1
  br i1 %57, label %47, label %49, !llvm.loop !12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k43_sum_ascii(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %41, label %4

4:                                                ; preds = %2
  %5 = icmp ult i64 %1, 16
  br i1 %5, label %6, label %9

6:                                                ; preds = %35, %4
  %7 = phi i64 [ 0, %4 ], [ %10, %35 ]
  %8 = phi i64 [ 0, %4 ], [ %39, %35 ]
  br label %43

9:                                                ; preds = %4
  %10 = and i64 %1, -16
  br label %11

11:                                               ; preds = %11, %9
  %12 = phi i64 [ 0, %9 ], [ %33, %11 ]
  %13 = phi <4 x i64> [ zeroinitializer, %9 ], [ %29, %11 ]
  %14 = phi <4 x i64> [ zeroinitializer, %9 ], [ %30, %11 ]
  %15 = phi <4 x i64> [ zeroinitializer, %9 ], [ %31, %11 ]
  %16 = phi <4 x i64> [ zeroinitializer, %9 ], [ %32, %11 ]
  %17 = getelementptr inbounds i8, ptr %0, i64 %12
  %18 = getelementptr inbounds i8, ptr %17, i64 4
  %19 = getelementptr inbounds i8, ptr %17, i64 8
  %20 = getelementptr inbounds i8, ptr %17, i64 12
  %21 = load <4 x i8>, ptr %17, align 1, !tbaa !5
  %22 = load <4 x i8>, ptr %18, align 1, !tbaa !5
  %23 = load <4 x i8>, ptr %19, align 1, !tbaa !5
  %24 = load <4 x i8>, ptr %20, align 1, !tbaa !5
  %25 = zext <4 x i8> %21 to <4 x i64>
  %26 = zext <4 x i8> %22 to <4 x i64>
  %27 = zext <4 x i8> %23 to <4 x i64>
  %28 = zext <4 x i8> %24 to <4 x i64>
  %29 = add <4 x i64> %13, %25
  %30 = add <4 x i64> %14, %26
  %31 = add <4 x i64> %15, %27
  %32 = add <4 x i64> %16, %28
  %33 = add nuw i64 %12, 16
  %34 = icmp eq i64 %33, %10
  br i1 %34, label %35, label %11, !llvm.loop !13

35:                                               ; preds = %11
  %36 = add <4 x i64> %30, %29
  %37 = add <4 x i64> %31, %36
  %38 = add <4 x i64> %32, %37
  %39 = tail call i64 @llvm.vector.reduce.add.v4i64(<4 x i64> %38)
  %40 = icmp eq i64 %10, %1
  br i1 %40, label %41, label %6

41:                                               ; preds = %43, %35, %2
  %42 = phi i64 [ 0, %2 ], [ %39, %35 ], [ %49, %43 ]
  ret i64 %42

43:                                               ; preds = %6, %43
  %44 = phi i64 [ %50, %43 ], [ %7, %6 ]
  %45 = phi i64 [ %49, %43 ], [ %8, %6 ]
  %46 = getelementptr inbounds i8, ptr %0, i64 %44
  %47 = load i8, ptr %46, align 1, !tbaa !5
  %48 = zext i8 %47 to i64
  %49 = add i64 %45, %48
  %50 = add nuw i64 %44, 1
  %51 = icmp eq i64 %50, %1
  br i1 %51, label %41, label %43, !llvm.loop !14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k50_count_masked(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %55, label %6

6:                                                ; preds = %4
  %7 = icmp ult i64 %1, 16
  br i1 %7, label %8, label %11

8:                                                ; preds = %49, %6
  %9 = phi i64 [ 0, %6 ], [ %12, %49 ]
  %10 = phi i64 [ 0, %6 ], [ %53, %49 ]
  br label %57

11:                                               ; preds = %6
  %12 = and i64 %1, -16
  %13 = insertelement <4 x i8> poison, i8 %2, i64 0
  %14 = shufflevector <4 x i8> %13, <4 x i8> poison, <4 x i32> zeroinitializer
  %15 = insertelement <4 x i8> poison, i8 %3, i64 0
  %16 = shufflevector <4 x i8> %15, <4 x i8> poison, <4 x i32> zeroinitializer
  br label %17

17:                                               ; preds = %17, %11
  %18 = phi i64 [ 0, %11 ], [ %47, %17 ]
  %19 = phi <4 x i64> [ zeroinitializer, %11 ], [ %43, %17 ]
  %20 = phi <4 x i64> [ zeroinitializer, %11 ], [ %44, %17 ]
  %21 = phi <4 x i64> [ zeroinitializer, %11 ], [ %45, %17 ]
  %22 = phi <4 x i64> [ zeroinitializer, %11 ], [ %46, %17 ]
  %23 = getelementptr inbounds i8, ptr %0, i64 %18
  %24 = getelementptr inbounds i8, ptr %23, i64 4
  %25 = getelementptr inbounds i8, ptr %23, i64 8
  %26 = getelementptr inbounds i8, ptr %23, i64 12
  %27 = load <4 x i8>, ptr %23, align 1, !tbaa !5
  %28 = load <4 x i8>, ptr %24, align 1, !tbaa !5
  %29 = load <4 x i8>, ptr %25, align 1, !tbaa !5
  %30 = load <4 x i8>, ptr %26, align 1, !tbaa !5
  %31 = and <4 x i8> %27, %14
  %32 = and <4 x i8> %28, %14
  %33 = and <4 x i8> %29, %14
  %34 = and <4 x i8> %30, %14
  %35 = icmp eq <4 x i8> %31, %16
  %36 = icmp eq <4 x i8> %32, %16
  %37 = icmp eq <4 x i8> %33, %16
  %38 = icmp eq <4 x i8> %34, %16
  %39 = zext <4 x i1> %35 to <4 x i64>
  %40 = zext <4 x i1> %36 to <4 x i64>
  %41 = zext <4 x i1> %37 to <4 x i64>
  %42 = zext <4 x i1> %38 to <4 x i64>
  %43 = add <4 x i64> %19, %39
  %44 = add <4 x i64> %20, %40
  %45 = add <4 x i64> %21, %41
  %46 = add <4 x i64> %22, %42
  %47 = add nuw i64 %18, 16
  %48 = icmp eq i64 %47, %12
  br i1 %48, label %49, label %17, !llvm.loop !15

49:                                               ; preds = %17
  %50 = add <4 x i64> %44, %43
  %51 = add <4 x i64> %45, %50
  %52 = add <4 x i64> %46, %51
  %53 = tail call i64 @llvm.vector.reduce.add.v4i64(<4 x i64> %52)
  %54 = icmp eq i64 %12, %1
  br i1 %54, label %55, label %8

55:                                               ; preds = %57, %49, %4
  %56 = phi i64 [ 0, %4 ], [ %53, %49 ], [ %65, %57 ]
  ret i64 %56

57:                                               ; preds = %8, %57
  %58 = phi i64 [ %66, %57 ], [ %9, %8 ]
  %59 = phi i64 [ %65, %57 ], [ %10, %8 ]
  %60 = getelementptr inbounds i8, ptr %0, i64 %58
  %61 = load i8, ptr %60, align 1, !tbaa !5
  %62 = and i8 %61, %2
  %63 = icmp eq i8 %62, %3
  %64 = zext i1 %63 to i64
  %65 = add i64 %59, %64
  %66 = add nuw i64 %58, 1
  %67 = icmp eq i64 %66, %1
  br i1 %67, label %55, label %57, !llvm.loop !16
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.vector.reduce.add.v4i64(<4 x i64>) #1

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="arrowlake-s" "target-features"="+64bit,+adx,+aes,+avx,+avx2,+avxifma,+avxneconvert,+avxvnni,+avxvnniint16,+avxvnniint8,+bmi,+bmi2,+clflushopt,+clwb,+cmov,+cmpccxadd,+crc32,+cx16,+cx8,+f16c,+fma,+fsgsbase,+fxsr,+gfni,+invpcid,+lzcnt,+mmx,+movbe,+movdir64b,+movdiri,+pclmul,+popcnt,+prfchw,+rdpid,+rdrnd,+rdseed,+sahf,+serialize,+sha,+sha512,+shstk,+sm3,+sm4,+sse,+sse2,+sse3,+sse4.1,+sse4.2,+ssse3,+vaes,+vpclmulqdq,+waitpkg,+x87,+xsave,+xsavec,+xsaveopt,+xsaves,-amx-bf16,-amx-complex,-amx-fp16,-amx-int8,-amx-tile,-avx10.1-256,-avx10.1-512,-avx512bf16,-avx512bitalg,-avx512bw,-avx512cd,-avx512dq,-avx512f,-avx512fp16,-avx512ifma,-avx512vbmi,-avx512vbmi2,-avx512vl,-avx512vnni,-avx512vp2intersect,-avx512vpopcntdq,-ccmp,-cf,-cldemote,-clzero,-egpr,-enqcmd,-fma4,-hreset,-kl,-lwp,-mwaitx,-ndd,-pconfig,-pku,-ppx,-prefetchi,-ptwrite,-push2pop2,-raoint,-rdpru,-rtm,-sgx,-sse4a,-tbm,-tsxldtrk,-uintr,-usermsr,-wbnoinvd,-widekl,-xop" }
attributes #1 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

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
!8 = distinct !{!8, !9, !10, !11}
!9 = !{!"llvm.loop.mustprogress"}
!10 = !{!"llvm.loop.isvectorized", i32 1}
!11 = !{!"llvm.loop.unroll.runtime.disable"}
!12 = distinct !{!12, !9, !11, !10}
!13 = distinct !{!13, !9, !10, !11}
!14 = distinct !{!14, !9, !11, !10}
!15 = distinct !{!15, !9, !10, !11}
!16 = distinct !{!16, !9, !11, !10}
