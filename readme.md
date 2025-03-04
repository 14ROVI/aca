# This repo has a couple parts

- An assembler
- A cpu simulator

The program takes `input.acasm` and assembles it into a list of object-like instructions which is then run by the cpu simulator.

## ACASM

Acasm files support these these directives:

- `.ascii "text you want stored"`
- `.int i1, i2, ..., in`
- `.space n`
- `.memory` followed by memory initialisation
- `.instructions` followed by program instructions

And these instructions:

- `li out immediate` loads immediate into register.
- `lw out pointer offset` load word into register.
- `sw reg pointer offset` store word into memory.
- `add out left right` add values in left and right and store in out.
- `addi out left immediate` add value in left to immediate and store in out.
- `sub out left right` subtract value stored in left and right and store in out.
- `subi` subtract_immediate p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `mult` multiply p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])
- `multno` multiply_no_overflow p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])
- `div` divide p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])
- `cmp` compare p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])
- `and` bit_and p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])
- `andi` bit_and_immediate p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `or` bit_or p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])
- `ori` bit_or_immediate p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `lsft` left_shift p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `rsft` right_shift p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `be` branch_equal p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `bne` branch_not_equal p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `bg` branch_greater p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `bge` branch_greater_equal p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `bl` branch_less p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `ble` branch_less_equal p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])
- `j` jump_immediate p_i32(&args[0])
- `jr` jump_reg p_reg(&args[0])
- `jl` jump_and_link p_reg(&args[0]), p_i32(&args[1])