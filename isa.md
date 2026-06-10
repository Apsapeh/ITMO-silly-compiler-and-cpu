# Overview

| Parameter                 | Value                |
| ------------------------- | -------------------- |
| Word size                 | 16 bit               |
| Unit of addres resolution | Word                 |
| Address space             | 65536 word (128 KiB) |

# Registers

## General purpose registers

| Register | Opcode | Description |
| -------- | ------ | ----------- |
| AL       | `0x0`  |             |
| AH       | `0x1`  |             |
| BL       | `0x2`  |             |
| BH       | `0x3`  |             |
| CL       | `0x4`  |             |
| CH       | `0x5`  |             |

## Special Registers

| Register | Opcode | Description                |
| -------- | ------ | -------------------------- |
| SP       | `0x6`  | Stack pointer              |
| BP       | `0x7`  | Base pointer (stack frame) |

## Hidden

IP - Instruction pointer

# Addressing modes

| Mode         | Opcode | Additional Length | Additional Delay | Description     |
| ------------ | :----: | :---------------: | :--------------: | --------------- |
| Reg-to-Reg   | `0x0`  |         0         |        0         | Rd ← Rs         |
| Imm-to-Reg   | `0x1`  |         1         |        1         | Rd ← #imm       |
| MemR-to-Reg  | `0x2`  |         0         |        0         | Rd ← [Rs]       |
| MemA-to-Reg  | `0x3`  |         1         |        1         | Rd ← [addr]     |
| MemRA-to-Reg | `0x4`  |         1         |        1         | Rd ← [Rs+off]   |
| Reg-to-MemR  | `0x5`  |         0         |        0         | [Rd] ← Rd       |
| Imm-to-MemR  | `0x6`  |         1         |        1         | [Rd] ← #imm     |
| Reg-to-MemA  | `0x7`  |         1         |        1         | [addr] ← Rs     |
| Imm-to-MemA  | `0x8`  |         2         |        2         | [addr] ← #imm   |
| Reg-to-MemRA | `0x9`  |         1         |        1         | [Rd+off] ← Rd   |
| Imm-to-MemRA | `0xA`  |         2         |        2         | [Rd+off] ← #imm |

# Instructions

## System

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description                 |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | --------------------------- |
| NOP         |        | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | Not operation. Just nothing |
| HLT         |        | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | Halt machine                |

## Data movement

| Instruction   | Opcode | Mode |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description |
| ------------- | :----: | :--: | :-------------: | :------------: | :-: | :-: | :-: | :-: | ----------- |
| MOV dist, src |        | ANY  | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← src  |

## Math

| Instruction    | Opcode |    Mode    |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description                          |
| -------------- | :----: | :--------: | :-------------: | :------------: | :-: | :-: | :-: | :-: | ------------------------------------ |
| ADD dist, src  |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist + src                    |
| ADDC dist, src |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist + src + CF               |
| SUB dist, src  |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist - src                    |
| SUBC dist, src |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist - src - CF               |
| MUL Rd, src    |        | ANY-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | Rd+1:Rd ← Rd × src                   |
| DIV Rd, src    |        | ANY-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | Rd ← Rd+1:Rd ÷ src, Rd+1 ← remainder |
| INC Rd, Rs     |        | Reg-to-Reg |        1        |       1        | \*  | \*  | \*  | \*  | Rd ← Rs + 1                          |
| DEC Rd, Rs     |        | Reg-to-Reg |        1        |       1        | \*  | \*  | \*  | \*  | Rd ← Rs - 1                          |
| NEG Rd, Rs     |        | Reg-to-Reg |        1        |       1        | \*  | \*  | \*  | \*  | Rd ← -Rs                             |

## Logic

| Instruction    | Opcode |    Mode    |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description |
| -------------- | :----: | :--------: | :-------------: | :------------: | :-: | :-: | :-: | :-: | ----------- |
| AND dist, src  |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  |             |
| OR dist, src   |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  |             |
| XOR dist, src  |        |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  |             |
| NOT dist, src  |        | Reg-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  |             |
| SHL dist, #off |        | Imm-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  |             |
| SHR dist, #off |        | Imm-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  |             |

## Comparision

| Instruction    | Opcode | Mode |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description                |
| -------------- | :----: | :--: | :-------------: | :------------: | :-: | :-: | :-: | :-: | -------------------------- |
| CMP dist, src  |        | ANY  | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist - src, just set flags |
| TEST dist, src |        | ANY  | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist & src, just set flags |

## Control flow

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description                      |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | -------------------------------- |
| JMP #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | IP ← #addr                       |
| JE #addr    |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=1: IP ← #addr              |
| JNE #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=0: IP ← #addr              |
| JNS #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF=1: IP ← #addr              |
| JNC #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF=1: IP ← #addr              |
| JCS #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if CF=1: IP ← #addr              |
| JCC #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if CF=0: IP ← #addr              |
| JOS #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if OF=1: IP ← #addr              |
| JOC #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if OF=0: IP ← #addr              |
| JL #addr    |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF != OF: IP ← #addr          |
| JLE #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=1 or NF!=OF: IP ← #addr    |
| JG #addr    |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=0 and NF=OF: IP ← #addr    |
| JGE #addr   |        | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF=OF: IP ← #addr             |
| CALL #addr  |        | Imm-to-Reg |   2    |   3   |  -  |  -  |  -  |  -  | SP ← SP-1; [SP] ← IP; IP ← #addr |
| RET         |        | Reg-to-Reg |   1    |   2   |  -  |  -  |  -  |  -  | IP ← [SP]; SP ← SP+1             |

## Stack

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description                     |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | ------------------------------- |
| PUSH Rd     |        | Reg-to-Reg |   1    |   2   |  -  |  -  |  -  |  -  | SP ← SP-1; [SP] ← Rd            |
| POP Rd      |        | Reg-to-Reg |   1    |   2   |  -  |  -  |  -  |  -  | Rd ← [SP]; SP ← SP+1            |
| ENTER #n    |        | Imm-to-Reg |   2    |   4   |  -  |  -  |  -  |  -  | PUSH BP; MOV BP, SP; SUB SP, #n |
| LEAVE       |        | Reg-to-Reg |   1    |   3   |  -  |  -  |  -  |  -  | MOV SP, BP; POP BP              |

## Interruptions

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description       |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | ----------------- |
| IRET        |        | Reg-to-Reg |   1    |   4   |  -  |  -  |  -  |  -  | POP IP; POP FLAGS |
| STI         |        | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | IF ← 1            |
| CLI         |        | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | IF ← 0            |

### AND

### OR

### XOR

### NOT

### JMP

### JE

### JNE

### CALL

### RET

### PUSH

### POP

### STI

### CLI

### IRET

### NOP

### HLT

### CLR
