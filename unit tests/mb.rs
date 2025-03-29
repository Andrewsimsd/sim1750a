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
        base.wrapping_add((opcode & 0xFF) as u16)
    }
}

fn get_page_and_offset(addr: u16) -> (u16, u16) {
    (addr >> 8, addr & 0xFF)
}

fn setup_registers(r2_val: i16, base_regs: [u16; 4]) {
    unsafe {
        c_types::simreg.r = [0; 16];
        c_types::simreg.r[2] = r2_val;
        for i in 0..4 {
            c_types::simreg.r[12 + i] = base_regs[i] as i16;
        }
        c_types::simreg.ic = 0;
    }
}

fn read_memory_word(addr: u16) -> i16 {
    let (page, offset) = get_page_and_offset(addr);
    unsafe {
        c_types::memoryBank.memPage[page as usize].memData[offset as usize] as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c_types::{simreg, opcode};
    use crate::ffi::ex_mb;
    use crate::helpers::set_memory_value;

    const NC_MB: i32 = 0x18; // replace with actual nc_MB value

    #[test]
    fn test_ex_mb_all_base_selectors_and_offsets() {
        let base_regs = [0x1000, 0x2000, 0x3000, 0x4000];
        let offsets = [0x00, 0x01, 0x7F, 0xFF];
        let r2_val: i16 = 3;
        let mem_val: i16 = 4;

        for base_sel in 0..=3 {
            for &offset in &offsets {
                let op = make_opcode(base_sel, offset);
                unsafe {
                    opcode = op;
                    setup_registers(r2_val, base_regs);
                    let addr = expected_addr(op);
                    let (page, offset) = get_page_and_offset(addr);
                    set_memory_value(page, offset, mem_val as u16);

                    let result = ex_mb();
                    assert_eq!(result, NC_MB, "Base {} Offset {:02X}: Incorrect return code", base_sel, offset);
                    assert_eq!(simreg.ic, 1, "Base {} Offset {:02X}: IC not incremented", base_sel, offset);
                    let expected = r2_val.wrapping_mul(mem_val);
                    assert_eq!(
                        simreg.r[2], expected,
                        "Base {} Offset {:02X}: Incorrect multiplication result: expected {}, got {}",
                        base_sel, offset, expected, simreg.r[2]
                    );
                }
            }
        }
    }

    #[test]
    fn test_ex_mb_signed_and_overflow_cases() {
        let base_regs = [0x0800; 4];
        let op = make_opcode(1, 0x42);
        let test_cases = [
            (0, 0),
            (1, 0),
            (0, 1),
            (i16::MAX, 1),
            (i16::MIN, 1),
            (-123, 456),
            (32767, 2),
            (-32768, 2),
        ];

        unsafe {
            opcode = op;
            for (i, &(r2_val, mem_val)) in test_cases.iter().enumerate() {
                setup_registers(r2_val, base_regs);
                let addr = expected_addr(op);
                let (page, offset) = get_page_and_offset(addr);
                set_memory_value(page, offset, mem_val as u16);

                let result = ex_mb();
                assert_eq!(result, NC_MB, "Test {}: Incorrect return code", i);
                let expected = r2_val.wrapping_mul(mem_val);
                assert_eq!(
                    simreg.r[2], expected,
                    "Test {}: Incorrect result: expected {}, got {}", i, expected, simreg.r[2]
                );
                assert_eq!(simreg.ic, 1, "Test {}: IC not incremented", i);
            }
        }
    }

    #[test]
    fn test_ex_mb_address_wraparound() {
        let base_regs = [0xFFFE, 0, 0, 0];
        let offset = 5;
        let op = make_opcode(0, offset);
        let r2_val = -7;
        let mem_val = -9;

        unsafe {
            opcode = op;
            setup_registers(r2_val, base_regs);
            let addr = expected_addr(op);
            let (page, offset) = get_page_and_offset(addr);
            set_memory_value(page, offset, mem_val as u16);

            let result = ex_mb();
            assert_eq!(result, NC_MB, "Wraparound: Incorrect return code");
            let expected = r2_val.wrapping_mul(mem_val);
            assert_eq!(simreg.r[2], expected, "Wraparound: multiplication result incorrect");
            assert_eq!(simreg.ic, 1, "Wraparound: IC not incremented");
        }
    }

    #[test]
    fn test_ex_mb_idempotency() {
        let base_regs = [0x2345, 0x4567, 0x6789, 0x789A];
        let r2_val = 100;
        let mem_val = -3;
        let op = make_opcode(3, 0x10);

        unsafe {
            opcode = op;
            let addr = expected_addr(op);
            let (page, offset) = get_page_and_offset(addr);
            set_memory_value(page, offset, mem_val as u16);

            for i in 0..3 {
                setup_registers(r2_val, base_regs);
                let result = ex_mb();
                assert_eq!(result, NC_MB, "Run {}: Incorrect return code", i);
                let expected = r2_val.wrapping_mul(mem_val);
                assert_eq!(
                    simreg.r[2], expected,
                    "Run {}: Incorrect result: expected {}, got {}", i, expected, simreg.r[2]
                );
                assert_eq!(simreg.ic, 1, "Run {}: IC not incremented", i);
            }
        }
    }
}
