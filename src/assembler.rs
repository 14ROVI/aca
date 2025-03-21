use std::{collections::HashMap, fs};

use bytes::{BufMut, BytesMut};
use regex::Regex;

use crate::instructions::Word;

pub fn assemble_file(filename: &str) -> (BytesMut, Vec<Word>) {
    let mut file_content = fs::read_to_string(filename).unwrap();
    file_content = preprocessor(&file_content);
    assemble(&file_content)
}

fn preprocessor(acasm: &str) -> String {
    acasm
        .lines()
        .map(|l| l.split("//").collect::<Vec<_>>()[0])
        .collect::<Vec<_>>()
        .join("\n")
}

fn assemble(acasm: &str) -> (BytesMut, Vec<Word>) {
    let re = Regex::new(r"(?:(?:\.memory)([\s\S]*))?(?:\.instructions)([\s\S]*)").unwrap();

    let captures = re.captures(acasm).unwrap();
    let (memory, label_locations) = match captures.get(1) {
        Some(mem_acasm) => create_memory(mem_acasm.as_str()),
        None => (BytesMut::with_capacity(4049), HashMap::new()),
    };

    let inst_acasm = captures.get(2).unwrap().as_str();
    let instructions = create_instructions(inst_acasm, label_locations);

    return (memory, instructions);
}

fn create_memory(acasm: &str) -> (BytesMut, HashMap<String, usize>) {
    let mut memory = BytesMut::with_capacity(4096);
    let mut label_locations = HashMap::new();

    let section_re = Regex::new(r"(\w+):\s(.\w+)\s(.*)").unwrap();

    let mut current_addr = 0;

    for sections in section_re.captures_iter(acasm) {
        let directive = sections.get(2).unwrap().as_str();
        let arguments = sections.get(3).unwrap();

        let start_addr = current_addr;

        current_addr += match directive {
            ".int" => int_directive,
            ".float" => float_directive,
            ".space" => space_directive,
            ".file" => file_directive,
            _ => todo!(),
        }(&mut memory, arguments.as_str());

        if let Some(label) = sections.get(1) {
            label_locations.insert(label.as_str().to_string(), start_addr);
        }
    }

    (memory, label_locations)
}

fn int_directive(memory: &mut BytesMut, arguments: &str) -> usize {
    let count = arguments
        .split(",")
        .map(|s| s.trim().parse().unwrap())
        .inspect(|i| memory.put_i32(*i))
        .count();

    return (count * 4) as usize;
}

fn float_directive(memory: &mut BytesMut, arguments: &str) -> usize {
    let count = arguments
        .split(",")
        .map(|s| s.trim().parse().unwrap())
        .inspect(|i| memory.put_f32(*i))
        .count();

    return (count * 4) as usize;
}

fn space_directive(memory: &mut BytesMut, arguments: &str) -> usize {
    let n: usize = arguments.trim().parse().unwrap();
    memory.put_bytes(0, n);
    return n as usize;
}

fn file_directive(memory: &mut BytesMut, arguments: &str) -> usize {
    let file_path = arguments.trim();
    let content = fs::read(file_path).unwrap();
    memory.put(content.as_slice());
    return content.len() as usize;
}

