//! The 600-cell-Full tool's run on `c600-run` through the run-time boundary, onto an engine double that records each
//! proposal (LP3, C5, acceptance item 3, first part). The bridge onto the command layer is tested in `bridge.rs`.

use c600_digest::sha256;
use c600_run::boundary::{Answer, Pins, Store};
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::provenance::{self, field, Json, INCOMPATIBLE};
use c600_run::value::Value;
use c600_tools::tool::{named_word_block, Tool, ROW};

/// What the engine double did with one proposal: the word, the target and the canonical request bytes.
type Proposal = (Vec<i32>, String, Vec<u8>);

/// An engine double: it records each proposal and answers with a full receipt, or refuses every preview.
#[derive(Default)]
struct Recorder {
    proposals: Vec<Proposal>,
    refuse: Option<&'static str>,
}

impl Host for Recorder {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            // KN5.9's cost: the exact length of the literal word (LG2.17).
            Request::Propose { word, .. } => Ok(RowTariff { kernel_cost: word.len() as u128, result_bound: 4096 }),
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let Request::Propose { word, target, request } = request else {
            return Err(NOT_GRANTED);
        };
        self.proposals.push((word.to_vec(), target.to_string(), request.to_vec()));
        match self.refuse {
            Some(code) => Err(code),
            None => Ok(Value::receipt(vec![
                ("base", Value::text("engine-base")),
                ("effect", Value::text("moves 12 stickers in 3 orbits")),
                ("end_state", Value::text("end-digest")),
                ("id", Value::text("preview-17")),
                ("protection", Value::tag("clear")),
            ])),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

/// BND.42 and BND.43: in the solving profile the tool's program receives one fixed acknowledgement, `{base, id}`,
/// whether the engine built the preview, the engine refused it, or preview admission refused the word before the
/// engine saw it. The program's output and every charge row are the same in the three runs; the shell keeps the full
/// answer, and the engine double sees the literal word the run was given, once, as a proposal to `operation.preview`.
///
/// CT-BND-11 (part, the 600-cell-Full tool: in the solving profile its preview outcome is withheld) and CT-BND-07
/// (part, the 600-cell-Full tool: whatever the engine answers, the program gets the fixed acknowledgement).
#[test]
fn the_program_receives_only_the_fixed_acknowledgement() {
    let tool = Tool::full_600_cell().expect("the shipped tool loads");
    let word = [1, -2, 1200, -1200];
    let refusal = "KN2:input.recipe_shape";
    let expected_output = b"{\"base\":\"oracle-O01@1/e0\",\"id\":\"O01-0/1\"}\n";
    let mut summaries = Vec::new();
    for case in ["built", "refused by the engine", "refused at admission"] {
        let mut engine =
            Recorder { refuse: (case == "refused by the engine").then_some(refusal), ..Recorder::default() };
        let mut store = Store::default();
        let mut context = tool.context("O01-0", "oracle-O01@1/e0");
        if case == "refused at admission" {
            context.pins = Pins::default();
        }
        let ran = tool.run_in(&word, context, &mut engine, &mut store).expect("the run starts");
        assert_eq!(ran.run.summary.outcome, Outcome::Completed, "{case}");
        assert_eq!(ran.run.output.as_deref(), Some(&expected_output[..]), "{case}");
        assert!(ran.captures.is_empty(), "{case}: the solving profile captures nothing");
        let [shown] = &ran.shell[..] else { panic!("{case}: one proposal") };
        assert_eq!((shown.id.as_str(), &shown.word[..], shown.target.as_str()), ("O01-0/1", &word[..], ROW), "{case}");

        // The proposal's block: the built-in tool's output in the solving profile, citing the named word's block.
        let block = provenance::read(&store.blocks[&shown.block]).expect("the proposal's block reads");
        assert_eq!(
            (field::author(&block), field::profile(&block), field::producer(&block)),
            ("built_in", "solving", "program")
        );
        assert_eq!(field::parts(&block), [sha256(&named_word_block())]);
        assert_eq!(block.get("depends"), Some(&Json::object([("read_state", Json::text("no"))])));

        match (case, &shown.answer) {
            ("built", Answer::Previewed(receipt)) => {
                let Value::Receipt(receipt) = receipt else { panic!("a receipt") };
                assert!(receipt.get("effect").is_some() && receipt.get("end_state").is_some(), "the shell sees all");
            }
            ("refused by the engine", Answer::Refused(r)) => assert_eq!(r.code, refusal),
            ("refused at admission", Answer::Refused(r)) => assert_eq!(r.code, INCOMPATIBLE),
            (case, answer) => panic!("{case}: {answer:?}"),
        }
        let request = format!("{{\"target\":\"{ROW}\",\"word\":[1,-2,1200,-1200]}}\n");
        let seen: Vec<Proposal> = if case == "refused at admission" {
            Vec::new()
        } else {
            vec![(word.to_vec(), ROW.to_string(), request.into_bytes())]
        };
        assert_eq!(engine.proposals, seen, "{case}");
        summaries.push(ran.run.summary);
    }
    assert!(summaries.iter().all(|summary| summary == &summaries[0]), "the charges do not depend on the answer");
}

/// The tool proposes whatever literal word it is given, letter for letter, including a word the kernel would refuse:
/// it adds, drops and reorders nothing (the source route relies on this for O07's invalid words).
#[test]
fn the_tool_proposes_its_argument_unchanged() {
    let tool = Tool::full_600_cell().unwrap();
    for word in
        [vec![1], vec![0], vec![1201], vec![-1201], vec![1, 0], (1..=1200).chain((1..=1200).map(|l| -l)).collect()]
    {
        let mut engine = Recorder::default();
        let ran = tool.run(&word, "run", "base", &mut engine, &mut Store::default()).unwrap();
        assert_eq!(ran.run.summary.outcome, Outcome::Completed);
        let [(seen, target, _)] = &engine.proposals[..] else { panic!("one proposal") };
        assert_eq!((seen, target.as_str()), (&word, ROW));
    }
}
