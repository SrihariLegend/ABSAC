; ModuleID = 'v11_corpus.c'
source_filename = "v11_corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@g_sink = internal unnamed_addr global i64 0, align 8

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p01_runtime_search_zero_elem(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
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
  br i1 %11, label %12, label %4, !llvm.loop !8

12:                                               ; preds = %9, %4, %2
  %13 = phi i64 [ 0, %2 ], [ %1, %9 ], [ %5, %4 ]
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %1)
  ret i64 %14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p02_runtime_search_eq_key_neg1(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3, %10
  %6 = phi i64 [ %11, %10 ], [ 0, %3 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %6
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp eq i8 %8, %2
  br i1 %9, label %13, label %10

10:                                               ; preds = %5
  %11 = add nuw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5, !llvm.loop !11

13:                                               ; preds = %10, %5, %3
  %14 = phi i64 [ -1, %3 ], [ -1, %10 ], [ %6, %5 ]
  ret i64 %14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p03_runtime_search_ne_key_n(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3, %10
  %6 = phi i64 [ %11, %10 ], [ 0, %3 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %6
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp eq i8 %8, %2
  br i1 %9, label %10, label %13

10:                                               ; preds = %5
  %11 = add nuw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5, !llvm.loop !12

13:                                               ; preds = %10, %5, %3
  %14 = phi i64 [ 0, %3 ], [ %1, %10 ], [ %6, %5 ]
  %15 = tail call i64 @llvm.umin.i64(i64 %14, i64 %1)
  ret i64 %15
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p04_runtime_search_u16(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %12, label %4

4:                                                ; preds = %2, %9
  %5 = phi i64 [ %10, %9 ], [ 0, %2 ]
  %6 = getelementptr inbounds i16, ptr %0, i64 %5
  %7 = load i16, ptr %6, align 2, !tbaa !13
  %8 = icmp eq i16 %7, 0
  br i1 %8, label %9, label %12

9:                                                ; preds = %4
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %12, label %4, !llvm.loop !15

12:                                               ; preds = %9, %4, %2
  %13 = phi i64 [ 0, %2 ], [ %1, %9 ], [ %5, %4 ]
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %1)
  ret i64 %14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p05_runtime_search_sentinel_nm1(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp ne i64 %1, 0
  br i1 %3, label %4, label %18

4:                                                ; preds = %2
  %5 = load i8, ptr %0, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %11, label %18

7:                                                ; preds = %11
  %8 = getelementptr inbounds i8, ptr %0, i64 %13
  %9 = load i8, ptr %8, align 1, !tbaa !5
  %10 = icmp eq i8 %9, 0
  br i1 %10, label %11, label %15, !llvm.loop !16

11:                                               ; preds = %4, %7
  %12 = phi i64 [ %13, %7 ], [ 0, %4 ]
  %13 = add nuw i64 %12, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %15, label %7, !llvm.loop !16

15:                                               ; preds = %11, %7
  %16 = phi i64 [ %1, %11 ], [ %13, %7 ]
  %17 = icmp ult i64 %13, %1
  br label %18

18:                                               ; preds = %15, %4, %2
  %19 = phi i64 [ 0, %2 ], [ 0, %4 ], [ %16, %15 ]
  %20 = phi i1 [ %3, %2 ], [ %3, %4 ], [ %17, %15 ]
  %21 = add i64 %1, -1
  %22 = select i1 %20, i64 %19, i64 %21
  ret i64 %22
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p06_runtime_search_do_while(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call i64 @llvm.umax.i64(i64 %1, i64 1)
  br label %4

4:                                                ; preds = %9, %2
  %5 = phi i64 [ 0, %2 ], [ %10, %9 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, 0
  br i1 %8, label %9, label %12

9:                                                ; preds = %4
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %3
  br i1 %11, label %12, label %4, !llvm.loop !17

12:                                               ; preds = %9, %4
  %13 = phi i64 [ %5, %4 ], [ %1, %9 ]
  ret i64 %13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p07_runtime_descending_search(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  br label %3

3:                                                ; preds = %6, %2
  %4 = phi i64 [ %1, %2 ], [ %7, %6 ]
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %11, label %6

6:                                                ; preds = %3
  %7 = add i64 %4, -1
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8, align 1, !tbaa !5
  %10 = icmp eq i8 %9, 0
  br i1 %10, label %3, label %11, !llvm.loop !18

11:                                               ; preds = %6, %3
  %12 = phi i64 [ %1, %3 ], [ %7, %6 ]
  ret i64 %12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p08_const_search_zero_elem(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %10, label %7

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 64
  br i1 %9, label %10, label %2, !llvm.loop !19

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 64, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p09_const_search_ge_key(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %3

3:                                                ; preds = %2, %8
  %4 = phi i64 [ 0, %2 ], [ %9, %8 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !5
  %7 = icmp ult i8 %6, %1
  br i1 %7, label %8, label %11

8:                                                ; preds = %3
  %9 = add nuw nsw i64 %4, 1
  %10 = icmp eq i64 %9, 64
  br i1 %10, label %11, label %3, !llvm.loop !20

11:                                               ; preds = %3, %8
  %12 = phi i64 [ 64, %8 ], [ %4, %3 ]
  ret i64 %12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p10_count_ne_const_48(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %11

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp ne i8 %8, %1
  %10 = zext i1 %9 to i64
  %11 = add i64 %6, %10
  %12 = add nuw nsw i64 %5, 1
  %13 = icmp eq i64 %12, 48
  br i1 %13, label %3, label %4, !llvm.loop !21
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p11_two_loops_const_56(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %3

3:                                                ; preds = %2, %3
  %4 = phi i64 [ 0, %2 ], [ %11, %3 ]
  %5 = phi i64 [ 0, %2 ], [ %10, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, %1
  %9 = zext i1 %8 to i64
  %10 = add i64 %5, %9
  %11 = add nuw nsw i64 %4, 1
  %12 = icmp eq i64 %11, 56
  br i1 %12, label %15, label %3, !llvm.loop !22

13:                                               ; preds = %15
  %14 = xor i64 %21, %10
  ret i64 %14

15:                                               ; preds = %3, %15
  %16 = phi i64 [ %22, %15 ], [ 0, %3 ]
  %17 = phi i64 [ %21, %15 ], [ 0, %3 ]
  %18 = getelementptr inbounds i8, ptr %0, i64 %16
  %19 = load i8, ptr %18, align 1, !tbaa !5
  %20 = zext i8 %19 to i64
  %21 = add i64 %17, %20
  %22 = add nuw nsw i64 %16, 1
  %23 = icmp eq i64 %22, 56
  br i1 %23, label %13, label %15, !llvm.loop !23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p12_sum_u16_const_64(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %6 = getelementptr inbounds i16, ptr %0, i64 %4
  %7 = load i16, ptr %6, align 2, !tbaa !13
  %8 = zext i16 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 1
  %11 = icmp eq i64 %10, 64
  br i1 %11, label %2, label %3, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local zeroext i16 @n01_running_max_const(ptr nocapture noundef readonly %0, i16 noundef zeroext %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i16 %9

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %10, %4 ]
  %6 = phi i16 [ %1, %2 ], [ %9, %4 ]
  %7 = getelementptr inbounds i16, ptr %0, i64 %5
  %8 = load i16, ptr %7, align 2, !tbaa !13
  %9 = tail call i16 @llvm.umax.i16(i16 %8, i16 %6)
  %10 = add nuw nsw i64 %5, 1
  %11 = icmp eq i64 %10, 64
  br i1 %11, label %3, label %4, !llvm.loop !25
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n02_masked_sum_ternary_const(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %11

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %12, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %11, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp ugt i8 %7, 5
  %9 = select i1 %8, i8 %7, i8 0
  %10 = zext i8 %9 to i64
  %11 = add i64 %5, %10
  %12 = add nuw nsw i64 %4, 1
  %13 = icmp eq i64 %12, 64
  br i1 %13, label %2, label %3, !llvm.loop !26
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n03_masked_sum_hi_const(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %10

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %11, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = and i8 %7, -16
  %9 = zext i8 %8 to i64
  %10 = add i64 %5, %9
  %11 = add nuw nsw i64 %4, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %2, label %3, !llvm.loop !27
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define dso_local i64 @n04_volatile_load_const(ptr noundef %0) local_unnamed_addr #1 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load volatile i8, ptr %6, align 1, !tbaa !5
  %8 = zext i8 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 1
  %11 = icmp eq i64 %10, 32
  br i1 %11, label %2, label %3, !llvm.loop !28
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite) uwtable
define dso_local i64 @n05_atomic_load_const(ptr nocapture noundef readonly %0) local_unnamed_addr #2 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %6 = getelementptr inbounds i32, ptr %0, i64 %5
  %7 = load atomic i32, ptr %6 monotonic, align 4
  %8 = zext i32 %7 to i64
  %9 = add i64 %4, %8
  %10 = add nuw nsw i64 %5, 1
  %11 = icmp eq i64 %10, 32
  br i1 %11, label %2, label %3, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local i64 @n06_may_alias_store_const(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1, i16 noundef zeroext %2) local_unnamed_addr #3 {
  br label %5

4:                                                ; preds = %5
  ret i64 %13

5:                                                ; preds = %3, %5
  %6 = phi i64 [ 0, %3 ], [ %14, %5 ]
  %7 = phi i64 [ 0, %3 ], [ %13, %5 ]
  %8 = getelementptr inbounds i16, ptr %1, i64 %6
  store i16 %2, ptr %8, align 2, !tbaa !13
  %9 = getelementptr inbounds i16, ptr %0, i64 %6
  %10 = load i16, ptr %9, align 2, !tbaa !13
  %11 = icmp eq i16 %10, %2
  %12 = zext i1 %11 to i64
  %13 = add i64 %7, %12
  %14 = add nuw nsw i64 %6, 1
  %15 = icmp eq i64 %14, 48
  br i1 %15, label %4, label %5, !llvm.loop !30
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n07_two_arrays_const(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %13

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %14, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %13, %4 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = getelementptr inbounds i8, ptr %1, i64 %5
  %10 = load i8, ptr %9, align 1, !tbaa !5
  %11 = icmp eq i8 %8, %10
  %12 = zext i1 %11 to i64
  %13 = add i64 %6, %12
  %14 = add nuw nsw i64 %5, 1
  %15 = icmp eq i64 %14, 36
  br i1 %15, label %3, label %4, !llvm.loop !31
}

; Function Attrs: mustprogress nofree norecurse noreturn nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @n08_nonterminating_reverse(ptr nocapture noundef readonly %0) local_unnamed_addr #4 {
  unreachable
}

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, argmem: read, inaccessiblemem: none) uwtable
define dso_local i64 @n09_early_exit_global_const(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #5 {
  br label %3

3:                                                ; preds = %2, %11
  %4 = phi i64 [ 0, %2 ], [ %12, %11 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !5
  %7 = icmp eq i8 %6, %1
  br i1 %7, label %8, label %11

8:                                                ; preds = %3
  %9 = load i64, ptr @g_sink, align 8, !tbaa !32
  %10 = add i64 %9, %4
  store i64 %10, ptr @g_sink, align 8, !tbaa !32
  br label %14

11:                                               ; preds = %3
  %12 = add nuw nsw i64 %4, 1
  %13 = icmp eq i64 %12, 48
  br i1 %13, label %14, label %3, !llvm.loop !34

14:                                               ; preds = %11, %8
  %15 = phi i64 [ %4, %8 ], [ 48, %11 ]
  ret i64 %15
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n10_runtime_sum_u16(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i16, ptr %0, i64 %7
  %10 = load i16, ptr %9, align 2, !tbaa !13
  %11 = zext i16 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !35
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n11_runtime_count_ge_u32(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i32, ptr %0, i64 %8
  %11 = load i32, ptr %10, align 4, !tbaa !36
  %12 = icmp uge i32 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !38
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n12_signed_sum_const(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %8

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %8, %3 ]
  %6 = getelementptr inbounds i64, ptr %0, i64 %4
  %7 = load i64, ptr %6, align 8, !tbaa !32
  %8 = add nsw i64 %7, %5
  %9 = add nuw nsw i64 %4, 1
  %10 = icmp eq i64 %9, 32
  br i1 %10, label %2, label %3, !llvm.loop !39
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #6

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #6

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i16 @llvm.umax.i16(i16, i16) #6

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { mustprogress nofree norecurse noreturn nosync nounwind willreturn memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nofree norecurse nosync nounwind memory(readwrite, argmem: read, inaccessiblemem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #6 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

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
!13 = !{!14, !14, i64 0}
!14 = !{!"short", !6, i64 0}
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
!32 = !{!33, !33, i64 0}
!33 = !{!"long", !6, i64 0}
!34 = distinct !{!34, !9, !10}
!35 = distinct !{!35, !9, !10}
!36 = !{!37, !37, i64 0}
!37 = !{!"int", !6, i64 0}
!38 = distinct !{!38, !9, !10}
!39 = distinct !{!39, !9, !10}
