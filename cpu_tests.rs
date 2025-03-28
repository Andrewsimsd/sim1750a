fn make_opcode(base_selector: u8, offset: u8) -> u16 {
    let base_bits = (base_selector as u16) << 8; // bits 8–9
    base_bits | offset as u16
}

fn expected_base_index(opcode: u16) -> usize {
    ((opcode & 0x0300) >> 8) as usize
}

fn expected_addr(opcode: u16, base_regs: [u16; 4]) -> u16 {
    let base_idx = expected_base_index(opcode);
    base_regs[base_idx].wrapping_add((opcode & 0xFF) as u16)
}
#[test]
fn test_ex_lb_all_bases_and_offsets() {
    unsafe {
        for base in 0..=3 {
            for offset in [0x00, 0x01, 0x7F, 0xFF] {
                let op = make_opcode(base, offset);
                opcode = op;
                simreg = SimReg::default();

                let addr = ((base as u16) << 8) + (offset as u16);
                let expected_value = 0x12345678 + (addr as u32); // Unique per address
                load_memory(addr, expected_value);

                let result = ex_lb();
                assert_eq!(result, NC_LB, "Return code failed for opcode: {:04X}", op);
                assert_eq!(simreg.r[2], expected_value, "Wrong value in r[2] for opcode: {:04X}", op);
                assert_eq!(simreg.ic, 1, "Instruction counter not incremented for opcode: {:04X}", op);
            }
        }
    }
}
//--------

#[test]
fn test_ex_dlb_exhaustive_base_and_offset_combinations() {
    unsafe {
        let mut tested_addresses = HashSet::new();

        for base in 0x04..=0x07 {
            for offset in [0x00, 0x01, 0x7F, 0xFF] {
                let op = make_opcode(base, offset);
                opcode = op;
                setup_simreg();

                let addr = expected_addr(op);
                assert!(addr < 0xFFFF, "addr + 1 would overflow");

                // Generate predictable but distinct values for each test
                let val0 = 0xA0000000 | (addr as u32);
                let val1 = 0xB0000000 | ((addr + 1) as u32);

                load_memory(addr, val0);
                load_memory(addr + 1, val1);

                let result = ex_dlb();
                assert_eq!(result, NC_DLB, "Return code mismatch for opcode {:04X}", op);
                assert_eq!(simreg.r[0], val0, "simreg.r[0] incorrect for opcode {:04X}", op);
                assert_eq!(simreg.r[1], val1, "simreg.r[1] incorrect for opcode {:04X}", op);
                assert_eq!(simreg.ic, 1, "Instruction count not incremented for opcode {:04X}", op);

                tested_addresses.insert(addr);
            }
        }

        assert!(
            tested_addresses.len() >= 12,
            "Expected at least 12 unique address tests"
        );
    }
}

#[test]
fn test_ex_dlb_idempotency_and_repeatability() {
    unsafe {
        let op = make_opcode(0x05, 0x10);
        opcode = op;
        let addr = expected_addr(op);

        let val0 = 0xDEADBEEF;
        let val1 = 0xFEEDBEEF;

        load_memory(addr, val0);
        load_memory(addr + 1, val1);

        for i in 0..3 {
            setup_simreg();
            let result = ex_dlb();
            assert_eq!(result, NC_DLB);
            assert_eq!(simreg.r[0], val0);
            assert_eq!(simreg.r[1], val1);
            assert_eq!(simreg.ic, 1, "Run {} failed", i);
        }
    }
}

#[test]
fn test_ex_dlb_memory_edge_handling() {
    unsafe {
        let op = make_opcode(0xFF, 0xFE); // base = 0xFF, offset = 0xFE => addr = 0x01FD
        opcode = op;
        setup_simreg();

        let addr = expected_addr(op);
        assert!(addr + 1 < 0x10000, "addr+1 would overflow memory");

        load_memory(addr, 0x12345678);
        load_memory(addr + 1, 0x87654321);

        let result = ex_dlb();
        assert_eq!(result, NC_DLB);
        assert_eq!(simreg.r[0], 0x12345678);
        assert_eq!(simreg.r[1], 0x87654321);
    }
}

