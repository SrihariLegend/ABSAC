; ModuleID = 'corpus/kernels.c'
source_filename = "corpus/kernels.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@popcount_table = internal unnamed_addr constant [256 x i8] c"\00\01\01\02\01\02\02\03\01\02\02\03\02\03\03\04\01\02\02\03\02\03\03\04\02\03\03\04\03\04\04\05\01\02\02\03\02\03\03\04\02\03\03\04\03\04\04\05\02\03\03\04\03\04\04\05\03\04\04\05\04\05\05\06\01\02\02\03\02\03\03\04\02\03\03\04\03\04\04\05\02\03\03\04\03\04\04\05\03\04\04\05\04\05\05\06\02\03\03\04\03\04\04\05\03\04\04\05\04\05\05\06\03\04\04\05\04\05\05\06\04\05\05\06\05\06\06\07\01\02\02\03\02\03\03\04\02\03\03\04\03\04\04\05\02\03\03\04\03\04\04\05\03\04\04\05\04\05\05\06\02\03\03\04\03\04\04\05\03\04\04\05\04\05\05\06\03\04\04\05\04\05\05\06\04\05\05\06\05\06\06\07\02\03\03\04\03\04\04\05\03\04\04\05\04\05\05\06\03\04\04\05\04\05\05\06\04\05\05\06\05\06\06\07\03\04\04\05\04\05\05\06\04\05\05\06\05\06\06\07\04\05\05\06\05\06\06\07\05\06\06\07\06\07\07\08", align 16

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k01_count_bits_and1(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = and i8 %10, 1
  %12 = zext nneg i8 %11 to i64
  %13 = add i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !8
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k02_count_bits_table(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %15, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %16, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %15, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = zext i8 %10 to i64
  %12 = getelementptr inbounds [256 x i8], ptr @popcount_table, i64 0, i64 %11
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = zext i8 %13 to i64
  %15 = add i64 %8, %14
  %16 = add nuw i64 %7, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %4, label %6, !llvm.loop !11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k03_kernighan_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %19, %2
  %5 = phi i64 [ 0, %2 ], [ %22, %19 ]
  ret i64 %5

6:                                                ; preds = %2, %19
  %7 = phi i64 [ %23, %19 ], [ 0, %2 ]
  %8 = phi i64 [ %22, %19 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, 0
  br i1 %11, label %19, label %12

12:                                               ; preds = %6, %12
  %13 = phi i8 [ %15, %12 ], [ 0, %6 ]
  %14 = phi i8 [ %17, %12 ], [ %10, %6 ]
  %15 = add i8 %13, 1
  %16 = add i8 %14, -1
  %17 = and i8 %16, %14
  %18 = icmp eq i8 %17, 0
  br i1 %18, label %19, label %12, !llvm.loop !12

19:                                               ; preds = %12, %6
  %20 = phi i8 [ 0, %6 ], [ %15, %12 ]
  %21 = zext i8 %20 to i64
  %22 = add i64 %8, %21
  %23 = add nuw i64 %7, 1
  %24 = icmp eq i64 %23, %1
  br i1 %24, label %4, label %6, !llvm.loop !13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k04_parity_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %7, label %9

4:                                                ; preds = %9
  %5 = and i8 %14, 1
  %6 = zext nneg i8 %5 to i64
  br label %7

7:                                                ; preds = %4, %2
  %8 = phi i64 [ 0, %2 ], [ %6, %4 ]
  ret i64 %8

9:                                                ; preds = %2, %9
  %10 = phi i64 [ %15, %9 ], [ 0, %2 ]
  %11 = phi i8 [ %14, %9 ], [ 0, %2 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = xor i8 %13, %11
  %15 = add nuw i64 %10, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %9, !llvm.loop !14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k05_count_nonzero(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp ne i8 %10, 0
  %12 = zext i1 %11 to i64
  %13 = add i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !15
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k06_count_pow2(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %15, %2
  %5 = phi i64 [ 0, %2 ], [ %18, %15 ]
  ret i64 %5

6:                                                ; preds = %2, %15
  %7 = phi i64 [ %18, %15 ], [ 0, %2 ]
  %8 = phi i64 [ %19, %15 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, 0
  br i1 %11, label %15, label %12

12:                                               ; preds = %6
  %13 = tail call range(i8 0, 9) i8 @llvm.ctpop.i8(i8 %10)
  %14 = icmp ult i8 %13, 2
  br label %15

15:                                               ; preds = %12, %6
  %16 = phi i1 [ false, %6 ], [ %14, %12 ]
  %17 = zext i1 %16 to i64
  %18 = add i64 %7, %17
  %19 = add nuw i64 %8, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %4, label %6, !llvm.loop !16
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k07_sum_trailing_zeros(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %25, %2
  %5 = phi i64 [ 0, %2 ], [ %27, %25 ]
  ret i64 %5

6:                                                ; preds = %2, %25
  %7 = phi i64 [ %28, %25 ], [ 0, %2 ]
  %8 = phi i64 [ %27, %25 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, 0
  br i1 %11, label %25, label %12

12:                                               ; preds = %6
  %13 = and i8 %10, 1
  %14 = icmp eq i8 %13, 0
  br i1 %14, label %15, label %22

15:                                               ; preds = %12, %15
  %16 = phi i8 [ %18, %15 ], [ 0, %12 ]
  %17 = phi i8 [ %19, %15 ], [ %10, %12 ]
  %18 = add i8 %16, 1
  %19 = lshr exact i8 %17, 1
  %20 = and i8 %17, 2
  %21 = icmp eq i8 %20, 0
  br i1 %21, label %15, label %22, !llvm.loop !17

22:                                               ; preds = %15, %12
  %23 = phi i8 [ 0, %12 ], [ %18, %15 ]
  %24 = zext i8 %23 to i64
  br label %25

25:                                               ; preds = %6, %22
  %26 = phi i64 [ %24, %22 ], [ 8, %6 ]
  %27 = add i64 %8, %26
  %28 = add nuw i64 %7, 1
  %29 = icmp eq i64 %28, %1
  br i1 %29, label %4, label %6, !llvm.loop !18
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k08_sum_leading_zeros(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %23, %2
  %5 = phi i64 [ 0, %2 ], [ %25, %23 ]
  ret i64 %5

6:                                                ; preds = %2, %23
  %7 = phi i64 [ %26, %23 ], [ 0, %2 ]
  %8 = phi i64 [ %25, %23 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, 0
  br i1 %11, label %23, label %12

12:                                               ; preds = %6
  %13 = icmp sgt i8 %10, -1
  br i1 %13, label %14, label %20

14:                                               ; preds = %12, %14
  %15 = phi i8 [ %17, %14 ], [ 0, %12 ]
  %16 = phi i8 [ %18, %14 ], [ %10, %12 ]
  %17 = add i8 %15, 1
  %18 = shl nuw i8 %16, 1
  %19 = icmp sgt i8 %18, -1
  br i1 %19, label %14, label %20, !llvm.loop !19

20:                                               ; preds = %14, %12
  %21 = phi i8 [ 0, %12 ], [ %17, %14 ]
  %22 = zext i8 %21 to i64
  br label %23

23:                                               ; preds = %6, %20
  %24 = phi i64 [ %22, %20 ], [ 8, %6 ]
  %25 = add i64 %8, %24
  %26 = add nuw i64 %7, 1
  %27 = icmp eq i64 %26, %1
  br i1 %27, label %4, label %6, !llvm.loop !20
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k09_any_high_bit(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = lshr i8 %10, 7
  %12 = zext nneg i8 %11 to i64
  %13 = or i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !21
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k10_count_bit3(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %14, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %15, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = lshr i8 %10, 3
  %12 = and i8 %11, 1
  %13 = zext nneg i8 %12 to i64
  %14 = add i64 %8, %13
  %15 = add nuw i64 %7, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %6, !llvm.loop !22
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 256) i64 @k11_ct_memcmp(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %7, label %9

5:                                                ; preds = %9
  %6 = zext i8 %17 to i64
  br label %7

7:                                                ; preds = %5, %3
  %8 = phi i64 [ 0, %3 ], [ %6, %5 ]
  ret i64 %8

9:                                                ; preds = %3, %9
  %10 = phi i64 [ %18, %9 ], [ 0, %3 ]
  %11 = phi i8 [ %17, %9 ], [ 0, %3 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = getelementptr inbounds i8, ptr %1, i64 %10
  %15 = load i8, ptr %14, align 1, !tbaa !5
  %16 = xor i8 %15, %13
  %17 = or i8 %16, %11
  %18 = add nuw i64 %10, 1
  %19 = icmp eq i64 %18, %2
  br i1 %19, label %5, label %9, !llvm.loop !23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k12_count_matches(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
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
  br i1 %18, label %5, label %7, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k13_count_mismatches(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
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
  %14 = icmp ne i8 %11, %13
  %15 = zext i1 %14 to i64
  %16 = add i64 %9, %15
  %17 = add nuw i64 %8, 1
  %18 = icmp eq i64 %17, %2
  br i1 %18, label %5, label %7, !llvm.loop !25
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @k14_min_element(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ -1, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ -1, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = tail call i8 @llvm.umin.i8(i8 %10, i8 %8)
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !26
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @k15_max_element(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ 0, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = tail call i8 @llvm.umax.i8(i8 %10, i8 %8)
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !27
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k16_count_gt(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp ugt i8 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !28
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k17_count_lt(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp ult i8 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k18_all_equal(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 1, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 1, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, %2
  %13 = select i1 %12, i64 %9, i64 0
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !30
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k19_any_equal(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, %2
  %13 = zext i1 %12 to i64
  %14 = or i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !31
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k20_sad(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %19, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %19, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %20, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %9
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = getelementptr inbounds i8, ptr %1, i64 %9
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = icmp ugt i8 %11, %13
  %15 = sub i8 %11, %13
  %16 = sub i8 %13, %11
  %17 = select i1 %14, i8 %15, i8 %16
  %18 = zext i8 %17 to i64
  %19 = add i64 %8, %18
  %20 = add nuw i64 %9, 1
  %21 = icmp eq i64 %20, %2
  br i1 %21, label %5, label %7, !llvm.loop !32
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @k21_xor_checksum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ 0, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = xor i8 %10, %8
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !33
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @k22_add_checksum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ 0, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = add i8 %10, %8
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !34
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @k23_fnv1a(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i32 [ -2128831035, %2 ], [ %13, %6 ]
  ret i32 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i32 [ %13, %6 ], [ -2128831035, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = zext i8 %10 to i32
  %12 = xor i32 %8, %11
  %13 = mul i32 %12, 16777619
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !35
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @k24_djb2(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i32 [ 5381, %2 ], [ %13, %6 ]
  ret i32 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i32 [ %13, %6 ], [ 5381, %2 ]
  %9 = mul i32 %8, 33
  %10 = getelementptr inbounds i8, ptr %0, i64 %7
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = zext i8 %11 to i32
  %13 = add i32 %9, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !36
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k25_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
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
  br i1 %14, label %4, label %6, !llvm.loop !37
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k26_sum_squares(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = zext i8 %10 to i64
  %12 = mul nuw nsw i64 %11, %11
  %13 = add i64 %12, %8
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !38
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k27_weighted_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = zext i8 %10 to i64
  %12 = mul i64 %7, %11
  %13 = add i64 %12, %8
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !39
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @k28_crc_simple(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %13, %2
  %5 = phi i32 [ -1, %2 ], [ %23, %13 ]
  ret i32 %5

6:                                                ; preds = %2, %13
  %7 = phi i64 [ %14, %13 ], [ 0, %2 ]
  %8 = phi i32 [ %23, %13 ], [ -1, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = zext i8 %10 to i32
  %12 = xor i32 %8, %11
  br label %16

13:                                               ; preds = %16
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !40

16:                                               ; preds = %6, %16
  %17 = phi i32 [ 0, %6 ], [ %24, %16 ]
  %18 = phi i32 [ %12, %6 ], [ %23, %16 ]
  %19 = and i32 %18, 1
  %20 = icmp eq i32 %19, 0
  %21 = lshr i32 %18, 1
  %22 = xor i32 %21, -306674912
  %23 = select i1 %20, i32 %21, i32 %22
  %24 = add nuw nsw i32 %17, 1
  %25 = icmp eq i32 %24, 8
  br i1 %25, label %13, label %16, !llvm.loop !41
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k29_count_true(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp ne i8 %10, 0
  %12 = zext i1 %11 to i64
  %13 = add i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !42
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k30_all_true(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 1, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 1, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, 0
  %12 = select i1 %11, i64 0, i64 %8
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !43
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k31_any_true(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp ne i8 %10, 0
  %12 = zext i1 %11 to i64
  %13 = or i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !44
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local void @k32_bool_union(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, ptr nocapture noundef writeonly %2, i64 noundef %3) local_unnamed_addr #1 {
  %5 = icmp eq i64 %3, 0
  br i1 %5, label %6, label %7

6:                                                ; preds = %7, %4
  ret void

7:                                                ; preds = %4, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %4 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = getelementptr inbounds i8, ptr %1, i64 %8
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = or i8 %12, %10
  %14 = getelementptr inbounds i8, ptr %2, i64 %8
  store i8 %13, ptr %14, align 1, !tbaa !5
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %3
  br i1 %16, label %6, label %7, !llvm.loop !45
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local void @k33_bool_intersect(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, ptr nocapture noundef writeonly %2, i64 noundef %3) local_unnamed_addr #1 {
  %5 = icmp eq i64 %3, 0
  br i1 %5, label %6, label %7

6:                                                ; preds = %7, %4
  ret void

7:                                                ; preds = %4, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %4 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = getelementptr inbounds i8, ptr %1, i64 %8
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = and i8 %12, %10
  %14 = getelementptr inbounds i8, ptr %2, i64 %8
  store i8 %13, ptr %14, align 1, !tbaa !5
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %3
  br i1 %16, label %6, label %7, !llvm.loop !46
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local void @k34_bool_xor(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, ptr nocapture noundef writeonly %2, i64 noundef %3) local_unnamed_addr #1 {
  %5 = icmp eq i64 %3, 0
  br i1 %5, label %6, label %7

6:                                                ; preds = %7, %4
  ret void

7:                                                ; preds = %4, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %4 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = getelementptr inbounds i8, ptr %1, i64 %8
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = xor i8 %12, %10
  %14 = getelementptr inbounds i8, ptr %2, i64 %8
  store i8 %13, ptr %14, align 1, !tbaa !5
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %3
  br i1 %16, label %6, label %7, !llvm.loop !47
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k35_count_both_true(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %17, %3
  %6 = phi i64 [ 0, %3 ], [ %20, %17 ]
  ret i64 %6

7:                                                ; preds = %3, %17
  %8 = phi i64 [ %21, %17 ], [ 0, %3 ]
  %9 = phi i64 [ %20, %17 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, 0
  br i1 %12, label %17, label %13

13:                                               ; preds = %7
  %14 = getelementptr inbounds i8, ptr %1, i64 %8
  %15 = load i8, ptr %14, align 1, !tbaa !5
  %16 = icmp ne i8 %15, 0
  br label %17

17:                                               ; preds = %13, %7
  %18 = phi i1 [ false, %7 ], [ %16, %13 ]
  %19 = zext i1 %18 to i64
  %20 = add i64 %9, %19
  %21 = add nuw i64 %8, 1
  %22 = icmp eq i64 %21, %2
  br i1 %22, label %5, label %7, !llvm.loop !48
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k36_count_xor_true(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %18, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %19, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %18, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp ne i8 %11, 0
  %13 = getelementptr inbounds i8, ptr %1, i64 %8
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = icmp ne i8 %14, 0
  %16 = xor i1 %12, %15
  %17 = zext i1 %16 to i64
  %18 = add i64 %9, %17
  %19 = add nuw i64 %8, 1
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %5, label %7, !llvm.loop !49
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k37_strlen(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %12, label %4

4:                                                ; preds = %2, %9
  %5 = phi i64 [ %10, %9 ], [ 0, %2 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, 0
  br i1 %8, label %12, label %9

9:                                                ; preds = %4
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %12, label %4, !llvm.loop !50

12:                                               ; preds = %4, %9, %2
  %13 = phi i64 [ 0, %2 ], [ %5, %4 ], [ %1, %9 ]
  ret i64 %13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k38_count_char(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !51
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @k39_contains_char(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = icmp eq i8 %11, %2
  %13 = zext i1 %12 to i64
  %14 = or i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !52
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k40_count_whitespace(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %18, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %18, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %19, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %10, 32
  %12 = add i8 %10, -9
  %13 = icmp ult i8 %12, 2
  %14 = or i1 %11, %13
  %15 = icmp eq i8 %10, 13
  %16 = or i1 %15, %14
  %17 = zext i1 %16 to i64
  %18 = add i64 %7, %17
  %19 = add nuw i64 %8, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %4, label %6, !llvm.loop !53
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k41_count_upper(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %14, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %15, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = add i8 %10, -65
  %12 = icmp ult i8 %11, 26
  %13 = zext i1 %12 to i64
  %14 = add i64 %7, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %6, !llvm.loop !54
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k42_count_digits(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %14, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %15, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = add i8 %10, -48
  %12 = icmp ult i8 %11, 10
  %13 = zext i1 %12 to i64
  %14 = add i64 %7, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %6, !llvm.loop !55
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
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

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k44_dot_product(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %17, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %18, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %17, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = zext i8 %11 to i64
  %13 = getelementptr inbounds i8, ptr %1, i64 %8
  %14 = load i8, ptr %13, align 1, !tbaa !5
  %15 = zext i8 %14 to i64
  %16 = mul nuw nsw i64 %15, %12
  %17 = add i64 %16, %9
  %18 = add nuw i64 %8, 1
  %19 = icmp eq i64 %18, %2
  br i1 %19, label %5, label %7, !llvm.loop !57
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k45_count_in_range(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %6, label %8

6:                                                ; preds = %8, %4
  %7 = phi i64 [ 0, %4 ], [ %17, %8 ]
  ret i64 %7

8:                                                ; preds = %4, %8
  %9 = phi i64 [ %18, %8 ], [ 0, %4 ]
  %10 = phi i64 [ %17, %8 ], [ 0, %4 ]
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = icmp uge i8 %12, %2
  %14 = icmp ule i8 %12, %3
  %15 = and i1 %13, %14
  %16 = zext i1 %15 to i64
  %17 = add i64 %10, %16
  %18 = add nuw i64 %9, 1
  %19 = icmp eq i64 %18, %1
  br i1 %19, label %6, label %8, !llvm.loop !58
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k46_sum_mod(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = urem i8 %11, %2
  %13 = zext i8 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !59
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @k47_count_divisible(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %15, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %16, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !5
  %12 = urem i8 %11, %2
  %13 = icmp eq i8 %12, 0
  %14 = zext i1 %13 to i64
  %15 = add i64 %9, %14
  %16 = add nuw i64 %8, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %5, label %7, !llvm.loop !60
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @k48_and_all(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ -1, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ -1, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = and i8 %10, %8
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !61
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i8 @k49_or_all(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i8 [ 0, %2 ], [ %11, %6 ]
  ret i8 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i8 [ %11, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = or i8 %10, %8
  %12 = add nuw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !62
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
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

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.ctpop.i8(i8) #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.umin.i8(i8, i8) #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.umax.i8(i8, i8) #2

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

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
!32 = distinct !{!32, !9, !10}
!33 = distinct !{!33, !9, !10}
!34 = distinct !{!34, !9, !10}
!35 = distinct !{!35, !9, !10}
!36 = distinct !{!36, !9, !10}
!37 = distinct !{!37, !9, !10}
!38 = distinct !{!38, !9, !10}
!39 = distinct !{!39, !9, !10}
!40 = distinct !{!40, !9, !10}
!41 = distinct !{!41, !9, !10}
!42 = distinct !{!42, !9, !10}
!43 = distinct !{!43, !9, !10}
!44 = distinct !{!44, !9, !10}
!45 = distinct !{!45, !9, !10}
!46 = distinct !{!46, !9, !10}
!47 = distinct !{!47, !9, !10}
!48 = distinct !{!48, !9, !10}
!49 = distinct !{!49, !9, !10}
!50 = distinct !{!50, !9, !10}
!51 = distinct !{!51, !9, !10}
!52 = distinct !{!52, !9, !10}
!53 = distinct !{!53, !9, !10}
!54 = distinct !{!54, !9, !10}
!55 = distinct !{!55, !9, !10}
!56 = distinct !{!56, !9, !10}
!57 = distinct !{!57, !9, !10}
!58 = distinct !{!58, !9, !10}
!59 = distinct !{!59, !9, !10}
!60 = distinct !{!60, !9, !10}
!61 = distinct !{!61, !9, !10}
!62 = distinct !{!62, !9, !10}
!63 = distinct !{!63, !9, !10}
