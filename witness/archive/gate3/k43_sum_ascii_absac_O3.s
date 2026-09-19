	.text
	.file	"k43_sum_ascii_compile.c"
	.globl	k43_sum_ascii                   # -- Begin function k43_sum_ascii
	.p2align	4, 0x90
	.type	k43_sum_ascii,@function
k43_sum_ascii:                          # @k43_sum_ascii
	.cfi_startproc
# %bb.0:
	cmpq	$32, %rsi
	jae	.LBB0_2
# %bb.1:
	vpxor	%xmm0, %xmm0, %xmm0
	xorl	%ecx, %ecx
	jmp	.LBB0_11
.LBB0_2:
	leaq	-32(%rsi), %rcx
	movq	%rcx, %rdx
	shrq	$5, %rdx
	incq	%rdx
	movl	%edx, %eax
	andl	$7, %eax
	cmpq	$224, %rcx
	jae	.LBB0_4
# %bb.3:
	vpxor	%xmm0, %xmm0, %xmm0
	movl	$32, %edx
	xorl	%ecx, %ecx
	testq	%rax, %rax
	jne	.LBB0_8
	jmp	.LBB0_11
.LBB0_4:
	andq	$-8, %rdx
	vpxor	%xmm1, %xmm1, %xmm1
	xorl	%ecx, %ecx
	vpxor	%xmm0, %xmm0, %xmm0
	.p2align	4, 0x90
.LBB0_5:                                # =>This Inner Loop Header: Depth=1
	vpsadbw	(%rdi,%rcx), %ymm1, %ymm2
	vpaddq	%ymm0, %ymm2, %ymm0
	vpsadbw	32(%rdi,%rcx), %ymm1, %ymm2
	vpsadbw	64(%rdi,%rcx), %ymm1, %ymm3
	vpaddq	%ymm2, %ymm3, %ymm2
	vpaddq	%ymm0, %ymm2, %ymm0
	vpsadbw	96(%rdi,%rcx), %ymm1, %ymm2
	vpsadbw	128(%rdi,%rcx), %ymm1, %ymm3
	vpaddq	%ymm2, %ymm3, %ymm2
	vpsadbw	160(%rdi,%rcx), %ymm1, %ymm3
	vpaddq	%ymm2, %ymm3, %ymm2
	vpaddq	%ymm0, %ymm2, %ymm0
	vpsadbw	192(%rdi,%rcx), %ymm1, %ymm2
	vpsadbw	224(%rdi,%rcx), %ymm1, %ymm3
	vpaddq	%ymm2, %ymm3, %ymm2
	vpaddq	%ymm0, %ymm2, %ymm0
	addq	$256, %rcx                      # imm = 0x100
	addq	$-8, %rdx
	jne	.LBB0_5
# %bb.6:
	leaq	32(%rcx), %rdx
	testq	%rax, %rax
	je	.LBB0_11
.LBB0_8:
	vpxor	%xmm1, %xmm1, %xmm1
	.p2align	4, 0x90
.LBB0_9:                                # =>This Inner Loop Header: Depth=1
	vpsadbw	(%rdi,%rcx), %ymm1, %ymm2
	vpaddq	%ymm0, %ymm2, %ymm0
	movq	%rdx, %rcx
	addq	$32, %rdx
	decq	%rax
	jne	.LBB0_9
# %bb.10:
	addq	$-32, %rdx
	movq	%rdx, %rcx
.LBB0_11:
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1              # xmm1 = xmm0[2,3,2,3]
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rax
	movq	%rcx, %rdx
	orq	$16, %rdx
	cmpq	%rsi, %rdx
	jbe	.LBB0_20
# %bb.12:
	movq	%rcx, %rdx
	jmp	.LBB0_13
.LBB0_20:
	vpxor	%xmm0, %xmm0, %xmm0
	.p2align	4, 0x90
.LBB0_21:                               # =>This Inner Loop Header: Depth=1
	vpsadbw	(%rdi,%rcx), %xmm0, %xmm1
	vmovq	%xmm1, %rdx
	addq	%rax, %rdx
	vpextrq	$1, %xmm1, %rax
	addq	%rdx, %rax
	leaq	16(%rcx), %rdx
	addq	$32, %rcx
	cmpq	%rsi, %rcx
	movq	%rdx, %rcx
	jbe	.LBB0_21
.LBB0_13:
	movq	%rsi, %rcx
	subq	%rdx, %rcx
	jbe	.LBB0_19
# %bb.14:
	cmpq	$15, %rcx
	jbe	.LBB0_18
# %bb.15:
	movq	%rcx, %r8
	andq	$-16, %r8
	vmovq	%rax, %xmm0
	leaq	(%rdx,%rdi), %rax
	addq	$12, %rax
	addq	%r8, %rdx
	vpxor	%xmm1, %xmm1, %xmm1
	xorl	%r9d, %r9d
	vpxor	%xmm2, %xmm2, %xmm2
	vpxor	%xmm3, %xmm3, %xmm3
	.p2align	4, 0x90
.LBB0_16:                               # =>This Inner Loop Header: Depth=1
	vpmovzxbq	-12(%rax,%r9), %ymm4    # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm0, %ymm0
	vpmovzxbq	-8(%rax,%r9), %ymm4     # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm1, %ymm1
	vpmovzxbq	-4(%rax,%r9), %ymm4     # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm2, %ymm2
	vpmovzxbq	(%rax,%r9), %ymm4       # ymm4 = mem[0],zero,zero,zero,zero,zero,zero,zero,mem[1],zero,zero,zero,zero,zero,zero,zero,mem[2],zero,zero,zero,zero,zero,zero,zero,mem[3],zero,zero,zero,zero,zero,zero,zero
	vpaddq	%ymm4, %ymm3, %ymm3
	addq	$16, %r9
	cmpq	%r9, %r8
	jne	.LBB0_16
# %bb.17:
	vpaddq	%ymm0, %ymm1, %ymm0
	vpaddq	%ymm0, %ymm2, %ymm0
	vpaddq	%ymm0, %ymm3, %ymm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1              # xmm1 = xmm0[2,3,2,3]
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rax
	cmpq	%r8, %rcx
	je	.LBB0_19
	.p2align	4, 0x90
.LBB0_18:                               # =>This Inner Loop Header: Depth=1
	movzbl	(%rdi,%rdx), %ecx
	addq	%rcx, %rax
	incq	%rdx
	cmpq	%rdx, %rsi
	jne	.LBB0_18
.LBB0_19:
	vzeroupper
	retq
.Lfunc_end0:
	.size	k43_sum_ascii, .Lfunc_end0-k43_sum_ascii
	.cfi_endproc
                                        # -- End function
	.ident	"Debian clang version 19.1.7 (3+b1)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
