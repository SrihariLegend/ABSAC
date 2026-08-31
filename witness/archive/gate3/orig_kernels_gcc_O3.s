	.file	"orig_kernels.c"
	.text
	.p2align 4
	.globl	k18_all_equal
	.type	k18_all_equal, @function
k18_all_equal:
.LFB0:
	.cfi_startproc
	testq	%rsi, %rsi
	movl	%edx, %r8d
	je	.L9
	leaq	-1(%rsi), %rax
	cmpq	$30, %rax
	jbe	.L10
	movq	%rsi, %rcx
	movl	$16843009, %r9d
	vmovd	%edx, %xmm5
	andq	$-32, %rcx
	vmovd	%r9d, %xmm4
	vpbroadcastb	%xmm5, %ymm5
	movq	%rdi, %rax
	leaq	(%rcx,%rdi), %rdx
	vpcmpeqd	%ymm3, %ymm3, %ymm3
	vpbroadcastd	%xmm4, %ymm4
	.p2align 4,,10
	.p2align 3
.L4:
	vpcmpeqb	(%rax), %ymm5, %ymm0
	addq	$32, %rax
	cmpq	%rax, %rdx
	vpand	%ymm4, %ymm0, %ymm0
	vpmovzxbw	%xmm0, %ymm2
	vextracti128	$0x1, %ymm0, %xmm0
	vpmovzxwd	%xmm2, %ymm7
	vpmovzxbw	%xmm0, %ymm0
	vpmovzxwd	%xmm0, %ymm6
	vpmovzxdq	%xmm7, %ymm1
	vextracti128	$0x1, %ymm0, %xmm0
	vextracti128	$0x1, %ymm7, %xmm7
	vpmovzxwd	%xmm0, %ymm0
	vpmovzxdq	%xmm7, %ymm7
	vpand	%ymm7, %ymm1, %ymm1
	vpmovzxdq	%xmm0, %ymm7
	vpand	%ymm7, %ymm1, %ymm1
	vpmovzxdq	%xmm6, %ymm7
	vextracti128	$0x1, %ymm6, %xmm6
	vextracti128	$0x1, %ymm2, %xmm2
	vpmovzxdq	%xmm6, %ymm6
	vpmovzxwd	%xmm2, %ymm2
	vpand	%ymm6, %ymm7, %ymm6
	vpand	%ymm6, %ymm1, %ymm1
	vpmovzxdq	%xmm2, %ymm6
	vextracti128	$0x1, %ymm2, %xmm2
	vpmovzxdq	%xmm2, %ymm2
	vextracti128	$0x1, %ymm0, %xmm0
	vpand	%ymm2, %ymm6, %ymm2
	vpmovzxdq	%xmm0, %ymm0
	vpand	%ymm0, %ymm2, %ymm0
	vpand	%ymm0, %ymm1, %ymm0
	vpand	%ymm0, %ymm3, %ymm3
	jne	.L4
	vextracti128	$0x1, %ymm3, %xmm0
	vpand	%xmm3, %xmm0, %xmm3
	vpsrldq	$8, %xmm3, %xmm0
	vpand	%xmm0, %xmm3, %xmm0
	vmovq	%xmm0, %rax
	andl	$1, %eax
	cmpq	%rcx, %rsi
	je	.L17
	vzeroupper
