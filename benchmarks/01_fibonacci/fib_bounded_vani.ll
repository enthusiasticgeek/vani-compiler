; ModuleID = 'intent-ssa'
declare i32 @printf(i8*, ...)
declare i32 @vsnprintf(i8*, i64, i8*, i8*)
declare i32 @_write(i32, i8*, i32)
declare void @llvm.va_start(i8*)
declare void @llvm.va_end(i8*)
define i32 @snprintf(i8* %_snp_buf, i64 %_snp_sz, i8* %_snp_fmt, ...) {
  %_snp_ap = alloca i8*, align 8
  %_snp_ap_i8 = bitcast i8** %_snp_ap to i8*
  call void @llvm.va_start(i8* %_snp_ap_i8)
  %_snp_ap_v = load i8*, i8** %_snp_ap
  %_snp_r = call i32 @vsnprintf(i8* %_snp_buf, i64 %_snp_sz, i8* %_snp_fmt, i8* %_snp_ap_v)
  call void @llvm.va_end(i8* %_snp_ap_i8)
  ret i32 %_snp_r
}
define i32 @dprintf(i32 %_dpr_fd, i8* %_dpr_fmt, ...) {
  %_dpr_buf = alloca [256 x i8], align 1
  %_dpr_bufp = getelementptr [256 x i8], [256 x i8]* %_dpr_buf, i64 0, i64 0
  %_dpr_ap = alloca i8*, align 8
  %_dpr_ap_i8 = bitcast i8** %_dpr_ap to i8*
  call void @llvm.va_start(i8* %_dpr_ap_i8)
  %_dpr_ap_v = load i8*, i8** %_dpr_ap
  %_dpr_n = call i32 @vsnprintf(i8* %_dpr_bufp, i64 256, i8* %_dpr_fmt, i8* %_dpr_ap_v)
  call void @llvm.va_end(i8* %_dpr_ap_i8)
  %_dpr_r = call i32 @_write(i32 %_dpr_fd, i8* %_dpr_bufp, i32 %_dpr_n)
  ret i32 %_dpr_r
}
declare i32 @putchar(i32)
declare void @abort() noreturn
declare noalias i8* @malloc(i64)
declare noalias i8* @realloc(i8*, i64)
declare void @free(i8*)
declare void @qsort(i8*, i64, i64, i32 (i8*, i8*)*)
declare i8* @memcpy(i8*, i8*, i64)
declare i8* @memmove(i8*, i8*, i64)
declare i32 @strcmp(i8*, i8*)
declare i64 @strlen(i8*)
declare void @llvm.assume(i1)
define internal void @__intent_bounds_check(i64 %idx, i64 %len) alwaysinline {
entry:
  %ok = icmp ult i64 %idx, %len
  br i1 %ok, label %cont, label %oob, !prof !{!"branch_weights", i32 1048576, i32 1}
oob:
  call void @abort()
  unreachable
cont:
  call void @llvm.assume(i1 %ok)
  ret void
}
@.empty_str_clone = private constant [1 x i8] c"\00"
@.fmt.c = private constant [3 x i8] c"%c\00"
declare i8* @CreateThread(i8*, i64, i8* (i8*)*, i8*, i32, i32*)
declare i32 @WaitForSingleObject(i8*, i32)
declare i32 @CloseHandle(i8*)
declare void @Sleep(i32)
declare i32 @WaitOnAddress(i8*, i8*, i64, i32)
declare void @WakeByAddressSingle(i8*)
%intent_task_handle = type { i64, i8* }
%intent_mutex_i64 = type { i64, i32 }
%intent_guard_i64 = type { %intent_mutex_i64* }

define i8* @intent_str_concat(i8* %l, i32 %lo, i8* %r, i32 %ro) {
  %ln = call i64 @strlen(i8* %l)
  %rn = call i64 @strlen(i8* %r)
  %sum = add i64 %ln, %rn
  %total = add i64 %sum, 1
  %buf = call i8* @malloc(i64 %total)
  %_cl = call i8* @memcpy(i8* %buf, i8* %l, i64 %ln)
  %tail = getelementptr i8, i8* %buf, i64 %ln
  %_cr = call i8* @memcpy(i8* %tail, i8* %r, i64 %rn)
  %nul = getelementptr i8, i8* %buf, i64 %sum
  store i8 0, i8* %nul
  %lo_b = icmp ne i32 %lo, 0
  br i1 %lo_b, label %free_l, label %check_r
free_l:
  call void @free(i8* %l)
  br label %check_r
check_r:
  %ro_b = icmp ne i32 %ro, 0
  br i1 %ro_b, label %free_r, label %done
free_r:
  call void @free(i8* %r)
  br label %done
done:
  ret i8* %buf
}

@.str.0 = private unnamed_addr constant [5 x i8] c"%lld\00"

define i64 @fn_fib(i64 %v_0) {
bb0:
  %v_1 = icmp sle i64 %v_0, 1
  br i1 %v_1, label %bb1, label %bb2
bb1:
  ret i64 %v_0
bb2:
  br label %bb3
bb3:
  %v_2 = sub i64 %v_0, 1
  %v_3 = call i64 @fn_fib(i64 %v_2)
  %v_4 = sub i64 %v_0, 2
  %v_5 = call i64 @fn_fib(i64 %v_4)
  %v_6 = add i64 %v_3, %v_5
  ret i64 %v_6
}

define i64 @fn_main() {
bb0:
  %v_0 = call i64 @fn_fib(i64 42)
  %v_1.fmt = getelementptr [5 x i8], [5 x i8]* @.str.0, i64 0, i64 0
  call i32 (i8*, ...) @printf(i8* %v_1.fmt, i64 %v_0)
  %v_2.putc = trunc i64 10 to i32
  %v_2.putcfmt = getelementptr [3 x i8], [3 x i8]* @.fmt.c, i64 0, i64 0
  call i32 (i8*, ...) @printf(i8* %v_2.putcfmt, i32 %v_2.putc)
  %v_3 = add i64 0, 0
  ret i64 %v_3
}

define i32 @main() {
entry:
  %r = call i64 @fn_main()
  %r32 = trunc i64 %r to i32
  ret i32 %r32
}
