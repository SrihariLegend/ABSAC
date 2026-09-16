; ModuleID = 'gate6b/corpus.c'
source_filename = "gate6b/corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@g6_out = external local_unnamed_addr global [64 x i8], align 16

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6p01_triad(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3, i8 noundef zeroext %4) local_unnamed_addr #0 {
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %10, %5
  %8 = phi i64 [ 0, %5 ], [ %18, %10 ]
  %9 = icmp eq i64 %1, 0
  br i1 %9, label %21, label %24

10:                                               ; preds = %5, %10
  %11 = phi i64 [ %19, %10 ], [ 0, %5 ]
  %12 = phi i64 [ %18, %10 ], [ 0, %5 ]
  %13 = getelementptr inbounds i8, ptr %0, i64 %11
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = and i8 %14, %2
  %16 = icmp eq i8 %15, %3
  %17 = zext i1 %16 to i64
  %18 = add i64 %12, %17
  %19 = add nuw i64 %11, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %7, label %10, !llvm.loop !8

21:                                               ; preds = %24, %7
  %22 = phi i64 [ 0, %7 ], [ %30, %24 ]
  %23 = icmp eq i64 %1, 0
  br i1 %23, label %33, label %37

24:                                               ; preds = %7, %24
  %25 = phi i64 [ %31, %24 ], [ 0, %7 ]
  %26 = phi i64 [ %30, %24 ], [ 0, %7 ]
  %27 = getelementptr inbounds i8, ptr %0, i64 %25
  %28 = load i8, ptr %27, align 1, !tbaa !5
  %29 = zext i8 %28 to i64
  %30 = add i64 %26, %29
  %31 = add nuw i64 %25, 1
  %32 = icmp eq i64 %31, %1
  br i1 %32, label %21, label %24, !llvm.loop !11

33:                                               ; preds = %37, %21
  %34 = phi i64 [ 1, %21 ], [ %43, %37 ]
  %35 = xor i64 %22, %8
  %36 = xor i64 %35, %34
  ret i64 %36

