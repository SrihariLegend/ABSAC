	.text
	.file	"orig_kernels.c"
	.globl	k18_all_equal                   # -- Begin function k18_all_equal
	.p2align	4, 0x90
	.type	k18_all_equal,@function
k18_all_equal:                          # @k18_all_equal
	.cfi_startproc
# %bb.0:
	testq	%rsi, %rsi
	je	.LBB0_1
# %bb.2:
	cmpq	$15, %rsi
	ja	.LBB0_6
# %bb.3:
	movl	$1, %eax
	xorl	%ecx, %ecx
	jmp	.LBB0_4
.LBB0_1:
	movl	$1, %eax
	retq
.LBB0_6:
	movq	%rsi, %rcx
	andq	$-16, %rcx
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %xmm1
	vpxor	%xmm0, %xmm0, %xmm0
	xorl	%eax, %eax
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpxor	%xmm3, %xmm3, %xmm3
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	.p2align	4, 0x90
.LBB0_7:                                # =>This Inner Loop Header: Depth=1
	vmovd	(%rdi,%rax), %xmm6              # xmm6 = mem[0],zero,zero,zero
	vmovd	4(%rdi,%rax), %xmm7             # xmm7 = mem[0],zero,zero,zero
	vmovd	8(%rdi,%rax), %xmm8             # xmm8 = mem[0],zero,zero,zero
	vmovd	12(%rdi,%rax), %xmm9            # xmm9 = mem[0],zero,zero,zero
	vpcmpeqb	%xmm1, %xmm6, %xmm6
	vpxor	%xmm2, %xmm6, %xmm6
	vpmovsxbd	%xmm6, %xmm6
	vpor	%xmm6, %xmm0, %xmm0
	vpcmpeqb	%xmm1, %xmm7, %xmm6
	vpxor	%xmm2, %xmm6, %xmm6
	vpmovsxbd	%xmm6, %xmm6
	vpor	%xmm6, %xmm3, %xmm3
	vpcmpeqb	%xmm1, %xmm8, %xmm6
	vpxor	%xmm2, %xmm6, %xmm6
	vpmovsxbd	%xmm6, %xmm6
	vpor	%xmm6, %xmm4, %xmm4
	vpcmpeqb	%xmm1, %xmm9, %xmm6
	vpxor	%xmm2, %xmm6, %xmm6
	vpmovsxbd	%xmm6, %xmm6
	vpor	%xmm6, %xmm5, %xmm5
	addq	$16, %rax
	cmpq	%rax, %rcx
	jne	.LBB0_7
# %bb.8:
	vpor	%xmm0, %xmm3, %xmm0
	vpor	%xmm0, %xmm4, %xmm0
	vpor	%xmm0, %xmm5, %xmm0
	vpslld	$31, %xmm0, %xmm0
	vmovmskps	%xmm0, %r8d
	xorl	%eax, %eax
	testl	%r8d, %r8d
	sete	%al
	cmpq	%rsi, %rcx
	je	.LBB0_9
.LBB0_4:
	xorl	%r8d, %r8d
	.p2align	4, 0x90
.LBB0_5:                                # =>This Inner Loop Header: Depth=1
	cmpb	%dl, (%rdi,%rcx)
	cmovneq	%r8, %rax
	incq	%rcx
	cmpq	%rcx, %rsi
	jne	.LBB0_5
.LBB0_9:
	retq
.Lfunc_end0:
	.size	k18_all_equal, .Lfunc_end0-k18_all_equal
	.cfi_endproc
                                        # -- End function
	.globl	k43_sum_ascii                   # -- Begin function k43_sum_ascii
	.p2align	4, 0x90
	.type	k43_sum_ascii,@function
k43_sum_ascii:                          # @k43_sum_ascii
	.cfi_startproc
# %bb.0:
	testq	%rsi, %rsi
	je	.LBB1_1
# %bb.2:
	cmpq	$15, %rsi
	ja	.LBB1_4
# %bb.3:
	xorl	%ecx, %ecx
	xorl	%eax, %eax
	jmp	.LBB1_7
.LBB1_1:
	xorl	%eax, %eax
	retq
.LBB1_4:
	movq	%rsi, %rcx
	andq	$-16, %rcx
	vpxor	%xmm0, %xmm0, %xmm0
	xorl	%eax, %eax
	vpxor	%xmm1, %xmm1, %xmm1
	vpxor	%xmm2, %xmm2, %xmm2
	vpxor	%xmm3, %xmm3, %xmm3
	.p2align	4, 0x90