.L3:
	movq	%rsi, %rdx
	subq	%rcx, %rdx
	leaq	-1(%rdx), %r9
	cmpq	$14, %r9
	jbe	.L7
	vmovd	%r8d, %xmm0
	vpbroadcastb	%xmm0, %xmm0
	vpcmpeqb	(%rdi,%rcx), %xmm0, %xmm0
	movl	$16843009, %eax
	vmovd	%eax, %xmm1
	vpbroadcastd	%xmm1, %xmm1
	vpand	%xmm1, %xmm0, %xmm0
	vpmovzxbw	%xmm0, %xmm2
	vpmovzxwd	%xmm2, %xmm5
	vpsrldq	$8, %xmm0, %xmm0
	vpsrldq	$8, %xmm2, %xmm2
	vpmovzxbw	%xmm0, %xmm0
	vpmovzxwd	%xmm2, %xmm2
	vpmovzxwd	%xmm0, %xmm4
	vpmovzxdq	%xmm2, %xmm1
	vpsrldq	$8, %xmm0, %xmm0
	vpsrldq	$8, %xmm2, %xmm2
	vpmovzxwd	%xmm0, %xmm0
	vpmovzxdq	%xmm2, %xmm2
	vpand	%xmm2, %xmm1, %xmm1
	vpmovzxdq	%xmm0, %xmm2
	vpand	%xmm2, %xmm1, %xmm1
	vpmovzxdq	%xmm5, %xmm2
	vpsrldq	$8, %xmm5, %xmm5
	vpmovzxdq	%xmm5, %xmm5
	vpsrldq	$8, %xmm0, %xmm0
	vpand	%xmm5, %xmm2, %xmm2
	vpmovzxdq	%xmm0, %xmm0
	vpand	%xmm0, %xmm2, %xmm0
	vpand	%xmm0, %xmm1, %xmm0
	vpmovzxdq	%xmm4, %xmm1
	vpsrldq	$8, %xmm4, %xmm4
	vpmovzxdq	%xmm4, %xmm4
	vpand	%xmm4, %xmm1, %xmm1
	vpand	%xmm3, %xmm1, %xmm1
	vpand	%xmm1, %xmm0, %xmm0
	vpsrldq	$8, %xmm0, %xmm1
	movq	%rdx, %r9
	vpand	%xmm1, %xmm0, %xmm0
	andq	$-16, %r9
	vmovq	%xmm0, %rax
	andl	$1, %eax
	addq	%r9, %rcx
	andl	$15, %edx
	je	.L1
