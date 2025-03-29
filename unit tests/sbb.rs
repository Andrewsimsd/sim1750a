fn make_opcode(base_selector: u8, offset: u8) -> u16 {
    ((base_selector as u16) << 8) | offset as u16
}

fn expected_base_index(opcode: u16) -> usize {
    ((opcode & 0x0300) >> 8) as usize
}

fn expected_addr(opcode: u16) -> u16 {
    unsafe {
        let base_idx = expected_base_index(opcode);
        let base = c_types::simreg.r[base_idx] as u16;
        let offset = (opcode & 0xFF) as u16;
        base.wrapping_add(offset)
    }
}

fn read_memory_word(addr: u16) -> i16 {
    let page = addr >> 8;
    let offset = addr & 0xFF;
    unsafe {
        c_types::memoryBank.memPage[page as usize].memData[offset as usize] as i16
    }
}

fn setup_registers(r2_val: i16, base_regs: [u16; 4]) {
    unsafe {
        c_types::simreg.r = [0; 16];
        c_types::simreg.r[2] = r2_val;
        for (i, &val) in base_regs.iter().enumerate() {
            c_types::simreg.r[12 + i] = val as i16;
        }
        c_types::simreg.ic = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use c_types::{simreg, opcode};
    use crate::ffi::ex_sbb;
    use crate::helpers::set_memory_value;

    const NC_SBB: i32 = 0x14; // Replace with actual value of nc_SBB

    #[test]
    fn test_ex_sbb_all_base_selectors_and_offsets() {
        let base_regs = [0x1000, 0x2000, 0x3000, 0x4000];
        let offsets = [0x00, 0x01, 0x7F, 0xFF];
        let mem_val: i16 = 100;
        let r2_val: i16 = 1000;

        for base_sel in 0..=3 {
            for &offset in &offsets {
                let op = make_opcode(base_sel, offset);
                unsafe {
                    opcode = op;
                    setup_registers(r2_val, base_regs);
                    let addr = expected_addr(op);
                    let page = addr >> 8;
                    let off = addr & 0xFF;
                    set_memory_value(page, off, mem_val as u16);

                    let result = ex_sbb();
                    assert_eq!(result, NC_SBB, "Return code incorrect for base {} offset {:02X}", base_sel, offset);

                    let expected = r2_val.wrapping_sub(mem_val);
                    assert_eq!(
                        simreg.r[2], expected,
                        "Subtraction failed for base {} offset {:02X}: expected {}, got {}",
                        base_sel, offset, expected, simreg.r[2]
                    );
                    assert_eq!(simreg.ic, 1, "Instruction counter not incremented for base {} offset {:02X}", base_sel, offset);
                }
            }
        }
    }

    #[test]
    fn test_ex_sbb_signed_edge_cases() {
        let base_regs = [0x0800; 4];
        let op = make_opcode(1, 0x20);
        let test_vals = [
            (0, 0),
            (0, 1),
            (1, 0),
            (i16::MAX, 1),
            (i16::MIN, -1),
            (-1000, 1000),
            (32767, -32768),
            (-32768, 32767),
        ];

        unsafe {
            opcode = op;
            for (i, &(r2_val, mem_val)) in test_vals.iter().enumerate() {
                setup_registers(r2_val, base_regs);
                let addr = expected_addr(op);
                let page = addr >> 8;
                let off = addr & 0xFF;
                set_memory_value(page, off, mem_val as u16);

                let result = ex_sbb();
                assert_eq!(result, NC_SBB, "Test {}: Unexpected return code", i);
                let expected = r2_val.wrapping_sub(mem_val);
                assert_eq!(
                    simreg.r[2], expected,
                    "Test {}: Subtraction incorrect: expected {}, got {}", i, expected, simreg.r[2]
                );
                assert_eq!(simreg.ic, 1, "Test {}: Instruction count mismatch", i);
            }
        }
    }

    #[test]
    fn test_ex_sbb_address_wraparound() {
        let base_regs = [0xFFFC, 0, 0, 0];
        let offset = 6;
        let op = make_opcode(0, offset);
        let r2_val = 1500;
        let mem_val = 200;

        unsafe {
            opcode = op;
            setup_registers(r2_val, base_regs);
            let addr = expected_addr(op);
            let page = addr >> 8;
            let off = addr & 0xFF;
            set_memory_value(page, off, mem_val as u16);

            let result = ex_sbb();
            assert_eq!(result, NC_SBB, "Wraparound: return code incorrect");
            let expected = r2_val.wrapping_sub(mem_val);
            assert_eq!(simreg.r[2], expected, "Wraparound: subtraction result incorrect");
            assert_eq!(simreg.ic, 1, "Wraparound: IC not incremented");
        }
    }

    #[test]
    fn test_ex_sbb_idempotency() {
        let base_regs = [0x1000, 0x2000, 0x3000, 0x4000];
        let op = make_opcode(3, 0x10);
        let r2_val = 9999;
        let mem_val = -222;

        unsafe {
            opcode = op;
            let addr = expected_addr(op);
            let page = addr >> 8;
            let off = addr & 0xFF;
            set_memory_value(page, off, mem_val as u16);

            for i in 0..3 {
                setup_registers(r2_val, base_regs);
                let result = ex_sbb();
                assert_eq!(result, NC_SBB, "Run {}: return code mismatch", i);
                let expected = r2_val.wrapping_sub(mem_val);
                assert_eq!(simreg.r[2], expected, "Run {}: r[2] mismatch", i);
                assert_eq!(simreg.ic, 1, "Run {}: IC not incremented", i);
            }
        }
    }
}