fn create_instructions(acasm: &str, mut labels: HashMap<String, usize>) -> Vec<Word> {
    let mut instructions = Vec::new();

    let lines: Vec<&str> = acasm
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut pc = 0;
    for line in &lines {
        if line.ends_with(":") {
            let label = line[0..line.len() - 1].to_string();
            labels.insert(label, pc);
        } else {
            pc += 1;
        }
    }

    for line in lines.iter().filter(|l| !l.ends_with(':')) {
        let split: Vec<&str> = line.split_whitespace().collect();
        let op = split[0];

        // replace labels with values
        let args = split[1..split.len()]
            .iter()
            .map(|arg| {
                if labels.contains_key(*arg) {
                    if arg.starts_with('$') {
                        return format!("${}", labels.get(*arg).unwrap());
                    } else {
                        return match op {
                            "be" | "bne" | "bg" | "bge" | "bl" | "ble" => {
                                (*labels.get(*arg).unwrap() as i32) - (instructions.len() as i32)
                            }
                            _ => *labels.get(*arg).unwrap() as i32,
                        }
                        .to_string();
                    }
                } else {
                    return arg.to_string();
                }
            })
            .collect::<Vec<String>>();
        // dbg!(&args);
        let word = match op {
            "li" => Word::load_immediate(p_reg(&args[0]), p_i32(&args[1])),
            "lw" => Word::load_memory(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "lhw" => Word::load_half_word(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "lc" => Word::load_char(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "sw" => Word::store_memory(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "sc" => Word::store_char(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "add" => Word::add(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "addi" => Word::add_immediate(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "sub" => Word::subtract(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "subi" => Word::subtract_immediate(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "mult" => Word::multiply(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "multno" => Word::multiply_no_overflow(p_reg(&args[0]), p_reg(&args[1])),
            "div" => Word::divide(p_reg(&args[0]), p_reg(&args[1])),
            "cmp" => Word::compare(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "and" => Word::bit_and(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "andi" => Word::bit_and_immediate(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "or" => Word::bit_or(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "ori" => Word::bit_or_immediate(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "neg" => Word::neg(p_reg(&args[0]), p_reg(&args[1])),
            "lsft" => Word::left_shift(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "rsft" => Word::right_shift(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "be" => Word::branch_equal(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "bne" => Word::branch_not_equal(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "bg" => Word::branch_greater(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "bge" => Word::branch_greater_equal(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "bl" => Word::branch_less(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "ble" => Word::branch_less_equal(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "j" => Word::jump_immediate(p_i32(&args[0])),
            "jr" => Word::jump_reg(p_reg(&args[0])),
            "jal" => Word::jump_and_link(p_reg(&args[0]), p_i32(&args[1])),
            "fli" => Word::fload_immediate(p_reg(&args[0]), p_f32(&args[1])),
            "fadd" => Word::fadd(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "faddi" => Word::fadd_immediate(p_reg(&args[0]), p_reg(&args[1]), p_f32(&args[2])),
            "fsub" => Word::fsubtract(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "fsubi" => Word::fsubtract_immediate(p_reg(&args[0]), p_reg(&args[1]), p_f32(&args[2])),
            "fmult" => Word::fmultiply(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "fdiv" => Word::fdivide(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "fcmp" => Word::fcompare(p_reg(&args[0]), p_reg(&args[1]), p_reg(&args[2])),
            "lv" => Word::v_load_memory(p_v_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "sv" => Word::v_store_memory(p_v_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "vadd" => Word::v_add(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vsub" => Word::v_subtract(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vmult" => Word::v_multiply(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vdiv" => Word::v_divide(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vlsft" => Word::v_left_shift(p_v_reg(&args[0]), p_v_reg(&args[1]), p_i32(&args[2])),
            "vrsft" => Word::v_right_shift(p_v_reg(&args[0]), p_v_reg(&args[1]), p_i32(&args[2])),
            "vfadd" => Word::v_fadd(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vfsub" => Word::v_fsubtract(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vfmult" => Word::v_fmultiply(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vfdiv" => Word::v_fdivide(p_v_reg(&args[0]), p_v_reg(&args[1]), p_v_reg(&args[2])),
            "vsum" => Word::v_sum(p_reg(&args[0]), p_reg(&args[1]), p_v_reg(&args[2])),
            "mfhi" => Word::move_from_high(p_reg(&args[0])),
            "mflo" => Word::move_from_low(p_reg(&args[0])),
            "mv" => Word::add_immediate(p_reg(&args[0]), p_reg(&args[1]), 0),
            "exit" => Word::exit(p_reg(&args[0])),
            "reserve" => Word::reserve_memory(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            "save" => Word::save(p_reg(&args[0]), p_reg(&args[1]), p_i32(&args[2])),
            other => todo!("instruction '{}' not implemented!", other),
        };

        instructions.push(word);
    }

    return instructions;
}

fn p_reg(reg: &str) -> u32 {
    let mut chars = reg.chars();

    if Some('$') == chars.next() {
        chars.as_str().parse().unwrap()
    } else {
        panic!("{} is not a register", reg);
    }
}

fn p_v_reg(reg: &str) -> u32 {
    let mut chars = reg.chars();

    if Some('$') == chars.next() && Some('v') == chars.next() {
        chars.as_str().parse().unwrap()
    } else {
        panic!("{} is not a register", reg);
    }
}

fn p_i32(immediate: &str) -> i32 {
    immediate.parse().unwrap()
}

fn p_f32(immediate: &str) -> f32 {
    immediate.parse().unwrap()
}