#[test]
fn test_ex_dlb_lower_bound_memory_access() {
    unsafe {
        let op = make_opcode(0x00, 0x00);
        opcode = op;
        setup_simreg();

        load_memory(0, 0x11111111);
        load_memory(1, 0x22222222);

        let result = ex_dlb();
        assert_eq!(result, NC_DLB);
        assert_eq!(simreg.r[0], 0x11111111);
        assert_eq!(simreg.r[1], 0x22222222);
    }
}
// -------------
fn test_ex_stb_all_opcode_combinations() {
    unsafe {
        let mut seen_addrs = HashSet::new();

        for base in 0x09..=0x0B {
            for offset in [0x00, 0x01, 0x7F, 0xFF] {
                let op = make_opcode(base, offset);
                opcode = op;
                let addr = expected_addr(op);
                seen_addrs.insert(addr);

                clear_memory(addr);

                let val = 0xABCD0000 | (addr as u32); // unique test value
                setup_simreg_with_value(val);

                let result = ex_stb();
                assert_eq!(result, NC_STB, "Return value mismatch for opcode {:04X}", op);
                assert_eq!(simreg.ic, 1, "Instruction count mismatch for opcode {:04X}", op);
                let written = read_memory(addr);
                assert_eq!(written, val, "Memory write failed at addr {:04X} for opcode {:04X}", addr, op);
            }
        }

        assert_eq!(seen_addrs.len(), 12, "Expected 12 unique test addresses");
    }
}

#[test]
fn test_ex_stb_memory_edge_lowest_address() {
    unsafe {
        let op = make_opcode(0x00, 0x00);
        opcode = op;
        let addr = expected_addr(op);
        clear_memory(addr);

        let val = 0x11111111;
        setup_simreg_with_value(val);

        let result = ex_stb();
        assert_eq!(result, NC_STB);
        assert_eq!(simreg.ic, 1);
        assert_eq!(read_memory(addr), val);
    }
}

#[test]
fn test_ex_stb_memory_edge_highest_address() {
    unsafe {
        let op = make_opcode(0xFF, 0x00); // addr = 0xFF + 0x00 = 0xFF
        opcode = op;
        let addr = expected_addr(op);
        clear_memory(addr);

        let val = 0xFEEDBEEF;
        setup_simreg_with_value(val);

        let result = ex_stb();
        assert_eq!(result, NC_STB);
        assert_eq!(simreg.ic, 1);
        assert_eq!(read_memory(addr), val);
    }
}

#[test]
fn test_ex_stb_idempotency_and_repeatability() {
    unsafe {
        let op = make_opcode(0x0A, 0x20); // addr = 0xCA
        opcode = op;
        let addr = expected_addr(op);
        clear_memory(addr);

        let val = 0xCAFECAFE;
        setup_simreg_with_value(val);

        for i in 0..5 {
            simreg.ic = 0;
            let result = ex_stb();
            assert_eq!(result, NC_STB, "Iteration {i}: Return value mismatch");
            assert_eq!(simreg.ic, 1, "Iteration {i}: IC not incremented");
            assert_eq!(read_memory(addr), val, "Iteration {i}: Incorrect memory value");
        }
    }
}

#[test]
fn test_ex_stb_opcode_wraparound() {
    unsafe {
        let op = make_opcode(0xFF, 0x01); // 0xFF + 0x01 = 0x100 (wrap to 0x00 if ushort)
        opcode = op;
        let addr = expected_addr(op);
        assert_eq!(addr, 0x100);
        clear_memory(addr);

        let val = 0x12345678;
        setup_simreg_with_value(val);

        let result = ex_stb();
        assert_eq!(result, NC_STB);
        assert_eq!(simreg.ic, 1);
        assert_eq!(read_memory(addr), val);
    }
}
// ex_dstb-------
#[test]
fn test_ex_dstb_various_opcode_cases() {
    unsafe {
        let mut tested = HashSet::new();

        for base in 0x0A..=0x0F {
            for offset in [0x00, 0x01, 0x7F, 0xFF] {
                let op = make_opcode(base, offset);
                opcode = op;
                let addr = expected_addr(op);
                tested.insert(addr);

                clear_memory(addr);
                clear_memory(addr + 1);

                let val0 = 0xAA000000 | (addr as u32);
                let val1 = 0xBB000000 | ((addr + 1) as u32);

                setup_simreg(val0, val1);

                let result = ex_dstb();
                assert_eq!(result, NC_DSTB, "Incorrect return code for opcode {:04X}", op);
                assert_eq!(simreg.ic, 1, "Instruction count not incremented for opcode {:04X}", op);

                let mem0 = read_memory(addr);
                let mem1 = read_memory(addr + 1);

                assert_eq!(mem0, val0, "Memory at addr 0x{:04X} incorrect for opcode 0x{:04X}", addr, op);
                assert_eq!(mem1, val1, "Memory at addr+1 (0x{:04X}) incorrect for opcode 0x{:04X}", addr + 1, op);
            }
        }

        assert_eq!(tested.len(), 24, "Expected 24 unique address tests (6 bases × 4 offsets)");
    }
}

