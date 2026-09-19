; Gate 6A-v14 — pre-registered native coverage for search role binding.
; Frozen with v14_expected.csv and v14_manifest.sha256 before evaluation.

; Search for zero in the first array while also accessing a second array of
; the same promoted extent. The collection role must bind the searched array.
define dso_local i64 @s01_search_a_while_accessing_b(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr {
entry:
  br label %loop

loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %latch ]
  %pa = getelementptr inbounds i8, ptr %0, i64 %i
  %va = load i8, ptr %pa, align 1
  %pb = getelementptr inbounds i8, ptr %1, i64 %i
  %vb = load i8, ptr %pb, align 1
  %unused = xor i8 %vb, %vb
  %hit = icmp eq i8 %va, 0
  br i1 %hit, label %found, label %latch

latch:
  %inext = add nuw nsw i64 %i, 1
  %bound = icmp eq i64 %inext, 64
  br i1 %bound, label %found, label %loop

found:
  %res = phi i64 [ 64, %latch ], [ %i, %loop ]
  ret i64 %res
}

; Equality search with a trailing extra integer parameter. The scalar role
; must bind the compared key, not the last integer parameter.
define dso_local i64 @s02_eq_key_with_extra(ptr nocapture noundef readonly %0, i8 noundef zeroext %1, i8 noundef zeroext %2) local_unnamed_addr {
entry:
  br label %loop

loop:
  %i = phi i64 [ 0, %entry ], [ %inext, %latch ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %v = load i8, ptr %p, align 1
  %hit = icmp eq i8 %v, %1
  br i1 %hit, label %found, label %latch

latch:
  %inext = add nuw nsw i64 %i, 1
  %bound = icmp eq i64 %inext, 64
  br i1 %bound, label %found, label %loop

found:
  %res = phi i64 [ 64, %latch ], [ %i, %loop ]
  ret i64 %res
}
