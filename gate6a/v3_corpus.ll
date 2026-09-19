; ModuleID = 'gate6a/v3_corpus.c'
source_filename = "gate6a/v3_corpus.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@g6a_v3_out = external local_unnamed_addr global i64, align 8

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p01_count_ge_u16(ptr nocapture noundef readonly %0, i64 noundef %1, i16 noundef zeroext %2) local_unnamed_addr #0 {
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
  %12 = icmp uge i16 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !9
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @p02_sum_u8_u32acc(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #0 {
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
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !12
  %13 = zext i8 %12 to i32
  %14 = add i32 %10, %13
  %15 = add nuw nsw i64 %9, 1
  %16 = icmp eq i64 %15, %5
  br i1 %16, label %6, label %8, !llvm.loop !13
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @p03_all_ne_const(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef signext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 1, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 1, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = icmp eq i8 %11, %2
  %13 = select i1 %12, i64 0, i64 %9
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !14
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p04_count_ge_const_u32(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #0 {
  br label %4

3:                                                ; preds = %4
  ret i64 %11

4:                                                ; preds = %2, %4
  %5 = phi i64 [ 0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %7 = getelementptr inbounds i32, ptr %0, i64 %5
  %8 = load i32, ptr %7, align 4, !tbaa !15
  %9 = icmp uge i32 %8, %1
  %10 = zext i1 %9 to i64
  %11 = add i64 %6, %10
  %12 = add nuw nsw i64 %5, 1
  %13 = icmp eq i64 %12, 128
  br i1 %13, label %3, label %4, !llvm.loop !17
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p05_sum_u16_u64acc(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i16, ptr %0, i64 %7
  %10 = load i16, ptr %9, align 2, !tbaa !5
  %11 = zext i16 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !18
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p06_count_ne_u8(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %14, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = icmp ne i8 %11, %2
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %5, label %7, !llvm.loop !19
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @p07_all_eq_u64(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 1, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 1, %3 ]
  %10 = getelementptr inbounds i64, ptr %0, i64 %8
  %11 = load i64, ptr %10, align 8, !tbaa !20
  %12 = icmp eq i64 %11, %2
  %13 = select i1 %12, i64 %9, i64 0
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !22
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @p08_all_ge_u8(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 1, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 1, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = icmp ult i8 %11, %2
  %13 = select i1 %12, i64 0, i64 %9
  %14 = add nuw i64 %8, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %5, label %7, !llvm.loop !23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local range(i64 0, 2) i64 @p09_any_or_u8(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
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
  %13 = load i8, ptr %12, align 1, !tbaa !12
  %14 = or i8 %13, %11
  %15 = add nuw i64 %10, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %9, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p10_count_eq_do_while(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #0 {
  %4 = tail call i64 @llvm.umax.i64(i64 %1, i64 1)
  br label %5

5:                                                ; preds = %5, %3
  %6 = phi i64 [ 0, %3 ], [ %12, %5 ]
  %7 = phi i64 [ 0, %3 ], [ %13, %5 ]
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8, align 1, !tbaa !12
  %10 = icmp eq i8 %9, %2
  %11 = zext i1 %10 to i64
  %12 = add i64 %6, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %4
  br i1 %14, label %15, label %5, !llvm.loop !25

15:                                               ; preds = %5
  ret i64 %12
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p11_sum_plus_one_u16(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i16, ptr %0, i64 %7
  %10 = load i16, ptr %9, align 2, !tbaa !5
  %11 = zext i16 %10 to i64
  %12 = add i64 %8, 1
  %13 = add i64 %12, %11
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6, !llvm.loop !26
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @p12_two_reductions(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %7

4:                                                ; preds = %7, %2
  %5 = phi i64 [ 0, %2 ], [ %14, %7 ]
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %17, label %20

7:                                                ; preds = %2, %7
  %8 = phi i64 [ %15, %7 ], [ 0, %2 ]
  %9 = phi i64 [ %14, %7 ], [ 0, %2 ]
  %10 = getelementptr inbounds i32, ptr %0, i64 %8
  %11 = load i32, ptr %10, align 4, !tbaa !15
  %12 = icmp eq i32 %11, 0
  %13 = zext i1 %12 to i64
  %14 = add i64 %9, %13
  %15 = add nuw i64 %8, 1
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %4, label %7, !llvm.loop !27

17:                                               ; preds = %20, %4
  %18 = phi i64 [ 0, %4 ], [ %26, %20 ]
  %19 = xor i64 %18, %5
  ret i64 %19

20:                                               ; preds = %4, %20
  %21 = phi i64 [ %27, %20 ], [ 0, %4 ]
  %22 = phi i64 [ %26, %20 ], [ 0, %4 ]
  %23 = getelementptr inbounds i32, ptr %0, i64 %21
  %24 = load i32, ptr %23, align 4, !tbaa !15
  %25 = zext i32 %24 to i64
  %26 = add i64 %22, %25
  %27 = add nuw i64 %21, 1
  %28 = icmp eq i64 %27, %1
  br i1 %28, label %17, label %20, !llvm.loop !28
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n01_sum_i64_nsw(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp sgt i64 %1, 0
  br i1 %3, label %6, label %4

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %11, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %11, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i64, ptr %0, i64 %7
  %10 = load i64, ptr %9, align 8, !tbaa !20
  %11 = add nsw i64 %10, %8
  %12 = add nuw nsw i64 %7, 1
  %13 = icmp eq i64 %12, %1
  br i1 %13, label %4, label %6, !llvm.loop !29
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i32 @n02_running_max(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = load i32, ptr %0, align 4, !tbaa !15
  %4 = icmp sgt i64 %1, 1
  br i1 %4, label %7, label %5

5:                                                ; preds = %7, %2
  %6 = phi i32 [ %3, %2 ], [ %12, %7 ]
  ret i32 %6

7:                                                ; preds = %2, %7
  %8 = phi i64 [ %13, %7 ], [ 1, %2 ]
  %9 = phi i32 [ %12, %7 ], [ %3, %2 ]
  %10 = getelementptr inbounds i32, ptr %0, i64 %8
  %11 = load i32, ptr %10, align 4, !tbaa !15
  %12 = tail call i32 @llvm.smax.i32(i32 %11, i32 %9)
  %13 = add nuw nsw i64 %8, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %5, label %7, !llvm.loop !30
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n03_count_equal_pairs(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %16, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %17, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %16, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = getelementptr inbounds i8, ptr %1, i64 %8
  %13 = load i8, ptr %12, align 1, !tbaa !12
  %14 = icmp eq i8 %11, %13
  %15 = zext i1 %14 to i64
  %16 = add i64 %9, %15
  %17 = add nuw i64 %8, 1
  %18 = icmp eq i64 %17, %2
  br i1 %18, label %5, label %7, !llvm.loop !31
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local i64 @n04_store_maybe_alias(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1, i64 noundef %2, i8 noundef zeroext %3) local_unnamed_addr #1 {
  %5 = icmp eq i64 %2, 0
  br i1 %5, label %6, label %8

6:                                                ; preds = %8, %4
  %7 = phi i64 [ 0, %4 ], [ %16, %8 ]
  ret i64 %7

8:                                                ; preds = %4, %8
  %9 = phi i64 [ %17, %8 ], [ 0, %4 ]
  %10 = phi i64 [ %16, %8 ], [ 0, %4 ]
  %11 = getelementptr inbounds i8, ptr %1, i64 %9
  store i8 %3, ptr %11, align 1, !tbaa !12
  %12 = getelementptr inbounds i8, ptr %0, i64 %9
  %13 = load i8, ptr %12, align 1, !tbaa !12
  %14 = icmp eq i8 %13, %3
  %15 = zext i1 %14 to i64
  %16 = add i64 %10, %15
  %17 = add nuw i64 %9, 1
  %18 = icmp eq i64 %17, %2
  br i1 %18, label %6, label %8, !llvm.loop !32
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define dso_local i64 @n05_volatile_load(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load volatile i8, ptr %9, align 1, !tbaa !12
  %11 = zext i8 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !33
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n06_stride2(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !12
  %11 = zext i8 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 2
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %6, label %4, !llvm.loop !34
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n07_dynamic_stride(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %7, %3
  %6 = phi i64 [ 0, %3 ], [ %13, %7 ]
  ret i64 %6

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %14, %7 ], [ 0, %3 ]
  %9 = phi i64 [ %13, %7 ], [ 0, %3 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = zext i8 %11 to i64
  %13 = add i64 %9, %12
  %14 = add i64 %8, %2
  %15 = icmp ult i64 %14, %1
  br i1 %15, label %7, label %5, !llvm.loop !35
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n08_conditional_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %15, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %16, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %15, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1, !tbaa !12
  %11 = and i8 %10, 1
  %12 = icmp eq i8 %11, 0
  %13 = select i1 %12, i8 0, i8 %10
  %14 = zext i8 %13 to i64
  %15 = add i64 %8, %14
  %16 = add nuw i64 %7, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %4, label %6, !llvm.loop !36
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @n09_self_recurrence(i64 noundef %0) local_unnamed_addr #3 {
  ret i64 1
}

; Function Attrs: nofree norecurse nosync nounwind memory(write, argmem: read, inaccessiblemem: none) uwtable
define dso_local range(i64 0, 2) i64 @n10_early_exit_write(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr #4 {
  %4 = icmp ne i64 %1, 0
  br i1 %4, label %5, label %21

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !12
  %7 = icmp eq i8 %6, %2
  br i1 %7, label %14, label %17

8:                                                ; preds = %17
  %9 = getelementptr inbounds i8, ptr %0, i64 %19
  %10 = load i8, ptr %9, align 1, !tbaa !12
  %11 = icmp eq i8 %10, %2
  br i1 %11, label %12, label %17, !llvm.loop !37

12:                                               ; preds = %8
  %13 = icmp ult i64 %19, %1
  br label %14

14:                                               ; preds = %12, %5
  %15 = phi i1 [ %4, %5 ], [ %13, %12 ]
  %16 = phi i64 [ 0, %5 ], [ %19, %12 ]
  store i64 %16, ptr @g6a_v3_out, align 8, !tbaa !20
  br label %21

17:                                               ; preds = %5, %8
  %18 = phi i64 [ %19, %8 ], [ 0, %5 ]
  %19 = add nuw i64 %18, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %21, label %8, !llvm.loop !37

21:                                               ; preds = %17, %3, %14
  %22 = phi i1 [ %15, %14 ], [ %4, %3 ], [ false, %17 ]
  %23 = zext i1 %22 to i64
  ret i64 %23
}

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: read) uwtable
define dso_local i64 @n11_reverse_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = icmp sgt i64 %1, 0
  br i1 %3, label %6, label %4

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %9, %6 ], [ %1, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = add nsw i64 %7, -1
  %10 = getelementptr inbounds i8, ptr %0, i64 %9
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = zext i8 %11 to i64
  %13 = add i64 %8, %12
  %14 = icmp sgt i64 %7, 1
  br i1 %14, label %6, label %4, !llvm.loop !38
}

; Function Attrs: nofree norecurse nounwind memory(argmem: readwrite) uwtable
define dso_local i64 @n12_atomic_sum(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #5 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load atomic i8, ptr %9 monotonic, align 1
  %11 = zext i8 %10 to i64
  %12 = add i64 %7, %11
  %13 = add nuw i64 %8, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %4, label %6, !llvm.loop !39
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #6

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smax.i32(i32, i32) #6

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: read) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nounwind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nofree norecurse nosync nounwind memory(write, argmem: read, inaccessiblemem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nofree norecurse nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #6 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

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
!12 = !{!7, !7, i64 0}
!13 = distinct !{!13, !10, !11}
!14 = distinct !{!14, !10, !11}
!15 = !{!16, !16, i64 0}
!16 = !{!"int", !7, i64 0}
!17 = distinct !{!17, !10, !11}
!18 = distinct !{!18, !10, !11}
!19 = distinct !{!19, !10, !11}
!20 = !{!21, !21, i64 0}
!21 = !{!"long", !7, i64 0}
!22 = distinct !{!22, !10, !11}
!23 = distinct !{!23, !10, !11}
!24 = distinct !{!24, !10, !11}
!25 = distinct !{!25, !10, !11}
!26 = distinct !{!26, !10, !11}
!27 = distinct !{!27, !10, !11}
!28 = distinct !{!28, !10, !11}
!29 = distinct !{!29, !10, !11}
!30 = distinct !{!30, !10, !11}
!31 = distinct !{!31, !10, !11}
!32 = distinct !{!32, !10, !11}
!33 = distinct !{!33, !10, !11}
!34 = distinct !{!34, !10, !11}
!35 = distinct !{!35, !10, !11}
!36 = distinct !{!36, !10, !11}
!37 = distinct !{!37, !10, !11}
!38 = distinct !{!38, !10, !11}
!39 = distinct !{!39, !10, !11}
