: hello_str
# 104
# 101
# 108
# 108
# 111
# 0


: print_hello_str
push regtoreg al al
push regtoreg al bl
enter immtoreg al al
# 0
mov immtoreg al al
@ hello_str

: loop
mov memrtoreg bl al
cmp immtoreg bl al
# 0
je immtoreg al al
@ print_hello_str_out
mov regtomema al bl
# 129
inc regtoreg al al
jmp immtoreg al al
@ loop

: print_hello_str_out
leave regtoreg al al
pop regtoreg bl al
pop regtoreg al al
ret regtoreg al al
