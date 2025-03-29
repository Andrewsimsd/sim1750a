/// Sets up base registers (r12..r15) and a destination register `rX`
fn setup_simreg_with_base_and_target(dst_index: usize, dst_val: i16, base_regs: [i16; 4]) {
    unsafe {
        c_types::simreg.r = [0; 16];
        c_types::simreg.r[dst_index] = dst_val;
        for (i, val) in base_regs.iter().enumerate() {
            c_types::simreg.r[12 + i] = *val;
        }
        c_types::simreg.ic = 0;
    }
}

/// Writes a short value to the memory address resolved from the given opcode
fn write_operand_to_memory(opcode: u16, value: i16) {
    let addr = crate::helpers::expected_addr(opcode);
    let page = addr >> 8;
    let offset = addr & 0xFF;
    unsafe { crate::helpers::set_memory_value(page, offset, value as u16); }
}

/// Generates (opcode, resolved address) for a base selector and offset
fn make_opcode_and_addr(base_sel: u8, offset: u8) -> (u16, u16) {
    let opcode = crate::helpers::make_opcode(base_sel, offset);
    let addr = crate::helpers::expected_addr(opcode);
    (opcode, addr)
}
#[test]
fn test_ex_lb_all_base_selectors_and_offsets() {
    let base_regs = [0x1000, 0x2000, 0x3000, 0x4000];
    let test_offsets = [0x00, 0x01, 0x7F, 0xFF];
    let memory_val: i16 = 0x3456;

    for base_sel in 0..=3 {
        for &offset in &test_offsets {
            let (op, addr) = make_opcode_and_addr(base_sel, offset);
            unsafe {
                opcode = op;
                setup_simreg_with_base_and_target(2, -1, base_regs);
                write_operand_to_memory(op, memory_val);

                let result = ex_lb();
                assert_eq!(result, NC_LB, "Base {} Offset {:02X}: Incorrect return code", base_sel, offset);
                assert_eq!(simreg.r[2], memory_val, "Base {} Offset {:02X}: r[2] not loaded correctly", base_sel, offset);
                assert_eq!(simreg.ic, 1, "Base {} Offset {:02X}: IC not incremented", base_sel, offset);
            }
        }
    }
}

#[test]
fn test_ex_lb_signed_value_handling_and_edge_cases() {
    let test_cases = [
        0i16,
        1,
        -1,
        i16::MAX,
        i16::MIN,
        12345,
        -12345,
    ];

    let base_regs = [0x1000; 4];
    let op = make_opcode(0, 0x04);
    unsafe {
        opcode = op;
        for (i, &val) in test_cases.iter().enumerate() {
            setup_simreg_with_base_and_target(2, 0, base_regs);
            write_operand_to_memory(op, val);

            let result = ex_lb();
            assert_eq!(result, NC_LB, "Case {}: Incorrect return code", i);
            assert_eq!(simreg.r[2], val, "Case {}: Incorrect r[2] result", i);
            assert_eq!(simreg.ic, 1, "Case {}: Instruction counter mismatch", i);
        }
    }
}

#[test]
fn test_ex_lb_address_wraparound_behavior() {
    let base_regs = [0xFFFE, 0, 0, 0];
    let offset = 3;
    let (op, addr) = make_opcode_and_addr(0, offset);
    let mem_val: i16 = 0x7AAA;

    unsafe {
        opcode = op;
        setup_simreg_with_base_and_target(2, 0, base_regs);
        write_operand_to_memory(op, mem_val);

        let result = ex_lb();
        assert_eq!(result, NC_LB, "Wraparound: unexpected return");
        assert_eq!(simreg.r[2], mem_val, "Wraparound: incorrect r[2] read");
        assert_eq!(simreg.ic, 1, "Wraparound: instruction count incorrect");
    }
}

#[test]
fn test_ex_lb_idempotency() {
    let base_regs = [0x2345, 0x1111, 0xAAAA, 0xBEEF];
    let op = make_opcode(1, 0x20);
    let mem_val: i16 = -2048;

    unsafe {
        opcode = op;
        write_operand_to_memory(op, mem_val);

        for i in 0..3 {
            setup_simreg_with_base_and_target(2, 0x7FFF, base_regs);
            let result = ex_lb();
            assert_eq!(result, NC_LB, "Run {}: return code incorrect", i);
            assert_eq!(simreg.r[2], mem_val, "Run {}: r[2] inconsistent", i);
            assert_eq!(simreg.ic, 1, "Run {}: IC not incremented", i);
        }
    }
}