37:                                               ; preds = %21, %37
  %38 = phi i64 [ %44, %37 ], [ 0, %21 ]
  %39 = phi i64 [ %43, %37 ], [ 1, %21 ]
  %40 = getelementptr inbounds i8, ptr %0, i64 %38
  %41 = load i8, ptr %40, align 1, !tbaa !5
  %42 = icmp eq i8 %41, %4
  %43 = select i1 %42, i64 %39, i64 0
  %44 = add nuw i64 %38, 1
  %45 = icmp eq i64 %44, %1
  br i1 %45, label %33, label %37, !llvm.loop !12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6p02_count_sum(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %9, %4
  %7 = phi i64 [ 0, %4 ], [ %17, %9 ]
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %20, label %23

9:                                                ; preds = %4, %9
  %10 = phi i64 [ %18, %9 ], [ 0, %4 ]
  %11 = phi i64 [ %17, %9 ], [ 0, %4 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = and i8 %13, %2
  %15 = icmp eq i8 %14, %3
  %16 = zext i1 %15 to i64
  %17 = add i64 %11, %16
  %18 = add nuw i64 %10, 1
  %19 = icmp eq i64 %18, %1
  br i1 %19, label %6, label %9, !llvm.loop !13

20:                                               ; preds = %23, %6
  %21 = phi i64 [ 0, %6 ], [ %29, %23 ]
  %22 = xor i64 %21, %7
  ret i64 %22

23:                                               ; preds = %6, %23
  %24 = phi i64 [ %30, %23 ], [ 0, %6 ]
  %25 = phi i64 [ %29, %23 ], [ 0, %6 ]
  %26 = getelementptr inbounds i8, ptr %0, i64 %24
  %27 = load i8, ptr %26, align 1, !tbaa !5
  %28 = zext i8 %27 to i64
  %29 = add i64 %25, %28
  %30 = add nuw i64 %24, 1
  %31 = icmp eq i64 %30, %1
  br i1 %31, label %20, label %23, !llvm.loop !14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6p03_sum_all(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %8

5:                                                ; preds = %8, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %8 ]
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %17, label %20

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %15, %8 ], [ 0, %3 ]
  %10 = phi i64 [ %14, %8 ], [ 0, %3 ]
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = zext i8 %12 to i64
  %14 = add i64 %10, %13
  %15 = add nuw i64 %9, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %8, !llvm.loop !15

17:                                               ; preds = %20, %5
  %18 = phi i64 [ 1, %5 ], [ %26, %20 ]
  %19 = xor i64 %18, %6
  ret i64 %19

20:                                               ; preds = %5, %20
  %21 = phi i64 [ %27, %20 ], [ 0, %5 ]
  %22 = phi i64 [ %26, %20 ], [ 1, %5 ]
  %23 = getelementptr inbounds i8, ptr %0, i64 %21
  %24 = load i8, ptr %23, align 1, !tbaa !5
  %25 = icmp eq i8 %24, %2
  %26 = select i1 %25, i64 %22, i64 0
  %27 = add nuw i64 %21, 1
  %28 = icmp eq i64 %27, %1
  br i1 %28, label %17, label %20, !llvm.loop !16
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6p04_two_predicates(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3, i8 noundef zeroext %4) local_unnamed_addr #0 {
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %10, %5
  %8 = phi i64 [ 0, %5 ], [ %18, %10 ]
  %9 = icmp eq i64 %1, 0
  br i1 %9, label %21, label %24

10:                                               ; preds = %5, %10
  %11 = phi i64 [ %19, %10 ], [ 0, %5 ]
  %12 = phi i64 [ %18, %10 ], [ 0, %5 ]
  %13 = getelementptr inbounds i8, ptr %0, i64 %11
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = and i8 %14, %2
  %16 = icmp eq i8 %15, %3
  %17 = zext i1 %16 to i64
  %18 = add i64 %12, %17
  %19 = add nuw i64 %11, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %7, label %10, !llvm.loop !17

21:                                               ; preds = %24, %7
  %22 = phi i64 [ 0, %7 ], [ %32, %24 ]
  %23 = icmp eq i64 %1, 0
  br i1 %23, label %35, label %39

24:                                               ; preds = %7, %24
  %25 = phi i64 [ %33, %24 ], [ 0, %7 ]
  %26 = phi i64 [ %32, %24 ], [ 0, %7 ]
  %27 = getelementptr inbounds i8, ptr %0, i64 %25
  %28 = load i8, ptr %27, align 1, !tbaa !5
  %29 = and i8 %28, %2
  %30 = icmp eq i8 %29, %4
  %31 = zext i1 %30 to i64
  %32 = add i64 %26, %31
  %33 = add nuw i64 %25, 1
  %34 = icmp eq i64 %33, %1
  br i1 %34, label %21, label %24, !llvm.loop !18

35:                                               ; preds = %39, %21
  %36 = phi i64 [ 0, %21 ], [ %45, %39 ]
  %37 = xor i64 %22, %8
  %38 = xor i64 %37, %36
  ret i64 %38

39:                                               ; preds = %21, %39
  %40 = phi i64 [ %46, %39 ], [ 0, %21 ]
  %41 = phi i64 [ %45, %39 ], [ 0, %21 ]
  %42 = getelementptr inbounds i8, ptr %0, i64 %40
  %43 = load i8, ptr %42, align 1, !tbaa !5
  %44 = zext i8 %43 to i64
  %45 = add i64 %41, %44
  %46 = add nuw i64 %40, 1
  %47 = icmp eq i64 %46, %1
  br i1 %47, label %35, label %39, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6p05_literals(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %7

4:                                                ; preds = %7, %2
  %5 = phi i64 [ 0, %2 ], [ %15, %7 ]
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %18, label %21

7:                                                ; preds = %2, %7
  %8 = phi i64 [ %16, %7 ], [ 0, %2 ]
  %9 = phi i64 [ %15, %7 ], [ 0, %2 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = and i8 %11, 15
  %13 = icmp eq i8 %12, 5
  %14 = zext i1 %13 to i64
  %15 = add i64 %9, %14
  %16 = add nuw i64 %8, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %4, label %7, !llvm.loop !20

18:                                               ; preds = %21, %4
  %19 = phi i64 [ 0, %4 ], [ %27, %21 ]
  %20 = icmp eq i64 %1, 0
  br i1 %20, label %30, label %34

21:                                               ; preds = %4, %21
  %22 = phi i64 [ %28, %21 ], [ 0, %4 ]
  %23 = phi i64 [ %27, %21 ], [ 0, %4 ]
  %24 = getelementptr inbounds i8, ptr %0, i64 %22
  %25 = load i8, ptr %24, align 1, !tbaa !5
  %26 = zext i8 %25 to i64
  %27 = add i64 %23, %26
  %28 = add nuw i64 %22, 1
  %29 = icmp eq i64 %28, %1
  br i1 %29, label %18, label %21, !llvm.loop !21

30:                                               ; preds = %34, %18
  %31 = phi i64 [ 1, %18 ], [ %40, %34 ]
  %32 = xor i64 %19, %5
  %33 = xor i64 %32, %31
  ret i64 %33

34:                                               ; preds = %18, %34
  %35 = phi i64 [ %41, %34 ], [ 0, %18 ]
  %36 = phi i64 [ %40, %34 ], [ 1, %18 ]
  %37 = getelementptr inbounds i8, ptr %0, i64 %35
  %38 = load i8, ptr %37, align 1, !tbaa !5
  %39 = icmp eq i8 %38, 66
  %40 = select i1 %39, i64 %36, i64 0
  %41 = add nuw i64 %35, 1
  %42 = icmp eq i64 %41, %1
  br i1 %42, label %30, label %34, !llvm.loop !22
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6n06_two_buffers(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2, i8 noundef zeroext %3, i8 noundef zeroext %4) local_unnamed_addr #0 {
  %6 = icmp eq i64 %2, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %10, %5
  %8 = phi i64 [ 0, %5 ], [ %18, %10 ]
  %9 = icmp eq i64 %2, 0
  br i1 %9, label %21, label %24

10:                                               ; preds = %5, %10
  %11 = phi i64 [ %19, %10 ], [ 0, %5 ]
  %12 = phi i64 [ %18, %10 ], [ 0, %5 ]
  %13 = getelementptr inbounds i8, ptr %0, i64 %11
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = and i8 %14, %3
  %16 = icmp eq i8 %15, %4
  %17 = zext i1 %16 to i64
  %18 = add i64 %12, %17
  %19 = add nuw i64 %11, 1
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %7, label %10, !llvm.loop !23

21:                                               ; preds = %24, %7
  %22 = phi i64 [ 0, %7 ], [ %30, %24 ]
  %23 = xor i64 %22, %8
  ret i64 %23

24:                                               ; preds = %7, %24
  %25 = phi i64 [ %31, %24 ], [ 0, %7 ]
  %26 = phi i64 [ %30, %24 ], [ 0, %7 ]
  %27 = getelementptr inbounds i8, ptr %1, i64 %25
  %28 = load i8, ptr %27, align 1, !tbaa !5
  %29 = zext i8 %28 to i64
  %30 = add i64 %26, %29
  %31 = add nuw i64 %25, 1
  %32 = icmp eq i64 %31, %2
  br i1 %32, label %21, label %24, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6n07_half_domain(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = lshr i64 %1, 1
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %10, %4
  %8 = phi i64 [ 0, %4 ], [ %18, %10 ]
  %9 = icmp ult i64 %1, 2
  br i1 %9, label %21, label %24

10:                                               ; preds = %4, %10
  %11 = phi i64 [ %19, %10 ], [ 0, %4 ]
  %12 = phi i64 [ %18, %10 ], [ 0, %4 ]
  %13 = getelementptr inbounds i8, ptr %0, i64 %11
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = and i8 %14, %2
  %16 = icmp eq i8 %15, %3
  %17 = zext i1 %16 to i64
  %18 = add i64 %12, %17
  %19 = add nuw i64 %11, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %7, label %10, !llvm.loop !25

21:                                               ; preds = %24, %7
  %22 = phi i64 [ 0, %7 ], [ %30, %24 ]
  %23 = xor i64 %22, %8
  ret i64 %23

24:                                               ; preds = %7, %24
  %25 = phi i64 [ %31, %24 ], [ 0, %7 ]
  %26 = phi i64 [ %30, %24 ], [ 0, %7 ]
  %27 = getelementptr inbounds i8, ptr %0, i64 %25
  %28 = load i8, ptr %27, align 1, !tbaa !5
  %29 = zext i8 %28 to i64
  %30 = add i64 %26, %29
  %31 = add nuw nsw i64 %25, 1
  %32 = icmp eq i64 %31, %5
  br i1 %32, label %21, label %24, !llvm.loop !26
}

; Function Attrs: nofree norecurse nosync nounwind memory(write, argmem: read, inaccessiblemem: none) uwtable
define dso_local i64 @g6n08_store_loop(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #1 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %10

5:                                                ; preds = %10, %3
  %6 = phi i64 [ 0, %3 ], [ %16, %10 ]
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %19, label %8

8:                                                ; preds = %5
  %9 = tail call i64 @llvm.umin.i64(i64 %1, i64 64)
  tail call void @llvm.memset.p0.i64(ptr nonnull align 16 @g6_out, i8 %2, i64 %9, i1 false), !tbaa !5
  br label %19

10:                                               ; preds = %3, %10
  %11 = phi i64 [ %17, %10 ], [ 0, %3 ]
  %12 = phi i64 [ %16, %10 ], [ 0, %3 ]
  %13 = getelementptr inbounds i8, ptr %0, i64 %11
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = zext i8 %14 to i64
  %16 = add i64 %12, %15
  %17 = add nuw i64 %11, 1
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %5, label %10, !llvm.loop !27

19:                                               ; preds = %8, %5
  ret i64 %6
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6n09_reverse(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %9, %4
  %7 = phi i64 [ 0, %4 ], [ %15, %9 ]
  %8 = icmp sgt i64 %1, 0
  br i1 %8, label %21, label %18

9:                                                ; preds = %4, %9
  %10 = phi i64 [ %16, %9 ], [ 0, %4 ]
  %11 = phi i64 [ %15, %9 ], [ 0, %4 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = zext i8 %13 to i64
  %15 = add i64 %11, %14
  %16 = add nuw i64 %10, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %6, label %9, !llvm.loop !28

18:                                               ; preds = %21, %6
  %19 = phi i64 [ 0, %6 ], [ %30, %21 ]
  %20 = xor i64 %19, %7
  ret i64 %20

21:                                               ; preds = %6, %21
  %22 = phi i64 [ %24, %21 ], [ %1, %6 ]
  %23 = phi i64 [ %30, %21 ], [ 0, %6 ]
  %24 = add nsw i64 %22, -1
  %25 = getelementptr inbounds i8, ptr %0, i64 %24
  %26 = load i8, ptr %25, align 1, !tbaa !5
  %27 = and i8 %26, %2
  %28 = icmp eq i8 %27, %3
  %29 = zext i1 %28 to i64
  %30 = add i64 %23, %29
  %31 = icmp sgt i64 %22, 1
  br i1 %31, label %21, label %18, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @g6n10_dependent(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %9, %4
  %7 = phi i64 [ 0, %4 ], [ %17, %9 ]
  %8 = icmp eq i64 %7, 0
  br i1 %8, label %20, label %23

9:                                                ; preds = %4, %9
  %10 = phi i64 [ %18, %9 ], [ 0, %4 ]
  %11 = phi i64 [ %17, %9 ], [ 0, %4 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = and i8 %13, %2
  %15 = icmp eq i8 %14, %3
  %16 = zext i1 %15 to i64
  %17 = add i64 %11, %16
  %18 = add nuw i64 %10, 1
  %19 = icmp eq i64 %18, %1
  br i1 %19, label %6, label %9, !llvm.loop !30

20:                                               ; preds = %23, %6
  %21 = phi i64 [ 0, %6 ], [ %29, %23 ]
  %22 = xor i64 %21, %7
  ret i64 %22

23:                                               ; preds = %6, %23
  %24 = phi i64 [ %30, %23 ], [ 0, %6 ]
  %25 = phi i64 [ %29, %23 ], [ 0, %6 ]
  %26 = getelementptr inbounds i8, ptr %0, i64 %24
  %27 = load i8, ptr %26, align 1, !tbaa !5
  %28 = zext i8 %27 to i64
  %29 = add i64 %25, %28
  %30 = add nuw i64 %24, 1
  %31 = icmp eq i64 %30, %7
  br i1 %31, label %20, label %23, !llvm.loop !31
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #2

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #3

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nosync nounwind memory(write, argmem: read, inaccessiblemem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
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
