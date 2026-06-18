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
| AL       | `0x0`  | -           |
| AH       | `0x1`  | -           |
| BL       | `0x2`  | -           |
| BH       | `0x3`  | -           |
| CL       | `0x4`  | -           |
| CH       | `0x5`  | -           |

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
| MemR-to-Reg  | `0x2`  |         0         |        3         | Rd ← [Rs]       |
| MemA-to-Reg  | `0x3`  |         1         |        4         | Rd ← [addr]     |
| MemRA-to-Reg | `0x4`  |         1         |        4         | Rd ← [Rs+off]   |
| Reg-to-MemR  | `0x5`  |         0         |        3         | [Rd] ← Rd       |
| Imm-to-MemR  | `0x6`  |         1         |        4         | [Rd] ← #imm     |
| Reg-to-MemA  | `0x7`  |         1         |        4         | [addr] ← Rs     |
| Imm-to-MemA  | `0x8`  |         2         |        5         | [addr] ← #imm   |
| Reg-to-MemRA | `0x9`  |         1         |        4         | [Rd+off] ← Rd   |
| Imm-to-MemRA | `0xA`  |         2         |        5         | [Rd+off] ← #imm |

# Instructions

> [!note] Instruction base dealy
> Each instruction has base delay = 2 for the InstructionFetch step
> The "Delay" column indicates only the ADDITIONAL delay to this base

## System

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description                 |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | --------------------------- |
| NOP         |  0x00  | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | Not operation. Just nothing |
| HLT         |  0x01  | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | Halt machine                |

## Data movement

| Instruction   | Opcode | Mode |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description |
| ------------- | :----: | :--: | :-------------: | :------------: | :-: | :-: | :-: | :-: | ----------- |
| MOV dist, src |  0x02  | ANY  | 1 + mode length | 1 + mode delay |  -  |  -  |  -  |  -  | dist ← src  |

## Math

| Instruction    | Opcode |    Mode    |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description                          |
| -------------- | :----: | :--------: | :-------------: | :------------: | :-: | :-: | :-: | :-: | ------------------------------------ |
| ADD dist, src  |  0x03  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist + src                    |
| ADDC dist, src |  0x04  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist + src + CF               |
| SUB dist, src  |  0x05  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist - src                    |
| SUBC dist, src |  0x06  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist - src - CF               |
| MUL Rd, src    |  0x07  | ANY-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | Rd+1:Rd ← Rd × src                   |
| DIV Rd, src    |  0x08  | ANY-to-Reg | 1 + mode length | 1 + mode delay | \*  | \*  |  -  |  -  | Rd ← Rd+1:Rd ÷ src, Rd+1 ← remainder |
| INC Rd, Rs     |  0x09  | Reg-to-Reg |        1        |       1        | \*  | \*  |  -  | \*  | Rd ← Rs + 1                          |
| DEC Rd, Rs     |  0x0A  | Reg-to-Reg |        1        |       1        | \*  | \*  |  -  | \*  | Rd ← Rs - 1                          |
| NEG Rd, Rs     |  0x0B  | Reg-to-Reg |        1        |       1        | \*  | \*  | \*  | \*  | Rd ← -Rs                             |

## Logic

| Instruction    | Opcode |    Mode    |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description          |
| -------------- | :----: | :--------: | :-------------: | :------------: | :-: | :-: | :-: | :-: | -------------------- |
| AND dist, src  |  0x0C  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist & src    |
| OR dist, src   |  0x0D  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist \| src   |
| XOR dist, src  |  0x0E  |    ANY     | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist ← dist ^ src    |
| NOT dist, src  |  0x0F  | Reg-to-Reg |        1        |       1        | \*  | \*  | \*  | \*  | dist ← !src          |
| SHL dist, #off |  0x10  | Imm-to-Reg |        2        |       2        | \*  | \*  | \*  | \*  | dist ← dist \<< #off |
| SHR dist, #off |  0x11  | Imm-to-Reg |        2        |       2        | \*  | \*  | \*  | \*  | dist ← dist \<< #off |

## Comparision

| Instruction    | Opcode | Mode |     Length      |     Delay      | ZF  | NF  | CF  | OF  | Description                |
| -------------- | :----: | :--: | :-------------: | :------------: | :-: | :-: | :-: | :-: | -------------------------- |
| CMP dist, src  |  0x14  | ANY  | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist - src, just set flags |
| TEST dist, src |  0x15  | ANY  | 1 + mode length | 1 + mode delay | \*  | \*  | \*  | \*  | dist & src, just set flags |

## Control flow

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description                      |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | -------------------------------- |
| JMP #addr   |  0x16  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | IP ← #addr                       |
| JE #addr    |  0x17  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=1: IP ← #addr              |
| JNE #addr   |  0x18  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=0: IP ← #addr              |
| JNS #addr   |  0x19  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF=1: IP ← #addr              |
| JNC #addr   |  0x1A  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF=1: IP ← #addr              |
| JCS #addr   |  0x1B  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if CF=1: IP ← #addr              |
| JCC #addr   |  0x1C  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if CF=0: IP ← #addr              |
| JOS #addr   |  0x1D  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if OF=1: IP ← #addr              |
| JOC #addr   |  0x1E  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if OF=0: IP ← #addr              |
| JL #addr    |  0x1F  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF != OF: IP ← #addr          |
| JLE #addr   |  0x20  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=1 or NF!=OF: IP ← #addr    |
| JG #addr    |  0x21  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if ZF=0 and NF=OF: IP ← #addr    |
| JGE #addr   |  0x22  | Imm-to-Reg |   2    |   2   |  -  |  -  |  -  |  -  | if NF=OF: IP ← #addr             |
| CALL #addr  |  0x23  | Imm-to-Reg |   2    |   4   |  -  |  -  |  -  |  -  | SP ← SP-1; [SP] ← IP; IP ← #addr |
| RET         |  0x24  | Reg-to-Reg |   1    |   4   |  -  |  -  |  -  |  -  | IP ← [SP]; SP ← SP+1             |
| RET #n      |  0x24  | Imm-to-Reg |   1    |   5   |  -  |  -  |  -  |  -  | IP ← [SP]; SP ← SP+1; SP ← SP+#n |

## Stack

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description                     |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | ------------------------------- |
| PUSH Rd     |  0x25  | Reg-to-Reg |   1    |   3   |  -  |  -  |  -  |  -  | SP ← SP-1; [SP] ← Rd            |
| POP Rd      |  0x26  | Reg-to-Reg |   1    |   4   |  -  |  -  |  -  |  -  | Rd ← [SP]; SP ← SP+1            |
| ENTER #n    |  0x27  | Imm-to-Reg |   2    |   6   |  -  |  -  |  -  |  -  | PUSH BP; MOV BP, SP; SUB SP, #n |
| LEAVE       |  0x28  | Reg-to-Reg |   1    |   5   |  -  |  -  |  -  |  -  | MOV SP, BP; POP BP              |

## Interruptions

| Instruction | Opcode |    Mode    | Length | Delay | ZF  | NF  | CF  | OF  | Description       |
| ----------- | :----: | :--------: | :----: | :---: | :-: | :-: | :-: | :-: | ----------------- |
| IRET        |  0x2A  | Reg-to-Reg |   1    |   6   |  r  |  r  |  r  |  r  | POP IP; POP FLAGS |
| STI         |  0x2B  | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | IF ← 1            |
| CLI         |  0x2C  | Reg-to-Reg |   1    |   1   |  -  |  -  |  -  |  -  | IF ← 0            |