.L7:
	xorl	%edx, %edx
	cmpb	%r8b, (%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	1(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 1(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	2(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 2(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	3(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 3(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	4(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 4(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	5(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 5(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	6(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 6(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	7(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 7(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	8(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 8(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	9(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 9(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	10(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 10(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	11(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 11(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	12(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 12(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	13(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 13(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	leaq	14(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L1
	xorl	%edx, %edx
	cmpb	%r8b, 14(%rdi,%rcx)
	sete	%dl
	andq	%rdx, %rax
	ret
	.p2align 4,,10
	.p2align 3
.L9:
	movl	$1, %eax
.L1:
	ret
.L10:
	vpcmpeqd	%xmm3, %xmm3, %xmm3
	xorl	%ecx, %ecx
	movl	$1, %eax
	jmp	.L3
.L17:
	vzeroupper
	ret
	.cfi_endproc
.LFE0:
	.size	k18_all_equal, .-k18_all_equal
	.p2align 4
	.globl	k43_sum_ascii
	.type	k43_sum_ascii, @function
k43_sum_ascii:
.LFB1:
	.cfi_startproc
	testq	%rsi, %rsi
	je	.L26
	leaq	-1(%rsi), %rax
	cmpq	$30, %rax
	jbe	.L27
	movq	%rsi, %rcx
	andq	$-32, %rcx
	movq	%rdi, %rax
	leaq	(%rcx,%rdi), %rdx
	vpxor	%xmm5, %xmm5, %xmm5
	.p2align 4,,10
	.p2align 3
.L21:
	vmovdqu	(%rax), %ymm0
	addq	$32, %rax
	vpmovzxbw	%xmm0, %ymm1
	vextracti128	$0x1, %ymm0, %xmm0
	vpmovzxwd	%xmm1, %ymm4
	vpmovzxbw	%xmm0, %ymm0
	vpmovzxwd	%xmm0, %ymm3
	vpmovzxdq	%xmm4, %ymm2
	vextracti128	$0x1, %ymm0, %xmm0
	vextracti128	$0x1, %ymm4, %xmm4
	vpmovzxwd	%xmm0, %ymm0
	vpmovzxdq	%xmm4, %ymm4
	vpaddq	%ymm4, %ymm2, %ymm2
	vpmovzxdq	%xmm0, %ymm4
	vpaddq	%ymm4, %ymm2, %ymm2
	vpmovzxdq	%xmm3, %ymm4
	vextracti128	$0x1, %ymm3, %xmm3
	vextracti128	$0x1, %ymm1, %xmm1
	vpmovzxdq	%xmm3, %ymm3
	vpmovzxwd	%xmm1, %ymm1
	vpaddq	%ymm3, %ymm4, %ymm3
	vpaddq	%ymm3, %ymm2, %ymm2
	vpmovzxdq	%xmm1, %ymm3
	vextracti128	$0x1, %ymm1, %xmm1
	vpmovzxdq	%xmm1, %ymm1
	vextracti128	$0x1, %ymm0, %xmm0
	vpaddq	%ymm1, %ymm3, %ymm1
	vpmovzxdq	%xmm0, %ymm0
	vpaddq	%ymm0, %ymm1, %ymm0
	vpaddq	%ymm0, %ymm2, %ymm0
	cmpq	%rdx, %rax
	vpaddq	%ymm0, %ymm5, %ymm5
	jne	.L21
	vextracti128	$0x1, %ymm5, %xmm3
	vpaddq	%xmm5, %xmm3, %xmm3
	vpsrldq	$8, %xmm3, %xmm0
	vpaddq	%xmm0, %xmm3, %xmm0
	cmpq	%rcx, %rsi
	vmovq	%xmm0, %rax
	je	.L33
	vzeroupper
.L20:
	movq	%rsi, %rdx
	subq	%rcx, %rdx
	leaq	-1(%rdx), %r8
	cmpq	$14, %r8
	jbe	.L24
	vmovdqu	(%rdi,%rcx), %xmm0
	movq	%rdx, %r8
	vpmovzxbw	%xmm0, %xmm2
	vpsrldq	$8, %xmm0, %xmm0
	vpmovzxwd	%xmm2, %xmm5
	vpmovzxbw	%xmm0, %xmm0
	vpmovzxwd	%xmm0, %xmm4
	vpmovzxdq	%xmm5, %xmm1
	vpsrldq	$8, %xmm0, %xmm0
	vpsrldq	$8, %xmm5, %xmm5
	vpmovzxwd	%xmm0, %xmm0
	vpmovzxdq	%xmm5, %xmm5
	vpaddq	%xmm5, %xmm1, %xmm1
	vpsrldq	$8, %xmm2, %xmm2
	vpsrldq	$8, %xmm0, %xmm5
	vpmovzxwd	%xmm2, %xmm2
	vpmovzxdq	%xmm5, %xmm5
	vpaddq	%xmm5, %xmm1, %xmm1
	vpmovzxdq	%xmm2, %xmm5
	vpsrldq	$8, %xmm2, %xmm2
	vpmovzxdq	%xmm2, %xmm2
	vpaddq	%xmm2, %xmm5, %xmm2
	vpmovzxdq	%xmm0, %xmm0
	vpaddq	%xmm0, %xmm2, %xmm0
	vpaddq	%xmm0, %xmm1, %xmm0
	vpmovzxdq	%xmm4, %xmm1
	vpsrldq	$8, %xmm4, %xmm4
	vpmovzxdq	%xmm4, %xmm4
	vpaddq	%xmm4, %xmm1, %xmm1
	vpaddq	%xmm3, %xmm1, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	andq	$-16, %r8
	vpsrldq	$8, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	addq	%r8, %rcx
	andl	$15, %edx
	vmovq	%xmm0, %rax
	je	.L18
.L24:
	movzbl	(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	1(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	1(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	2(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	2(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	3(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	3(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	4(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	4(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	5(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	5(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	6(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	6(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	7(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	7(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	8(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	8(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	9(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	9(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	10(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	10(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	11(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	11(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	12(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	12(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	13(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	13(%rdi,%rcx), %edx
	addq	%rdx, %rax
	leaq	14(%rcx), %rdx
	cmpq	%rsi, %rdx
	jnb	.L18
	movzbl	14(%rdi,%rcx), %edx
	addq	%rdx, %rax
	ret
	.p2align 4,,10
	.p2align 3
.L26:
	xorl	%eax, %eax
.L18:
	ret
.L27:
	vpxor	%xmm3, %xmm3, %xmm3
	xorl	%ecx, %ecx
	xorl	%eax, %eax
	jmp	.L20
.L33:
	vzeroupper
	ret
	.cfi_endproc
.LFE1:
	.size	k43_sum_ascii, .-k43_sum_ascii
	.p2align 4
	.globl	k50_count_masked
	.type	k50_count_masked, @function
k50_count_masked:
.LFB2:
	.cfi_startproc
	testq	%rsi, %rsi
	movq	%rdi, %r8
	movl	%ecx, %r9d
	movq	%rsi, %rdi
	je	.L42
	leaq	-1(%rsi), %rax
	cmpq	$30, %rax
	jbe	.L43
	movl	$16843009, %r10d
	vmovd	%ecx, %xmm5
	vmovd	%edx, %xmm6
	andq	$-32, %rsi
	vmovd	%r10d, %xmm4
	vpbroadcastb	%xmm6, %ymm6
	vpbroadcastb	%xmm5, %ymm5
	movq	%r8, %rax
	leaq	(%rsi,%r8), %rcx
	vpxor	%xmm3, %xmm3, %xmm3
	vpbroadcastd	%xmm4, %ymm4
	.p2align 4,,10
	.p2align 3
.L37:
	vpand	(%rax), %ymm6, %ymm0
	addq	$32, %rax
	vpcmpeqb	%ymm5, %ymm0, %ymm0
	cmpq	%rax, %rcx
	vpand	%ymm4, %ymm0, %ymm0
	vpmovzxbw	%xmm0, %ymm1
	vextracti128	$0x1, %ymm0, %xmm0
	vpmovzxwd	%xmm1, %ymm8
	vpmovzxbw	%xmm0, %ymm0
	vpmovzxwd	%xmm0, %ymm7
	vpmovzxdq	%xmm8, %ymm2
	vextracti128	$0x1, %ymm0, %xmm0
	vextracti128	$0x1, %ymm8, %xmm8
	vpmovzxwd	%xmm0, %ymm0
	vpmovzxdq	%xmm8, %ymm8
	vpaddq	%ymm8, %ymm2, %ymm2
	vpmovzxdq	%xmm0, %ymm8
	vpaddq	%ymm8, %ymm2, %ymm2
	vpmovzxdq	%xmm7, %ymm8
	vextracti128	$0x1, %ymm7, %xmm7
	vextracti128	$0x1, %ymm1, %xmm1
	vpmovzxdq	%xmm7, %ymm7
	vpmovzxwd	%xmm1, %ymm1
	vpaddq	%ymm7, %ymm8, %ymm7
	vpaddq	%ymm7, %ymm2, %ymm2
	vpmovzxdq	%xmm1, %ymm7
	vextracti128	$0x1, %ymm1, %xmm1
	vpmovzxdq	%xmm1, %ymm1
	vextracti128	$0x1, %ymm0, %xmm0
	vpaddq	%ymm1, %ymm7, %ymm1
	vpmovzxdq	%xmm0, %ymm0
	vpaddq	%ymm0, %ymm1, %ymm0
	vpaddq	%ymm0, %ymm2, %ymm0
	vpaddq	%ymm0, %ymm3, %ymm3
	jne	.L37
	vextracti128	$0x1, %ymm3, %xmm0
	vpaddq	%xmm3, %xmm0, %xmm3
	vpsrldq	$8, %xmm3, %xmm0
	vpaddq	%xmm0, %xmm3, %xmm0
	cmpq	%rsi, %rdi
	vmovq	%xmm0, %rax
	je	.L49
	vzeroupper
.L36:
	movq	%rdi, %rcx
	subq	%rsi, %rcx
	leaq	-1(%rcx), %r10
	cmpq	$14, %r10
	jbe	.L40
	vmovd	%edx, %xmm0
	vpbroadcastb	%xmm0, %xmm0
	vpand	(%r8,%rsi), %xmm0, %xmm0
	vmovd	%r9d, %xmm1
	vpbroadcastb	%xmm1, %xmm1
	vpcmpeqb	%xmm1, %xmm0, %xmm0
	movl	$16843009, %eax
	vmovd	%eax, %xmm1
	vpbroadcastd	%xmm1, %xmm1
	vpand	%xmm1, %xmm0, %xmm0
	vpmovzxbw	%xmm0, %xmm2
	vpmovzxwd	%xmm2, %xmm5
	vpsrldq	$8, %xmm0, %xmm0
	vpsrldq	$8, %xmm2, %xmm2
	vpmovzxbw	%xmm0, %xmm0
	vpmovzxwd	%xmm2, %xmm2
	vpmovzxwd	%xmm0, %xmm4
	vpmovzxdq	%xmm2, %xmm1
	vpsrldq	$8, %xmm0, %xmm0
	vpsrldq	$8, %xmm2, %xmm2
	vpmovzxwd	%xmm0, %xmm0
	vpmovzxdq	%xmm2, %xmm2
	vpaddq	%xmm2, %xmm1, %xmm1
	vpmovzxdq	%xmm0, %xmm2
	vpaddq	%xmm2, %xmm1, %xmm1
	vpmovzxdq	%xmm5, %xmm2
	vpsrldq	$8, %xmm5, %xmm5
	vpmovzxdq	%xmm5, %xmm5
	vpsrldq	$8, %xmm0, %xmm0
	vpaddq	%xmm5, %xmm2, %xmm2
	vpmovzxdq	%xmm0, %xmm0
	vpaddq	%xmm0, %xmm2, %xmm0
	vpaddq	%xmm0, %xmm1, %xmm0
	vpmovzxdq	%xmm4, %xmm1
	vpsrldq	$8, %xmm4, %xmm4
	vpmovzxdq	%xmm4, %xmm4
	vpaddq	%xmm4, %xmm1, %xmm1
	vpaddq	%xmm3, %xmm1, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	movq	%rcx, %r10
	andq	$-16, %r10
	vpsrldq	$8, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	addq	%r10, %rsi
	andl	$15, %ecx
	vmovq	%xmm0, %rax
	je	.L34
.L40:
	movzbl	(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	1(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	1(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	2(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	2(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	3(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	3(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	4(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	4(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	5(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	5(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	6(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	6(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	7(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	7(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	8(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	8(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	9(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	9(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	10(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	10(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	11(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	11(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	12(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	12(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	13(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	movzbl	13(%r8,%rsi), %ecx
	andl	%edx, %ecx
	cmpb	%r9b, %cl
	sete	%cl
	movzbl	%cl, %ecx
	addq	%rcx, %rax
	leaq	14(%rsi), %rcx
	cmpq	%rdi, %rcx
	jnb	.L34
	andb	14(%r8,%rsi), %dl
	cmpb	%r9b, %dl
	sete	%dl
	movzbl	%dl, %edx
	addq	%rdx, %rax
	ret
	.p2align 4,,10
	.p2align 3
.L42:
	xorl	%eax, %eax
.L34:
	ret
.L43:
	vpxor	%xmm3, %xmm3, %xmm3
	xorl	%esi, %esi
	xorl	%eax, %eax
	jmp	.L36
.L49:
	vzeroupper
	ret
	.cfi_endproc
.LFE2:
	.size	k50_count_masked, .-k50_count_masked
	.ident	"GCC: (Debian 14.2.0-19) 14.2.0"
	.section	.note.GNU-stack,"",@progbits
