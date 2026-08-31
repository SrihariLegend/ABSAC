	.text
	.file	"k50_count_masked_compile.c"
	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3, 0x0                          # -- Begin function k50_count_masked
.LCPI0_0:
	.quad	1                               # 0x1
	.text
	.globl	k50_count_masked
	.p2align	4, 0x90
	.type	k50_count_masked,@function
k50_count_masked:                       # @k50_count_masked
	.cfi_startproc
# %bb.0:
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	cmpq	$32, %rsi
	jae	.LBB0_2
# %bb.1:
	xorl	%r8d, %r8d
	xorl	%eax, %eax
	movq	%r8, %r9
	orq	$16, %r9
	cmpq	%rsi, %r9
	ja	.LBB0_9
	jmp	.LBB0_19
.LBB0_2:
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %ymm0
	vmovd	%ecx, %xmm1
	vpbroadcastb	%xmm1, %ymm1
	leaq	-32(%rsi), %rax
	movq	%rax, %r10
	shrq	$5, %r10
	incq	%r10
	movl	%r10d, %r9d
	andl	$7, %r9d
	cmpq	$224, %rax
	jae	.LBB0_17
# %bb.3:
	movl	$32, %r10d
	xorl	%eax, %eax
	xorl	%r8d, %r8d
	testq	%r9, %r9
	jne	.LBB0_6
.LBB0_8:
	movq	%r8, %r9
	orq	$16, %r9
	cmpq	%rsi, %r9
	jbe	.LBB0_19
.LBB0_9:
	movq	%r8, %r9
	jmp	.LBB0_10
.LBB0_17:
	andq	$-8, %r10
	xorl	%r8d, %r8d
	xorl	%eax, %eax
	.p2align	4, 0x90
