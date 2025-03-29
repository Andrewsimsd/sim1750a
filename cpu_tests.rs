fn set_registers(r2_val: i16, base_regs: [u16; 4]) {
    unsafe {
        simreg.r = [0; 16];
        simreg.r[2] = r2_val;
        for (i, val) in base_regs.iter().enumerate() {
            simreg.r[12 + i] = *val as i16;
        }
        simreg.ic = 0;
    }
}

fn read_memory(addr: u16) -> i16 {
    let page = addr >> 8;
    let offset = addr & 0xFF;
    unsafe {
        memoryBank.memPage[page as usize].memData[offset as usize] as i16
    }
}

#[test]
fn test_ex_stb_all_base_selectors_and_offsets() {
    let base_regs = [0x1000, 0x2000, 0x3000, 0x4000];
    let offsets = [0x00, 0x01, 0x7F, 0xFF];
    let r2_val: i16 = 0x1234;

    for base_sel in 0..=3 {
        for &offset in &offsets {
            let op = make_opcode(base_sel, offset);
            unsafe {
                opcode = op;
                set_registers(r2_val, base_regs);
                let addr = expected_addr(op);

                let result = ex_stb();
                assert_eq!(result, NC_STB, "Base {} offset {:#04X}: Incorrect return code", base_sel, offset);
                assert_eq!(simreg.ic, 1, "Base {} offset {:#04X}: IC not incremented", base_sel, offset);
                let stored = read_memory(addr);
                assert_eq!(stored, r2_val, "Base {} offset {:#04X}: Memory write incorrect", base_sel, offset);
            }
        }
    }
}

#[test]
fn test_ex_stb_signed_values_edge_cases() {
    let base_regs = [0x0800; 4];
    let op = make_opcode(2, 0x04);
    let values = [
        0,
        1,
        -1,
        i16::MAX,
        i16::MIN,
        12345,
        -12345,
    ];

    unsafe {
        opcode = op;
        for (i, val) in values.iter().enumerate() {
            set_registers(*val, base_regs);
            let addr = expected_addr(op);

            let result = ex_stb();
            assert_eq!(result, NC_STB, "Test {}: Incorrect return code", i);
            assert_eq!(simreg.ic, 1, "Test {}: IC not incremented", i);
            let stored = read_memory(addr);
            assert_eq!(stored, *val, "Test {}: Incorrect value written to memory", i);
        }
    }
}

#[test]
fn test_ex_stb_address_wraparound() {
    let base = 0xFFFC;
    let offset = 5;
    let op = make_opcode(0, offset);
    let base_regs = [base, 0, 0, 0];
    let r2_val = -321;

    unsafe {
        opcode = op;
        set_registers(r2_val, base_regs);
        let addr = expected_addr(op);

        let result = ex_stb();
        assert_eq!(result, NC_STB, "Wraparound: incorrect return code");
        assert_eq!(simreg.ic, 1, "Wraparound: IC not incremented");
        let stored = read_memory(addr);
        assert_eq!(stored, r2_val, "Wraparound: Memory write incorrect");
    }
}

#[test]
fn test_ex_stb_idempotency() {
    let base_regs = [0x2222, 0x3333, 0x4444, 0x5555];
    let r2_val = 0x7ABC;
    let op = make_opcode(1, 0x20);

    unsafe {
        opcode = op;
        for i in 0..3 {
            set_registers(r2_val, base_regs);
            let addr = expected_addr(op);

            let result = ex_stb();
            assert_eq!(result, NC_STB, "Run {}: return code mismatch", i);
            assert_eq!(simreg.ic, 1, "Run {}: IC not incremented", i);
            let stored = read_memory(addr);
            assert_eq!(stored, r2_val, "Run {}: Memory write mismatch", i);
        }
    }
}