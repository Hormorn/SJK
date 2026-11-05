//! Linux takes its main-thread stack limit from the launching process, rather
//! than the executable's linker settings. Set it before constructing the client.

use crate::stack_size::MAIN_STACK_BYTES;

pub(super) fn configure() {
    let (soft, hard) = match rlimit::Resource::STACK.get() {
        Ok(limits) => limits,
        Err(error) => {
            crate::log::progress(format_args!(
                "warning: could not read main-stack limit: {error}"
            ));
            return;
        }
    };
    // Never lower an inherited allowance or change the administrator's hard limit.
    let wanted = soft.max(MAIN_STACK_BYTES.min(hard));
    if wanted > soft {
        if let Err(error) = rlimit::Resource::STACK.set(wanted, hard) {
            crate::log::progress(format_args!(
                "warning: could not raise main-stack limit from {soft} to {wanted}: {error}"
            ));
            return;
        }
        crate::log::progress(format_args!("main-stack limit: soft {soft} -> {wanted}"));
    }
    if wanted < MAIN_STACK_BYTES {
        crate::log::progress(format_args!(
            "warning: main-stack hard limit {hard} is below the client's {MAIN_STACK_BYTES} byte allowance; startup may overflow"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_main_stack_limits() {
        const CASE: &str = "SJK_TEST_MAIN_STACK_LIMIT";
        if let Ok(case) = std::env::var(CASE) {
            let mib = 1024 * 1024;
            let (soft, hard, expected) = match case.as_str() {
                "normal" => (8 * mib, rlimit::INFINITY, MAIN_STACK_BYTES),
                "larger" => (128 * mib, rlimit::INFINITY, 128 * mib),
                "unlimited" => (rlimit::INFINITY, rlimit::INFINITY, rlimit::INFINITY),
                "capped" => (8 * mib, 32 * mib, 32 * mib),
                _ => panic!("unknown stack-limit case"),
            };
            // A restricted test host may itself have a finite hard limit.
            let inherited_hard = rlimit::Resource::STACK.get().unwrap().1;
            let hard = hard.min(inherited_hard);
            let soft = soft.min(hard);
            let expected = expected.min(hard);
            rlimit::Resource::STACK.set(soft, hard).unwrap();
            configure();
            assert_eq!(rlimit::Resource::STACK.get().unwrap(), (expected, hard));
            return;
        }
        // Limits are process-wide: isolate each case from the parallel test runner.
        for case in ["normal", "larger", "unlimited", "capped"] {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "platform::main_stack::tests::linux_main_stack_limits",
                    "--nocapture",
                ])
                .env(CASE, case)
                .status()
                .unwrap();
            assert!(status.success(), "stack-limit case {case}");
        }
    }
}