.LBB0_18:                               # =>This Inner Loop Header: Depth=1
	vpand	(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %r11d
	popcntl	%r11d, %r11d
	addq	%rax, %r11
	vpand	32(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %eax
	popcntl	%eax, %eax
	vpand	64(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %ebx
	popcntl	%ebx, %ebx
	addq	%rax, %rbx
	addq	%r11, %rbx
	vpand	96(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %eax
	popcntl	%eax, %eax
	vpand	128(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %r11d
	popcntl	%r11d, %r11d
	addq	%rax, %r11
	vpand	160(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %eax
	xorl	%r14d, %r14d
	popcntl	%eax, %r14d
	addq	%r11, %r14
	addq	%rbx, %r14
	vpand	192(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %eax
	xorl	%r11d, %r11d
	popcntl	%eax, %r11d
	vpand	224(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %eax
	popcntl	%eax, %eax
	addq	%r11, %rax
	addq	%r14, %rax
	addq	$256, %r8                       # imm = 0x100
	addq	$-8, %r10
	jne	.LBB0_18
# %bb.4:
	leaq	32(%r8), %r10
	testq	%r9, %r9
	je	.LBB0_8
	.p2align	4, 0x90
.LBB0_6:                                # =>This Inner Loop Header: Depth=1
	vpand	(%rdi,%r8), %ymm0, %ymm2
	vpcmpeqb	%ymm1, %ymm2, %ymm2
	vpmovmskb	%ymm2, %r8d
	popcntl	%r8d, %r8d
	addq	%r8, %rax
	movq	%r10, %r8
	addq	$32, %r10
	decq	%r9
	jne	.LBB0_6
# %bb.7:
	addq	$-32, %r10
	movq	%r10, %r8
	movq	%r8, %r9
	orq	$16, %r9
	cmpq	%rsi, %r9
	ja	.LBB0_9
.LBB0_19:
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %xmm0
	vmovd	%ecx, %xmm1
	vpbroadcastb	%xmm1, %xmm1
	.p2align	4, 0x90
.LBB0_20:                               # =>This Inner Loop Header: Depth=1
	vpand	(%rdi,%r8), %xmm0, %xmm2
	vpcmpeqb	%xmm1, %xmm2, %xmm2
	vpmovmskb	%xmm2, %r9d
	popcntl	%r9d, %r9d
	addq	%r9, %rax
	leaq	16(%r8), %r9
	addq	$32, %r8
	cmpq	%rsi, %r8
	movq	%r9, %r8
	jbe	.LBB0_20
.LBB0_10:
	movq	%rsi, %r8
	subq	%r9, %r8
	jbe	.LBB0_16
# %bb.11:
	cmpq	$15, %r8
	jbe	.LBB0_15
# %bb.12:
	movq	%r8, %r10
	andq	$-16, %r10
	vmovq	%rax, %xmm0
	vmovd	%edx, %xmm1
	vpbroadcastb	%xmm1, %xmm1
	vmovd	%ecx, %xmm2
	vpbroadcastb	%xmm2, %xmm2
	leaq	(%r9,%rdi), %rax
	addq	$12, %rax
	addq	%r10, %r9
	vpxor	%xmm3, %xmm3, %xmm3
	xorl	%r11d, %r11d
	vpbroadcastq	.LCPI0_0(%rip), %ymm4   # ymm4 = [1,1,1,1]
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	.p2align	4, 0x90
.LBB0_13:                               # =>This Inner Loop Header: Depth=1
	vmovd	-12(%rax,%r11), %xmm7           # xmm7 = mem[0],zero,zero,zero
	vmovd	-8(%rax,%r11), %xmm8            # xmm8 = mem[0],zero,zero,zero
	vmovd	-4(%rax,%r11), %xmm9            # xmm9 = mem[0],zero,zero,zero
	vmovd	(%rax,%r11), %xmm10             # xmm10 = mem[0],zero,zero,zero
	vpand	%xmm1, %xmm7, %xmm7
	vpand	%xmm1, %xmm8, %xmm8
	vpand	%xmm1, %xmm9, %xmm9
	vpand	%xmm1, %xmm10, %xmm10
	vpcmpeqb	%xmm2, %xmm7, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm4, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm0, %ymm0
	vpcmpeqb	%xmm2, %xmm8, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm4, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm3, %ymm3
	vpcmpeqb	%xmm2, %xmm9, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm4, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm5, %ymm5
	vpcmpeqb	%xmm2, %xmm10, %xmm7
	vpmovzxbq	%xmm7, %ymm7            # ymm7 = xmm7[0],zero,zero,zero,zero,zero,zero,zero,xmm7[1],zero,zero,zero,zero,zero,zero,zero,xmm7[2],zero,zero,zero,zero,zero,zero,zero,xmm7[3],zero,zero,zero,zero,zero,zero,zero
	vpand	%ymm4, %ymm7, %ymm7
	vpaddq	%ymm7, %ymm6, %ymm6
	addq	$16, %r11
	cmpq	%r11, %r10
	jne	.LBB0_13
# %bb.14:
	vpaddq	%ymm0, %ymm3, %ymm0
	vpaddq	%ymm0, %ymm5, %ymm0
	vpaddq	%ymm0, %ymm6, %ymm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1              # xmm1 = xmm0[2,3,2,3]
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rax
	cmpq	%r10, %r8
	je	.LBB0_16
	.p2align	4, 0x90
.LBB0_15:                               # =>This Inner Loop Header: Depth=1
	movzbl	(%rdi,%r9), %r8d
	andb	%dl, %r8b
	xorl	%r10d, %r10d
	cmpb	%cl, %r8b
	sete	%r10b
	addq	%r10, %rax
	incq	%r9
	cmpq	%r9, %rsi
	jne	.LBB0_15
.LBB0_16:
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.Lfunc_end0:
	.size	k50_count_masked, .Lfunc_end0-k50_count_masked
	.cfi_endproc
                                        # -- End function
	.ident	"Debian clang version 19.1.7 (3+b1)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
