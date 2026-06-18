; fn __mul_to_32bit(a: word, b: word, out_l: &mut word, out_h: &mut word)
; This cann't be implemented in pure SHIT, cuz compiler store only
;   lower part of the result. So, here is need some assembly

: __mul_to_32bit
enter immtoreg   al al
# 0
push  regtoreg   al al
push  regtoreg   al ah
push  regtoreg   al bl

; a -> AL
mov   memratoreg al bp
# 5
; b -> BL
mov   memratoreg bl bp
# 4

; a * b -> (AH:AL) little-endian
mul   regtoreg   al bl

; AH -> mem[out_h]
mov   memratoreg bl bp
# 2
mov   regtomemr  bl ah

; AL -> mem[out_l]
mov   memratoreg bl bp
# 3
mov   regtomemr  bl al

pop   regtoreg   bl al
pop   regtoreg   ah al
pop   regtoreg   al al
leave regtoreg   al al
ret   immtoreg   al al
# 4



; fn __sub_32bit(
;   a_l: word, a_h: word,
;   b_l: word, b_h: word,
;   out_l: &mut word, out_h: &mut word
; )
; This cann't be implemented in pure SHIT,
;   cuz compiler doesn't support subc command

: __sub_32bit
enter immtoreg   al al
# 0
push  regtoreg   al al
push  regtoreg   al ah
push  regtoreg   al bl

; a_l -> AL
mov   memratoreg al bp
# 7
sub   memratoreg al bp
# 5

; a_h -> AH
mov   memratoreg ah bp
# 6
subc   memratoreg ah bp
# 4

; AH -> mem[out_h]
mov   memratoreg bl bp
# 2
mov   regtomemr  bl ah

; AL -> mem[out_l]
mov   memratoreg bl bp
# 3
mov   regtomemr  bl al

pop   regtoreg   bl al
pop   regtoreg   ah al
pop   regtoreg   al al
leave regtoreg   al al
ret   immtoreg   al al
# 4



; fn __add_32bit(
;   a_l: word, a_h: word,
;   b_l: word, b_h: word,
;   out_l: &mut word, out_h: &mut word
; )
; This cann't be implemented in pure SHIT,
;   cuz compiler doesn't support addc command

: __add_32bit
enter immtoreg   al al
# 0
push  regtoreg   al al
push  regtoreg   al ah
push  regtoreg   al bl

; a_l -> AL
mov   memratoreg al bp
# 7
add   memratoreg al bp
# 5

; a_h -> AH
mov   memratoreg ah bp
# 6
addc   memratoreg ah bp
# 4

; AH -> mem[out_h]
mov   memratoreg bl bp
# 2
mov   regtomemr  bl ah

; AL -> mem[out_l]
mov   memratoreg bl bp
# 3
mov   regtomemr  bl al

pop   regtoreg   bl al
pop   regtoreg   ah al
pop   regtoreg   al al
leave regtoreg   al al
ret   immtoreg   al al
# 4



; fn __is_less_32bit(
;   a_l: word, a_h: word,
;   b_l: word, b_h: word,
;   out_l: &mut word
; )
; This can be implemented in pure SHIT,
;   but it's more optimized

: __is_less_32bit
enter immtoreg   al al
# 0
push  regtoreg   al al
push  regtoreg   al ah
push  regtoreg   al bl

; if (a_h < b_h) return 1;
mov   memratoreg al bp
# 5
cmp   memratoreg al bp
# 3
jl   immtoreg   al al
@ __is_less_32bit.set_return.1

; if (a_h > b_h) return 0;
jg    immtoreg   al al
@ __is_less_32bit.set_return.0

; if (a_l < b_l) return 1;
mov   memratoreg al bp
# 6
cmp   memratoreg al bp
# 4
jcs    immtoreg   al al
@ __is_less_32bit.set_return.1

; else return 0;
jmp    immtoreg   al al
@ __is_less_32bit.set_return.0

: __is_less_32bit.set_return.1
mov   immtoreg al al
# 1
jmp   immtoreg al al
@ __is_less_32bit.return

: __is_less_32bit.set_return.0
mov   immtoreg al al
# 0

: __is_less_32bit.return
mov   memratoreg bl bp
# 2
mov   regtomemr  bl al

pop   regtoreg   bl al
pop   regtoreg   ah al
pop   regtoreg   al al
leave regtoreg   al al
ret   immtoreg   al al
# 5