#[test]
fn test_ex_dstb_edge_case_lowest_address() {
    unsafe {
        let op = make_opcode(0x00, 0x00);
        opcode = op;
        let addr = expected_addr(op);
        clear_memory(addr);
        clear_memory(addr + 1);

        let val0 = 0x11111111;
        let val1 = 0x22222222;
        setup_simreg(val0, val1);

        let result = ex_dstb();
        assert_eq!(result, NC_DSTB, "Unexpected return code at low address");
        assert_eq!(simreg.ic, 1, "IC not incremented at low address");
        assert_eq!(read_memory(addr), val0, "Incorrect r0 written at addr 0x{:04X}", addr);
        assert_eq!(read_memory(addr + 1), val1, "Incorrect r1 written at addr+1 0x{:04X}", addr + 1);
    }
}

#[test]
fn test_ex_dstb_edge_case_high_address() {
    unsafe {
        let op = make_opcode(0xFF, 0x00); // base + offset = 0xFF
        opcode = op;
        let addr = expected_addr(op);
        assert!(addr < 0xFFFF, "addr + 1 would overflow memory");
        clear_memory(addr);
        clear_memory(addr + 1);

        let val0 = 0xDEAD0000 | (addr as u32);
        let val1 = 0xBEEF0000 | ((addr + 1) as u32);
        setup_simreg(val0, val1);

        let result = ex_dstb();
        assert_eq!(result, NC_DSTB, "Incorrect return code at high address");
        assert_eq!(simreg.ic, 1, "Instruction count not incremented at high address");
        assert_eq!(read_memory(addr), val0, "Incorrect write at 0x{:04X}", addr);
        assert_eq!(read_memory(addr + 1), val1, "Incorrect write at 0x{:04X}", addr + 1);
    }
}

#[test]
fn test_ex_dstb_idempotency() {
    unsafe {
        let op = make_opcode(0x0E, 0x10); // base + offset = 0x0E + 0x10 = 0x1E
        let addr = expected_addr(op);
        opcode = op;

        let val0 = 0xCAFEBABE;
        let val1 = 0x8BADF00D;
        setup_simreg(val0, val1);

        for i in 0..3 {
            simreg.ic = 0;
            clear_memory(addr);
            clear_memory(addr + 1);

            let result = ex_dstb();
            assert_eq!(result, NC_DSTB, "Iteration {i}: wrong return code");
            assert_eq!(simreg.ic, 1, "Iteration {i}: IC not incremented");
            assert_eq!(read_memory(addr), val0, "Iteration {i}: r0 not written correctly");
            assert_eq!(read_memory(addr + 1), val1, "Iteration {i}: r1 not written correctly");
        }
    }
}

