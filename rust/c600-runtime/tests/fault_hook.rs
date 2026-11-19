//! LX-2's fault hook (feature `lx2-faults`, off by default; LX-2 is not run here). Without the feature the build has
//! no hook. With it, the test child `c600-runtime-hooked` runs `child::main_with` with a hook that records the answers
//! it is given: the hook runs before every call frame and before every result is written, with the answers the child
//! received over its life, so each call and each result reaches the caller after the hook's line for it, and the runs
//! end as they do in process. Source and synthetic evidence; no fault is injected.

#[cfg(not(feature = "lx2-faults"))]
#[test]
fn the_default_build_has_no_fault_hook() {
    const { assert!(!c600_runtime::FAULT_HOOK) };
}

#[cfg(feature = "lx2-faults")]
mod support;

#[cfg(feature = "lx2-faults")]
mod hooked {
    use std::path::{Path, PathBuf};

    use c600_run::interp::{Host, Request, RowTariff, Sink};
    use c600_run::value::Value;
    use c600_runtime::{run_in_process, Config, JobOptions, Runtime, FAULT_HOOK};

    use crate::support::{assert_same_end, header, job, Session, WIDE};

    /// Two snapshot reads, each a tariff call and a read.
    const TWO_READS: &str = "export fn two() -> Nat effects [snapshot] {\n    \
        let a = read_snapshot<Nat>(\"pieces\");\n    let b = read_snapshot<Nat>(\"pieces\");\n    \
        return exact_add(a, b);\n}\n";

    /// The lines in the hook's log so far.
    fn lines(log: &Path) -> usize {
        std::fs::read_to_string(log).map_or(0, |text| text.lines().count())
    }

    /// The caller's host: it counts the hook's lines as each call arrives, then answers as the session does. The child
    /// waits for the answer, so the count is settled; a hook that ran after the frame was written could not promise it.
    struct Watched<'a> {
        session: &'a mut Session,
        log: &'a Path,
        seen: Vec<usize>,
    }

    impl Watched<'_> {
        fn look(&mut self) {
            self.seen.push(lines(self.log));
        }
    }

    impl Host for Watched<'_> {
        fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
            self.look();
            self.session.tariff(request)
        }

        fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
            self.look();
            self.session.perform(request)
        }

        fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
            self.look();
            self.session.sink(sink, value, bytes)
        }
    }

    #[test]
    fn the_hook_runs_before_each_call_frame_and_each_result() {
        const { assert!(FAULT_HOOK) };
        let log =
            PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c600-runtime-hook-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&log);
        let mut rt = Runtime::new(Config::new(env!("CARGO_BIN_EXE_c600-runtime-hooked")));
        rt.config_mut().env.push(("C600_RUNTIME_HOOK_LOG".into(), log.clone().into_os_string()));
        let two = job(&format!("{}{TWO_READS}", header("")), "two", vec![], JobOptions::new(WIDE));

        let mut expected = Vec::new();
        let mut answers = 0;
        for _ in 0..2 {
            let mut reference = Session::default();
            let in_process = run_in_process(&two, &mut reference, None);
            let mut session = Session::default();
            let mut watched = Watched { session: &mut session, log: &log, seen: Vec::new() };
            let got = rt.run(&two, &mut watched, None).unwrap_or_else(|f| panic!("{f}"));
            let seen = watched.seen;
            assert_same_end(&in_process, &got);
            reference.assert_same(&session);
            let calls = session.log.len() as u64;
            assert_eq!(calls, 4, "two tariffs and two reads: {:?}", session.log);
            // The hook sees the answers so far before each call frame, then all of them before the result; each call
            // reaches the caller with its own line written, and so does the result.
            let before = expected.len();
            expected.extend(answers..answers + calls);
            assert_eq!(seen, (before + 1..=expected.len()).collect::<Vec<_>>(), "the hook's lines as each call came");
            answers += calls;
            expected.push(answers);
            assert_eq!(lines(&log), expected.len(), "the hook's line for the result came before the result");
        }
        assert_eq!(rt.spawns(), 1, "one child ran both jobs, so its count of answers runs on");

        let text = std::fs::read_to_string(&log).expect("the hook's log");
        let _ = std::fs::remove_file(&log);
        let seen: Vec<u64> = text.lines().map(|line| line.parse().expect("a count")).collect();
        assert_eq!(seen, expected);
    }
}
