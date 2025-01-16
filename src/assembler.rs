fn assemble(acasm: &str) -> Vec<u32> {
    let binary = Vec::new();

    for line in acasm.split('\n') {
        let split: Vec<&str> = line.split_whitespace().collect();
        let op = split[0];
        let vals = &split[1..];

        let word: u32 = 0;

        match op {
            "li" => ,
            "add" => (vals),
            _ => panic!("{op} not implemented!"),
        }
    }

    return binary;
}

fn parse_register(reg: &str) -> Result<usize, ParseRegisterError> {
    let mut chars = reg.chars();

    if Some('r') == chars.next() {
        chars
            .next()
            .and_then(|num| num.to_digit(10))
            .and_then(|num| num.try_into().ok())
            .ok_or(ParseRegisterError)
    } else {
        Err(ParseRegisterError)
    }
}