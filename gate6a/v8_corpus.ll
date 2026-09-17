; ModuleID = 'v8_corpus.c'
source_filename = "v8_corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@g_sink = internal unnamed_addr global i64 0, align 8

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p01_first_set_48(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 48
  br i1 %9, label %10, label %2, !llvm.loop !8

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 48, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p02_first_set_96(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 96
  br i1 %9, label %10, label %2, !llvm.loop !11

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 96, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p03_first_nonzero_u16_80(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i16, ptr %0, i64 %3
  %5 = load i16, ptr %4, align 2, !tbaa !12
  %6 = icmp eq i16 %5, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 80
  br i1 %9, label %10, label %2, !llvm.loop !14

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 80, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p04_first_eq_key_u8_64(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %3

3:                                                ; preds = %2, %8
  %4 = phi i64 [ 0, %2 ], [ %9, %8 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !5
  %7 = icmp eq i8 %6, %1
  br i1 %7, label %11, label %8

8:                                                ; preds = %3
  %9 = add nuw nsw i64 %4, 1
  %10 = icmp eq i64 %9, 64
  br i1 %10, label %11, label %3, !llvm.loop !15

11:                                               ; preds = %3, %8
  %12 = phi i64 [ 64, %8 ], [ %4, %3 ]
  ret i64 %12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p05_first_set_128(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 128
  br i1 %9, label %10, label %2, !llvm.loop !16

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 128, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p06_two_loops_count_sum_const(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
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
  %12 = icmp eq i64 %11, 40
  br i1 %12, label %15, label %3, !llvm.loop !17

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
  %23 = icmp eq i64 %22, 40
  br i1 %23, label %13, label %15, !llvm.loop !18
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p07_sum_u8_const_112(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = zext i8 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 1
  %11 = icmp eq i64 %10, 112
  br i1 %11, label %2, label %3, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p08_count_ge_const_u8_56(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %11

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %5
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp uge i8 %8, %1
  %10 = zext i1 %9 to i64
  %11 = add i64 %6, %10
  %12 = add nuw nsw i64 %5, 1
  %13 = icmp eq i64 %12, 56
  br i1 %13, label %3, label %4, !llvm.loop !20
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p09_count_le_const_u32_72(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %11

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %7 = getelementptr inbounds i32, ptr %0, i64 %5
  %8 = load i32, ptr %7, align 4, !tbaa !21
  %9 = icmp ule i32 %8, %1
  %10 = zext i1 %9 to i64
  %11 = add i64 %6, %10
  %12 = add nuw nsw i64 %5, 1
  %13 = icmp eq i64 %12, 72
  br i1 %13, label %3, label %4, !llvm.loop !23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @p10_all_ge_const_u16_40(ptr nocapture noundef readonly %0, i16 noundef zeroext %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %10

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %6 = phi i64 [ 1, %2 ], [ %10, %4 ]
  %7 = getelementptr inbounds i16, ptr %0, i64 %5
  %8 = load i16, ptr %7, align 2, !tbaa !12
  %9 = icmp ult i16 %8, %1
  %10 = select i1 %9, i64 0, i64 %6
  %11 = add nuw nsw i64 %5, 1
  %12 = icmp eq i64 %11, 40
  br i1 %12, label %3, label %4, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p11_map_then_sum_const(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %10

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %11, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %6 = getelementptr inbounds i16, ptr %0, i64 %4
  %7 = load i16, ptr %6, align 2, !tbaa !12
  %8 = xor i16 %7, 33
  %9 = zext i16 %8 to i64
  %10 = add i64 %5, %9
  %11 = add nuw nsw i64 %4, 1
  %12 = icmp eq i64 %11, 64
  br i1 %12, label %2, label %3, !llvm.loop !25
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p12_search_subrange_short(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 48
  br i1 %9, label %10, label %2, !llvm.loop !26

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 128, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p13_search_sentinel_zero(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %2

2:                                                ; preds = %1, %7
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %7, label %10

7:                                                ; preds = %2
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 64
  br i1 %9, label %10, label %2, !llvm.loop !27

10:                                               ; preds = %2, %7
  %11 = phi i64 [ 0, %7 ], [ %3, %2 ]
  ret i64 %11
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p14_runtime_extent_search(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %12, label %4

4:                                                ; preds = %2, %9
  %5 = phi i64 [ %10, %9 ], [ 0, %2 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, 0
  br i1 %8, label %9, label %12

9:                                                ; preds = %4
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %12, label %4, !llvm.loop !28

12:                                               ; preds = %9, %4, %2
  %13 = phi i64 [ 0, %2 ], [ %1, %9 ], [ %5, %4 ]
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %1)
  ret i64 %14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p15_do_while_stride3_sum(ptr nocapture noundef readonly %0) local_unnamed_addr #0 {
  br label %3

2:                                                ; preds = %3
  ret i64 %9

3:                                                ; preds = %1, %3
  %4 = phi i64 [ 0, %1 ], [ %10, %3 ]
  %5 = phi i64 [ 0, %1 ], [ %9, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = zext i8 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 3
  %11 = icmp ult i64 %4, 60
  br i1 %11, label %3, label %2, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p16_search_then_sum(ptr nocapture noundef readonly %0, i8 noundef zeroext %1) local_unnamed_addr #0 {
  %3 = load i8, ptr %0, align 1, !tbaa !5
  %4 = icmp eq i8 %3, %1
  br i1 %4, label %15, label %9

5:                                                ; preds = %9
  %6 = getelementptr inbounds i8, ptr %0, i64 %11
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, %1
  br i1 %8, label %13, label %9, !llvm.loop !30

9:                                                ; preds = %2, %5
  %10 = phi i64 [ %11, %5 ], [ 0, %2 ]
  %11 = add nuw nsw i64 %10, 1
  %12 = icmp eq i64 %11, 40
  br i1 %12, label %18, label %5, !llvm.loop !30

13:                                               ; preds = %5
  %14 = icmp ugt i64 %10, 38
  br label %15

15:                                               ; preds = %13, %2
  %16 = phi i1 [ %14, %13 ], [ false, %2 ]
  %17 = phi i64 [ %11, %13 ], [ 0, %2 ]
  br i1 %16, label %18, label %30

18:                                               ; preds = %9, %15
  br label %21

19:                                               ; preds = %21
  %20 = add i64 %27, 40
  br label %30

21:                                               ; preds = %18, %21
  %22 = phi i64 [ %28, %21 ], [ 0, %18 ]
  %23 = phi i64 [ %27, %21 ], [ 0, %18 ]
  %24 = getelementptr inbounds i8, ptr %0, i64 %22
  %25 = load i8, ptr %24, align 1, !tbaa !5
  %26 = zext i8 %25 to i64
  %27 = add i64 %23, %26
  %28 = add nuw nsw i64 %22, 1
  %29 = icmp eq i64 %28, 40
  br i1 %29, label %19, label %21, !llvm.loop !31

30:                                               ; preds = %15, %19
  %31 = phi i64 [ %17, %15 ], [ %20, %19 ]
  ret i64 %31
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
  %8 = load i16, ptr %7, align 2, !tbaa !12
  %9 = tail call i16 @llvm.umax.i16(i16 %8, i16 %6)
  %10 = add nuw nsw i64 %5, 1
  %11 = icmp eq i64 %10, 64
  br i1 %11, label %3, label %4, !llvm.loop !32
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
  br i1 %13, label %2, label %3, !llvm.loop !33
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
  br i1 %12, label %2, label %3, !llvm.loop !34
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
  br i1 %11, label %2, label %3, !llvm.loop !35
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite) uwtable
define dso_local i64 @n05_atomic_fetch_add(ptr nocapture noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %11, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %11, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = atomicrmw add ptr %0, i32 1 monotonic, align 4
  %10 = zext i32 %9 to i64
  %11 = add i64 %7, %10
  %12 = add nuw i64 %8, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !36
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
  store i16 %2, ptr %8, align 2, !tbaa !12
  %9 = getelementptr inbounds i16, ptr %0, i64 %6
  %10 = load i16, ptr %9, align 2, !tbaa !12
  %11 = icmp eq i16 %10, %2
  %12 = zext i1 %11 to i64
  %13 = add i64 %7, %12
  %14 = add nuw nsw i64 %6, 1
  %15 = icmp eq i64 %14, 48
  br i1 %15, label %4, label %5, !llvm.loop !37
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
  br i1 %15, label %3, label %4, !llvm.loop !38
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
  %9 = load i64, ptr @g_sink, align 8, !tbaa !39
  %10 = add i64 %9, %4
  store i64 %10, ptr @g_sink, align 8, !tbaa !39
  br label %14

11:                                               ; preds = %3
  %12 = add nuw nsw i64 %4, 1
  %13 = icmp eq i64 %12, 48
  br i1 %13, label %14, label %3, !llvm.loop !41

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
  %10 = load i16, ptr %9, align 2, !tbaa !12
  %11 = zext i16 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !42
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
  %11 = load i32, ptr %10, align 4, !tbaa !21
  %12 = icmp uge i32 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !43
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
  %7 = load i64, ptr %6, align 8, !tbaa !39
  %8 = add nsw i64 %7, %5
  %9 = add nuw nsw i64 %4, 1
  %10 = icmp eq i64 %9, 32
  br i1 %10, label %2, label %3, !llvm.loop !44
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #6

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
!12 = !{!13, !13, i64 0}
!13 = !{!"short", !6, i64 0}
!14 = distinct !{!14, !9, !10}
!15 = distinct !{!15, !9, !10}
!16 = distinct !{!16, !9, !10}
!17 = distinct !{!17, !9, !10}
!18 = distinct !{!18, !9, !10}
!19 = distinct !{!19, !9, !10}
!20 = distinct !{!20, !9, !10}
!21 = !{!22, !22, i64 0}
!22 = !{!"int", !6, i64 0}
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
!39 = !{!40, !40, i64 0}
!40 = !{!"long", !6, i64 0}
!41 = distinct !{!41, !9, !10}
!42 = distinct !{!42, !9, !10}
!43 = distinct !{!43, !9, !10}
!44 = distinct !{!44, !9, !10}