.LBB1_5:                                # =>This Inner Loop Header: Depth=1
	vpmovzxbq	(%rdi,%rax), %ymm4      # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm0, %ymm0
	vpmovzxbq	4(%rdi,%rax), %ymm4     # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm1, %ymm1
	vpmovzxbq	8(%rdi,%rax), %ymm4     # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm2, %ymm2
	vpmovzxbq	12(%rdi,%rax), %ymm4    # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm3, %ymm3
	addq	$16, %rax
	cmpq	%rax, %rcx
	jne	.LBB1_5
# %bb.6:
	vpaddq	%ymm0, %ymm1, %ymm0
	vpaddq	%ymm0, %ymm2, %ymm0
	vpaddq	%ymm0, %ymm3, %ymm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1              # xmm1 = xmm0[2,3,2,3]
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rax
	cmpq	%rsi, %rcx
	je	.LBB1_8
	.p2align	4, 0x90
.LBB1_7:                                # =>This Inner Loop Header: Depth=1
	movzbl	(%rdi,%rcx), %edx
	addq	%rdx, %rax
	incq	%rcx
	cmpq	%rcx, %rsi
	jne	.LBB1_7
.LBB1_8:
	vzeroupper
	retq
.Lfunc_end1:
	.size	k43_sum_ascii, .Lfunc_end1-k43_sum_ascii
	.cfi_endproc
                                        # -- End function
	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3, 0x0                          # -- Begin function k50_count_masked
.LCPI2_0:
	.quad	1                               # 0x1
	.text
	.globl	k50_count_masked
	.p2align	4, 0x90
	.type	k50_count_masked,@function
k50_count_masked:                       # @k50_count_masked
	.cfi_startproc
# %bb.0:
	testq	%rsi, %rsi
	je	.LBB2_1
# %bb.3:
	cmpq	$15, %rsi
	ja	.LBB2_5
# %bb.4:
	xorl	%r8d, %r8d
	xorl	%eax, %eax
	jmp	.LBB2_8
.LBB2_1:
	xorl	%eax, %eax
	retq
.LBB2_5:
	movq	%rsi, %r8
	andq	$-16, %r8
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %xmm0
	vmovd	%ecx, %xmm1
	vpbroadcastb	%xmm1, %xmm2
	vpxor	%xmm1, %xmm1, %xmm1
	xorl	%eax, %eax
	vpbroadcastq	.LCPI2_0(%rip), %ymm3   # ymm3 = [1,1,1,1]
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	.p2align	4, 0x90
.LBB2_6:                                # =>This Inner Loop Header: Depth=1
	vmovd	(%rdi,%rax), %xmm7              # xmm7 = mem[0],zero,zero,zero
	vmovd	4(%rdi,%rax), %xmm8             # xmm8 = mem[0],zero,zero,zero
	vmovd	8(%rdi,%rax), %xmm9             # xmm9 = mem[0],zero,zero,zero
	vmovd	12(%rdi,%rax), %xmm10           # xmm10 = mem[0],zero,zero,zero
	vpand	%xmm0, %xmm7, %xmm7
	vpand	%xmm0, %xmm8, %xmm8
	vpand	%xmm0, %xmm9, %xmm9
	vpand	%xmm0, %xmm10, %xmm10
	vpcmpeqb	%xmm2, %xmm7, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm3, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm1, %ymm1
	vpcmpeqb	%xmm2, %xmm8, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm3, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm4, %ymm4
	vpcmpeqb	%xmm2, %xmm9, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm3, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm5, %ymm5
	vpcmpeqb	%xmm2, %xmm10, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm3, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm6, %ymm6
	addq	$16, %rax
	cmpq	%rax, %r8
	jne	.LBB2_6
# %bb.7:
	vpaddq	%ymm1, %ymm4, %ymm0
	vpaddq	%ymm0, %ymm5, %ymm0
	vpaddq	%ymm0, %ymm6, %ymm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1              # xmm1 = xmm0[2,3,2,3]
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rax
	cmpq	%rsi, %r8
	je	.LBB2_2
	.p2align	4, 0x90
.LBB2_8:                                # =>This Inner Loop Header: Depth=1
	movzbl	(%rdi,%r8), %r9d
	andb	%dl, %r9b
	xorl	%r10d, %r10d
	cmpb	%cl, %r9b
	sete	%r10b
	addq	%r10, %rax
	incq	%r8
	cmpq	%r8, %rsi
	jne	.LBB2_8
.LBB2_2:
	vzeroupper
	retq
.Lfunc_end2:
	.size	k50_count_masked, .Lfunc_end2-k50_count_masked
	.cfi_endproc
                                        # -- End function
	.ident	"Debian clang version 19.1.7 (3+b1)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
