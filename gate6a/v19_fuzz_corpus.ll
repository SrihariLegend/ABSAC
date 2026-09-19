; Deterministic random differential corpus (gen_random_corpus).
; Shapes are restricted to templates already known to lower.

define dso_local i32 @f000(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = sub i32 %0, %0
  %v1 = lshr i32 %1, 18
  %v2 = shl i32 %v1, 15
  %v3 = and i32 %v2, %1
  ret i32 %v3
}

define dso_local i64 @f001(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ugt i8 %v, 1
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f002(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f003(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %1, 16
  %v1 = xor i64 %0, %1
  %v2 = xor i64 %0, %v0
  %v3 = and i64 %v2, %1
  ret i64 %v3
}

define dso_local i64 @f004(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ne i16 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f005(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f006(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = or i32 %1, %1
  %v1 = sub i32 %0, %1
  %v2 = lshr i32 %v1, 11
  %v3 = xor i32 %1, %v0
  ret i32 %v3
}

define dso_local i64 @f007(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f008(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f009(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = sub i32 %0, %0
  %v1 = mul i32 %v0, %v0
  %v2 = xor i32 %1, %1
  %v3 = add i32 %v0, %v2
  ret i32 %v3
}

define dso_local i64 @f010(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ne i32 %v, 7
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f011(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ult i16 %v, 3
  %cnt = zext i1 %cmp to i64
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f012(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = mul i64 %0, %0
  %v1 = add i64 %1, %0
  ret i64 %v1
}

define dso_local i64 @f013(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ugt i16 %v, 5
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f014(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %nz = icmp ne i8 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i8 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f015(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i64 %0, 4
  %v1 = sub i64 %0, %v0
  ret i64 %v1
}

define dso_local i64 @f016(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ne i8 %v, 5
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f017(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %one = and i16 %v, 1
  %cond = icmp ne i16 %one, 0
  %zero = add i16 0, 0
  %sel = select i1 %cond, i16 %v, i16 %zero
  %sel64 = zext i16 %sel to i64
  %pre = add i16 0, 0
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f018(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %1, 1
  %v1 = add i64 %0, %1
  %v2 = sub i64 %v0, %0
  %v3 = or i64 %0, %v2
  ret i64 %v3
}

define dso_local i64 @f019(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp uge i32 %v, 1
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f020(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %nz = icmp ne i32 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i32 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f021(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i64 %1, %0
  %v1 = sub i64 %1, %0
  ret i64 %v1
}

define dso_local i64 @f022(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ule i16 %v, 1
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f023(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f024(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = sub i64 %1, %1
  %v1 = or i64 %0, %0
  ret i64 %v1
}

define dso_local i64 @f025(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp uge i16 %v, 7
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f026(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ne i8 %v, 6
  %cnt = zext i1 %cmp to i64
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f027(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = or i64 %1, %0
  %v1 = add i64 %v0, %0
  %v2 = xor i64 %v0, %v1
  %v3 = xor i64 %v0, %0
  %v4 = add i64 %1, %1
  ret i64 %v4
}

define dso_local i64 @f028(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f029(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f030(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i64 %0, %1
  %v1 = mul i64 %v0, %1
  %v2 = add i64 %0, %v1
  ret i64 %v2
}

define dso_local i64 @f031(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f032(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ule i32 %v, 0
  %cnt = zext i1 %cmp to i64
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f033(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i64 %0, %0
  %v1 = xor i64 %1, %1
  %v2 = lshr i64 %v1, 10
  ret i64 %v2
}

define dso_local i64 @f034(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f035(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f036(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = mul i32 %1, %0
  %v1 = add i32 %0, %1
  %v2 = lshr i32 %0, 1
  ret i32 %v2
}

define dso_local i64 @f037(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f038(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f039(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = and i32 %1, %0
  %v1 = sub i32 %1, %1
  ret i32 %v1
}

define dso_local i64 @f040(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ne i32 %v, 7
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f041(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp uge i16 %v, 1
  %cnt = zext i1 %cmp to i64
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f042(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i64 %0, %1
  %v1 = mul i64 %v0, %0
  %v2 = mul i64 %v1, %1
  %v3 = or i64 %v0, %v2
  %v4 = mul i64 %1, %v3
  ret i64 %v4
}

define dso_local i64 @f043(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ne i16 %v, 5
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f044(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f045(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %0, 25
  %v1 = or i64 %0, %v0
  %v2 = xor i64 %1, %v0
  %v3 = lshr i64 %v2, 30
  %v4 = mul i64 %v2, %v0
  ret i64 %v4
}

define dso_local i64 @f046(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f047(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %nz = icmp ne i32 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i32 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f048(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i64 %1, 8
  %v1 = add i64 %0, %0
  %v2 = lshr i64 %1, 1
  %v3 = or i64 %v1, %0
  ret i64 %v3
}

define dso_local i64 @f049(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ule i32 %v, 6
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f050(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp uge i16 %v, 4
  %cnt = zext i1 %cmp to i64
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f051(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i64 %1, %0
  %v1 = xor i64 %v0, %v0
  ret i64 %v1
}

define dso_local i64 @f052(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp uge i8 %v, 4
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f053(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f054(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i64 %1, %0
  %v1 = shl i64 %v0, 5
  ret i64 %v1
}

define dso_local i64 @f055(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f056(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f057(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i64 %1, 9
  %v1 = shl i64 %0, 21
  %v2 = shl i64 %v1, 7
  %v3 = mul i64 %v2, %1
  ret i64 %v3
}

define dso_local i64 @f058(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ult i32 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f059(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ugt i16 %v, 5
  %cnt = zext i1 %cmp to i64
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f060(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = sub i32 %0, %0
  %v1 = shl i32 %0, 26
  %v2 = and i32 %v1, %1
  %v3 = lshr i32 %0, 28
  ret i32 %v3
}

define dso_local i64 @f061(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f062(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f063(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i32 %1, %1
  %v1 = xor i32 %v0, %0
  %v2 = sub i32 %1, %v1
  %v3 = add i32 %1, %v1
  %v4 = and i32 %v3, %v2
  ret i32 %v4
}

define dso_local i64 @f064(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ne i16 %v, 7
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f065(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f066(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = sub i32 %1, %0
  %v1 = lshr i32 %0, 12
  %v2 = or i32 %v1, %v1
  %v3 = mul i32 %v0, %0
  %v4 = and i32 %0, %v3
  ret i32 %v4
}

define dso_local i64 @f067(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f068(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f069(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %1, 2
  %v1 = or i64 %v0, %v0
  %v2 = sub i64 %0, %v0
  ret i64 %v2
}

define dso_local i64 @f070(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f071(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %nz = icmp ne i32 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i32 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 16
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f072(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = sub i64 %1, %1
  %v1 = mul i64 %v0, %v0
  %v2 = or i64 %1, %1
  %v3 = or i64 %0, %v0
  %v4 = xor i64 %v0, %v1
  ret i64 %v4
}

define dso_local i64 @f073(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f074(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %nz = icmp ne i32 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i32 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f075(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i64 %0, %1
  %v1 = add i64 %v0, %v0
  %v2 = xor i64 %1, %v1
  %v3 = mul i64 %v2, %v0
  ret i64 %v3
}

define dso_local i64 @f076(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp eq i32 %v, 4
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f077(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f078(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i32 %0, %0
  %v1 = sub i32 %v0, %0
  %v2 = add i32 %0, %v1
  %v3 = add i32 %0, %0
  %v4 = sub i32 %v1, %v0
  ret i32 %v4
}

define dso_local i64 @f079(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp eq i32 %v, 3
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f080(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp eq i32 %v, 5
  %cnt = zext i1 %cmp to i64
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f081(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i32 %1, 19
  %v1 = sub i32 %1, %0
  ret i32 %v1
}

define dso_local i64 @f082(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f083(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f084(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i32 %1, %1
  %v1 = and i32 %0, %0
  %v2 = shl i32 %0, 3
  %v3 = and i32 %v0, %v2
  %v4 = and i32 %v3, %1
  ret i32 %v4
}

define dso_local i64 @f085(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp uge i8 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f086(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f087(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = or i32 %0, %0
  %v1 = mul i32 %v0, %0
  %v2 = sub i32 %v0, %0
  ret i32 %v2
}

define dso_local i64 @f088(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ne i32 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f089(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f090(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i32 %1, 0
  %v1 = shl i32 %v0, 0
  %v2 = or i32 %v1, %v0
  %v3 = xor i32 %v0, %1
  %v4 = xor i32 %1, %v1
  ret i32 %v4
}

define dso_local i64 @f091(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ult i8 %v, 0
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f092(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %one = and i8 %v, 1
  %cond = icmp ne i8 %one, 0
  %zero = add i8 0, 0
  %sel = select i1 %cond, i8 %v, i8 %zero
  %sel64 = zext i8 %sel to i64
  %pre = add i8 0, 0
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f093(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %1, 20
  %v1 = mul i64 %0, %1
  %v2 = or i64 %v1, %v1
  ret i64 %v2
}

define dso_local i64 @f094(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ult i32 %v, 0
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f095(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %one = and i16 %v, 1
  %cond = icmp ne i16 %one, 0
  %zero = add i16 0, 0
  %sel = select i1 %cond, i16 %v, i16 %zero
  %sel64 = zext i16 %sel to i64
  %pre = add i16 0, 0
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f096(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i32 %1, %0
  %v1 = or i32 %0, %v0
  %v2 = or i32 %1, %0
  ret i32 %v2
}

define dso_local i64 @f097(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp uge i8 %v, 0
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f098(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %nz = icmp ne i8 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i8 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f099(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = or i32 %0, %0
  %v1 = or i32 %0, %v0
  ret i32 %v1
}

define dso_local i64 @f100(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f101(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f102(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i32 %0, 29
  %v1 = shl i32 %0, 19
  %v2 = shl i32 %v0, 26
  %v3 = mul i32 %0, %v2
  ret i32 %v3
}

define dso_local i64 @f103(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ne i8 %v, 5
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f104(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %one = and i8 %v, 1
  %cond = icmp ne i8 %one, 0
  %zero = add i8 0, 0
  %sel = select i1 %cond, i8 %v, i8 %zero
  %sel64 = zext i8 %sel to i64
  %pre = add i8 0, 0
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f105(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = mul i32 %1, %0
  %v1 = lshr i32 %1, 28
  %v2 = add i32 %1, %v0
  %v3 = xor i32 %0, %v2
  ret i32 %v3
}

define dso_local i64 @f106(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp uge i32 %v, 1
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f107(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %one = and i8 %v, 1
  %cond = icmp ne i8 %one, 0
  %zero = add i8 0, 0
  %sel = select i1 %cond, i8 %v, i8 %zero
  %sel64 = zext i8 %sel to i64
  %pre = add i8 0, 0
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f108(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i64 %1, %1
  %v1 = shl i64 %v0, 19
  %v2 = shl i64 %v1, 5
  ret i64 %v2
}

define dso_local i64 @f109(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f110(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %one = and i8 %v, 1
  %cond = icmp ne i8 %one, 0
  %zero = add i8 0, 0
  %sel = select i1 %cond, i8 %v, i8 %zero
  %sel64 = zext i8 %sel to i64
  %pre = add i8 0, 0
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f111(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = and i32 %1, %1
  %v1 = sub i32 %v0, %v0
  %v2 = shl i32 %v1, 8
  %v3 = mul i32 %v0, %v1
  %v4 = xor i32 %0, %v0
  ret i32 %v4
}

define dso_local i64 @f112(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f113(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f114(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i64 %1, 30
  %v1 = xor i64 %1, %0
  %v2 = xor i64 %v0, %0
  %v3 = sub i64 %v1, %0
  %v4 = and i64 %1, %v1
  ret i64 %v4
}

define dso_local i64 @f115(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f116(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %one = and i16 %v, 1
  %cond = icmp ne i16 %one, 0
  %zero = add i16 0, 0
  %sel = select i1 %cond, i16 %v, i16 %zero
  %sel64 = zext i16 %sel to i64
  %pre = add i16 0, 0
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 16
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f117(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i32 %0, 11
  %v1 = add i32 %v0, %1
  %v2 = or i32 %v1, %v0
  %v3 = and i32 %v2, %0
  ret i32 %v3
}

define dso_local i64 @f118(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp uge i32 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f119(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f120(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = and i64 %0, %1
  %v1 = add i64 %1, %v0
  %v2 = add i64 %1, %v1
  %v3 = sub i64 %v1, %0
  ret i64 %v3
}

define dso_local i64 @f121(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f122(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %one = and i16 %v, 1
  %cond = icmp ne i16 %one, 0
  %zero = add i16 0, 0
  %sel = select i1 %cond, i16 %v, i16 %zero
  %sel64 = zext i16 %sel to i64
  %pre = add i16 0, 0
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 16
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f123(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = or i64 %0, %0
  %v1 = shl i64 %v0, 30
  ret i64 %v1
}

define dso_local i64 @f124(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ne i16 %v, 6
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f125(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = or i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 48
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f126(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %1, 21
  %v1 = shl i64 %0, 6
  %v2 = shl i64 %v1, 0
  ret i64 %v2
}

define dso_local i64 @f127(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ugt i8 %v, 5
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f128(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f129(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = mul i32 %0, %1
  %v1 = mul i32 %0, %v0
  %v2 = or i32 %v1, %1
  %v3 = lshr i32 %v2, 13
  %v4 = sub i32 %1, %1
  ret i32 %v4
}

define dso_local i64 @f130(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ule i32 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f131(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 96
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f132(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i32 %0, 18
  %v1 = lshr i32 %1, 16
  %v2 = shl i32 %v0, 11
  ret i32 %v2
}

define dso_local i64 @f133(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ugt i16 %v, 6
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f134(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %nz = icmp ne i8 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i8 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f135(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = and i32 %1, %1
  %v1 = mul i32 %v0, %1
  ret i32 %v1
}

define dso_local i64 @f136(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f137(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %one = and i16 %v, 1
  %cond = icmp ne i16 %one, 0
  %zero = add i16 0, 0
  %sel = select i1 %cond, i16 %v, i16 %zero
  %sel64 = zext i16 %sel to i64
  %pre = add i16 0, 0
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f138(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = add i32 %1, %1
  %v1 = or i32 %v0, %1
  %v2 = and i32 %v0, %v1
  %v3 = and i32 %v2, %v2
  %v4 = xor i32 %v0, %v2
  ret i32 %v4
}

define dso_local i64 @f139(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f140(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp ule i8 %v, 7
  %cnt = zext i1 %cmp to i64
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %cnt
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 64
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f141(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i64 %0, 5
  %v1 = sub i64 %v0, %0
  %v2 = mul i64 %v0, %0
  %v3 = add i64 %v1, %0
  ret i64 %v3
}

define dso_local i64 @f142(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %cmp = icmp ule i32 %v, 6
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f143(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %nz = icmp ne i8 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i8 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f144(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = mul i64 %0, %1
  %v1 = or i64 %0, %v0
  ret i64 %v1
}

define dso_local i64 @f145(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %cmp = icmp eq i8 %v, 5
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f146(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 16
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f147(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = shl i32 %0, 17
  %v1 = or i32 %1, %0
  %v2 = and i32 %0, %v0
  ret i32 %v2
}

define dso_local i64 @f148(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f149(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %nz = icmp ne i8 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i8 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 16
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f150(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = lshr i32 %1, 15
  %v1 = sub i32 %1, %1
  %v2 = add i32 %0, %v1
  %v3 = add i32 %1, %v2
  ret i32 %v3
}

define dso_local i64 @f151(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %ve = zext i16 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f152(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %ve = zext i8 %v to i64
  %accnext = add i64 %acc, %ve
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f153(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = and i64 %1, %0
  %v1 = xor i64 %0, %v0
  %v2 = or i64 %v0, %v1
  ret i64 %v2
}

define dso_local i64 @f154(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp eq i16 %v, 0
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f155(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i32, ptr %0, i64 %i
  %v = load i32, ptr %p, align 4
  %one = and i32 %v, 1
  %cond = icmp ne i32 %one, 0
  %zero = add i32 0, 0
  %sel = select i1 %cond, i32 %v, i32 %zero
  %sel64 = zext i32 %sel to i64
  %pre = add i32 0, 0
  %ve = zext i32 %v to i64
  %accnext = add i64 %acc, %sel64
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 16
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i32 @f156(i32 noundef %0, i32 noundef %1) local_unnamed_addr {
entry:
  %v0 = xor i32 %0, %0
  %v1 = and i32 %1, %0
  ret i32 %v1
}

define dso_local i64 @f157(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
entry:
  %z = icmp eq i64 %1, 0
  br i1 %z, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %cmp = icmp ule i16 %v, 2
  %bit = zext i1 %cmp to i64
  %accnext = add i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, %1
  br i1 %term, label %exit, label %loop
exit:
  %result = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  ret i64 %result
}

define dso_local i64 @f158(ptr nocapture noundef readonly %0) local_unnamed_addr {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %accnext, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %v = load i16, ptr %p, align 2
  %nz = icmp ne i16 %v, 0
  %bit = zext i1 %nz to i64
  %ve = zext i16 %v to i64
  %accnext = xor i64 %acc, %bit
  %inext = add nuw nsw i64 %i, 1
  %term = icmp eq i64 %inext, 32
  br i1 %term, label %exit, label %loop
exit:
  ret i64 %accnext
}

define dso_local i64 @f159(i64 noundef %0, i64 noundef %1) local_unnamed_addr {
entry:
  %v0 = mul i64 %0, %1
  %v1 = and i64 %1, %0
  %v2 = mul i64 %0, %v1
  ret i64 %v2
}

