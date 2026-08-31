	.text
	.file	"k18_all_equal_compile.c"
	.globl	k18_all_equal                   # -- Begin function k18_all_equal
	.p2align	4, 0x90
	.type	k18_all_equal,@function
k18_all_equal:                          # @k18_all_equal
	.cfi_startproc
# %bb.0:
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %ymm0
	xorl	%eax, %eax
	xorl	%ecx, %ecx
	.p2align	4, 0x90
.LBB0_1:                                # =>This Inner Loop Header: Depth=1
	leaq	32(%rcx), %r8
	cmpq	%rsi, %r8
	ja	.LBB0_2
# %bb.5:                                #   in Loop: Header=BB0_1 Depth=1
	vpxor	(%rdi,%rcx), %ymm0, %ymm1
	vptest	%ymm1, %ymm1
	movq	%r8, %rcx
	je	.LBB0_1
.LBB0_11:
	vzeroupper
	retq
.LBB0_2:
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %xmm0
	xorl	%eax, %eax
	.p2align	4, 0x90
.LBB0_3:                                # =>This Inner Loop Header: Depth=1
	leaq	16(%rcx), %r8
	cmpq	%rsi, %r8
	ja	.LBB0_4
# %bb.6:                                #   in Loop: Header=BB0_3 Depth=1
	vpxor	(%rdi,%rcx), %xmm0, %xmm1
	vptest	%xmm1, %xmm1
	movq	%r8, %rcx
	je	.LBB0_3
	jmp	.LBB0_11
.LBB0_4:
	movl	$1, %eax
	cmpq	%rsi, %rcx
	jae	.LBB0_11
	.p2align	4, 0x90
.LBB0_9:                                # =>This Inner Loop Header: Depth=1
	cmpb	%dl, (%rdi,%rcx)
	jne	.LBB0_10
# %bb.7:                                #   in Loop: Header=BB0_9 Depth=1
	incq	%rcx
	cmpq	%rsi, %rcx
	jb	.LBB0_9
	jmp	.LBB0_11
.LBB0_10:
	xorl	%eax, %eax
	vzeroupper
	retq
.Lfunc_end0:
	.size	k18_all_equal, .Lfunc_end0-k18_all_equal
	.cfi_endproc
                                        # -- End function
	.ident	"Debian clang version 19.1.7 (3+b1)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
