#[test]
    fn test_all_base_selectors_with_common_offsets() {
        let base_regs = [0x1000, 0x2000, 0x3000, 0x4000];
        let offsets = [0x00, 0x01, 0x7F, 0xFF];
        let help_val: i16 = 100;
        let initial_r2: i16 = 1000;

        for base_sel in 0..=3 {
            for &offset in &offsets {
                let op = make_opcode(base_sel, offset);
                unsafe {
                    opcode = op;
                    setup_simreg(initial_r2, base_regs);
                    let addr = expected_addr(op, base_regs);
                    let (page, off) = get_page_offset(addr);
                    set_memory_value(page, off, help_val as u16);

                    let result = ex_ab();
                    assert_eq!(result, NC_AB, "Return code incorrect for base {} offset {:#04X}", base_sel, offset);

                    let expected = initial_r2.wrapping_add(help_val);
                    assert_eq!(
                        simreg.r[2], expected,
                        "r[2] incorrect for base {} offset {:#04X}: expected {}, got {}",
                        base_sel, offset, expected, simreg.r[2]
                    );
                    assert_eq!(
                        simreg.ic, 1,
                        "Instruction count not incremented for base {} offset {:#04X}", base_sel, offset
                    );
                }
            }
        }
    }

    #[test]
    fn test_signed_arithmetic_overflow_cases() {
        let base_regs = [0x0000; 4];
        let test_cases = [
            (0, 0),
            (i16::MAX, 1),
            (i16::MIN, -1),
            (1000, -1000),
            (-32760, -10),
            (32760, 10),
        ];

        let op = make_opcode(2, 0x42);
        unsafe {
            opcode = op;
            for (i, &(r2_val, help_val)) in test_cases.iter().enumerate() {
                setup_simreg(r2_val, base_regs);
                let addr = expected_addr(op, base_regs);
                let (page, off) = get_page_offset(addr);
                set_memory_value(page, off, help_val as u16);

                let result = ex_ab();
                assert_eq!(result, NC_AB, "Test {}: unexpected return code", i);

                let expected = r2_val.wrapping_add(help_val);
                assert_eq!(
                    simreg.r[2], expected,
                    "Test {}: r[2] result mismatch: expected {}, got {}", i, expected, simreg.r[2]
                );
                assert_eq!(simreg.ic, 1, "Test {}: instruction counter not incremented", i);
            }
        }
    }

    #[test]
    fn test_address_wraparound_behavior() {
        let base_regs = [0xFFFF, 0x1000, 0x2000, 0x3000];
        let offset = 0x02;
        let op = make_opcode(0, offset); // base + offset → wrap to 0x0001
        let help_val = -123;
        let r2_initial = 32767;

        unsafe {
            opcode = op;
            setup_simreg(r2_initial, base_regs);
            let addr = expected_addr(op, base_regs);
            let (page, off) = get_page_offset(addr);
            set_memory_value(page, off, help_val as u16);

            let result = ex_ab();
            assert_eq!(result, NC_AB, "Unexpected return code for wraparound test");

            let expected = r2_initial.wrapping_add(help_val);
            assert_eq!(
                simreg.r[2], expected,
                "Wraparound failed: expected {}, got {}", expected, simreg.r[2]
            );
            assert_eq!(simreg.ic, 1, "Instruction counter not incremented on wraparound");
        }
    }

    #[test]
    fn test_idempotency_of_ex_ab() {
        let base_regs = [0x1111, 0x2222, 0x3333, 0x4444];
        let op = make_opcode(3, 0x10);
        let help_val: i16 = 42;
        let initial_r2: i16 = 999;

        unsafe {
            opcode = op;
            let addr = expected_addr(op, base_regs);
            let (page, off) = get_page_offset(addr);
            set_memory_value(page, off, help_val as u16);

            for i in 0..3 {
                setup_simreg(initial_r2, base_regs);
                let result = ex_ab();
                assert_eq!(result, NC_AB, "Idempotency run {}: wrong return", i);

                let expected = initial_r2.wrapping_add(help_val);
                assert_eq!(
                    simreg.r[2], expected,
                    "Idempotency run {}: expected {}, got {}", i, expected, simreg.r[2]
                );
                assert_eq!(simreg.ic, 1, "Idempotency run {}: IC not incremented", i);
            }
        }
    }