#[test]
fn test_ex_dstb_wraparound_behavior() {
    unsafe {
        let op = make_opcode(0xFF, 0x01); // addr = 0xFF + 0x01 = 0x100
        opcode = op;
        let addr = expected_addr(op);
        assert_eq!(addr, 0x100);
        clear_memory(addr);
        clear_memory(addr + 1);

        let val0 = 0x0BADBEEF;
        let val1 = 0x0DEADC0D;
        setup_simreg(val0, val1);

        let result = ex_dstb();
        assert_eq!(result, NC_DSTB, "Incorrect return code at wraparound");
        assert_eq!(simreg.ic, 1, "Instruction counter not incremented at wraparound");
        assert_eq!(read_memory(addr), val0, "Incorrect r0 write at wraparound addr 0x{:04X}", addr);
        assert_eq!(read_memory(addr + 1), val1, "Incorrect r1 write at wraparound addr+1 0x{:04X}", addr + 1);
    }
}
// ex_ab -------------
#[test]
fn test_ex_ab_all_opcode_variants() {
    unsafe {
        for base in 0x10..=0x13 {
            for offset in [0x00, 0x01, 0x7F, 0xFF] {
                let op = make_opcode(base, offset);
                opcode = op;
                let addr = expected_addr(op);
                let help_value: i16 = 5;
                let initial_r2 = 10;

                setup_simreg(initial_r2);
                load_memory_short(addr, help_value);

                let result = ex_ab();
                assert_eq!(result, NC_AB, "Incorrect return code for opcode 0x{:04X}", op);
                assert_eq!(
                    simreg.r[2],
                    initial_r2.wrapping_add(help_value as i32 as u32),
                    "Incorrect addition result for opcode 0x{:04X}: r2 = {}, help = {}",
                    op,
                    initial_r2,
                    help_value
                );
                assert_eq!(simreg.ic, 1, "Instruction counter not incremented for opcode 0x{:04X}", op);
            }
        }
    }
}

#[test]
fn test_ex_ab_addition_edge_cases() {
    unsafe {
        let edge_cases: &[(u32, i16)] = &[
            (0, 0),                       // zero + zero
            (0, 123),                     // zero + positive
            (1000, -500),                 // positive + negative
            (u32::MAX, -1),               // wraparound (simulate overflow)
            (0x7FFFFFFF, 1),              // overflow to negative in signed
            (0x80000000, -1),             // underflow from signed min
            (1, i16::MIN),                // large negative
            (u32::MAX - 1, 1),            // wrap to 0
        ];

        for (i, &(r2_init, help_val)) in edge_cases.iter().enumerate() {
            let op = make_opcode(0x12, 0x34); // arbitrary valid opcode
            opcode = op;
            let addr = expected_addr(op);

            setup_simreg(r2_init);
            load_memory_short(addr, help_val);

            let result = ex_ab();
            assert_eq!(result, NC_AB, "Test {}: Unexpected return value", i);
            let expected = r2_init.wrapping_add(help_val as i32 as u32);
            assert_eq!(
                simreg.r[2], expected,
                "Test {}: Incorrect r[2] after addition: initial = {}, help = {}, expected = {}, actual = {}",
                i, r2_init, help_val, expected, simreg.r[2]
            );
            assert_eq!(simreg.ic, 1, "Test {}: Instruction counter incorrect", i);
        }
    }
}

#[test]
fn test_ex_ab_memory_address_wraparound() {
    unsafe {
        let op = make_opcode(0xFF, 0x01); // 0xFF + 0x01 = 0x100
        opcode = op;
        let addr = expected_addr(op);
        assert_eq!(addr, 0x100, "Address calculation failed");

        setup_simreg(0x100);
        load_memory_short(addr, 100);

        let result = ex_ab();
        assert_eq!(result, NC_AB, "Incorrect return code on wraparound");
        assert_eq!(simreg.r[2], 200, "Wraparound: Incorrect r[2] value after addition");
        assert_eq!(simreg.ic, 1, "Wraparound: Instruction counter not incremented");
    }
}

#[test]
fn test_ex_ab_determinism_over_repeated_calls() {
    unsafe {
        let op = make_opcode(0x11, 0x22);
        let addr = expected_addr(op);
        let help_val: i16 = 42;
        let r2_init: u32 = 1000;

        opcode = op;
        load_memory_short(addr, help_val);

        for i in 0..3 {
            setup_simreg(r2_init);
            let result = ex_ab();
            assert_eq!(result, NC_AB, "Run {}: Return code incorrect", i);
            assert_eq!(
                simreg.r[2],
                r2_init.wrapping_add(help_val as i32 as u32),
                "Run {}: Incorrect r[2] after addition", i
            );
            assert_eq!(simreg.ic, 1, "Run {}: Instruction counter not incremented", i);
        }
    }
}