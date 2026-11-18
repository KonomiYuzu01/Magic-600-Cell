//! CT-LB2-02's Net and Strict halves and CT-LB2-03's oversize and chunking parts through the 600-cell-Full tool's
//! route (LP3, C5's next piece, acceptance items 3 and 4; the Strict half on the command layer's Strict mode, LP2's
//! pull request 198). Each test runs the tool on the in-process host and on the process host. Neither ID runs whole
//! here: each test's comment names the part it covers and the part that is not built.

mod process_support;

use c600_command::{
    code, draft_args, CommandLayer, Completion, CompletionKind, Error, HeadlessFrames, Preview, Receipt, Shell, Status,
    POLICY_ROW,
};
use c600_kernel::codes::{PRIMITIVE_CEILING, RECIPE_SHAPE};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Level, ProtectedSet, ProtectionMode, Verdict};
use c600_run::meter::Outcome;
use c600_tools::bridge::{proposal_args, word_args, Steps};
use c600_tools::expand::{expand, literal_digest, word_steps, Expansion};
use c600_tools::process::ProcessHost;
use c600_tools::tool::{Tool, ROW};
use process_support::{chunked, fingerprint, kernel, on, process_host, run_on, run_with, session, session_with, HOSTS};

/// 0.4's limits, which LB2.4 keeps: the letters of one word step and the primitives of one recipe.
const WORD_LIMIT: usize = 100_000;
const TOTAL_LIMIT: usize = 3_000_000;

/// A word of `length` generator letters, none the inverse of its neighbour.
fn letters(length: usize) -> Vec<i64> {
    (0..length).map(|index| (index % 5) as i64 + 1).collect()
}

fn narrow(word: &[i64]) -> Vec<i32> {
    word.iter().map(|&letter| letter as i32).collect()
}

/// The command route's preview of `recipe`: the shell's own `operation.preview` invocation.
fn command_preview(layer: &mut CommandLayer, shell: &mut Shell, recipe: &Json) -> Receipt {
    shell.invoke(layer, ROW, draft_args(recipe)).expect("the shell sends a preview without a token")
}

/// The tool's route for `word` on one host: the run completes, and its one invocation's receipt comes back.
fn tool_preview(host: Option<&mut ProcessHost>, tool: &Tool, layer: &mut CommandLayer, word: &[i32]) -> Receipt {
    let (ran, sent) = run_on(host, tool, layer, word, "tool");
    let ran = ran.unwrap_or_else(|stopped| panic!("the run ends: {:?}", stopped.cause));
    assert_eq!(ran.run.summary.outcome, Outcome::Completed);
    let [one] = &sent[..] else { panic!("one invocation, not {}", sent.len()) };
    assert_eq!(one.invocation.args.canonical(), word_args(word).canonical(), "the word reaches the layer unchanged");
    one.receipt.clone()
}

/// Runs the admitted preview job and presents it in the next headless frame.
fn present(layer: &mut CommandLayer, shell: &mut Shell, admitted: &Receipt) -> Preview {
    let Status::Running(id) = admitted.status else { panic!("not admitted: {admitted:?}") };
    assert_eq!(layer.run_jobs(), 1);
    HeadlessFrames::after(layer).present(shell, layer, id).expect("the built preview is presented");
    layer.preview().filter(|preview| preview.id == id).cloned().expect("the presented preview")
}

fn commit(layer: &mut CommandLayer, shell: &mut Shell) -> Receipt {
    shell.press(layer, "Ctrl+Shift+Enter", false).expect("the chord is sent").expect("not a repeat")
}

fn text(value: &str) -> Json {
    Json::String(value.to_owned())
}

/// A session as [`session_with`] makes it, which the shell then switches to Strict with its own invocation of
/// `protection.policy.set` (LP2's `POLICY_ROW`, which the runtime may not invoke): a durable workspace action that
/// answers `completed(workspace)` and keeps the head and the state.
fn strict_session(name: &str, protected: ProtectedSet) -> (CommandLayer, Shell) {
    let (mut layer, mut shell) = session_with(name, protected);
    assert_eq!(layer.policy(), ProtectionMode::Net, "a session starts under Net");
    let before = fingerprint(&layer);
    let strict = json::object([("confirm", Json::Bool(true)), ("policy", text("strict"))]);
    let set = shell.invoke(&mut layer, POLICY_ROW, strict).expect("the shell sends a policy change without a token");
    assert!(matches!(set.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })), "{set:?}");
    assert_eq!(layer.policy(), ProtectionMode::Strict);
    let after = fingerprint(&layer);
    assert_eq!((&after.0, &after.1, after.2), (&before.0, &before.1, before.2), "the head and the state stay");
    assert_eq!(after.3, before.3 + 1, "one durable workspace action");
    (layer, shell)
}

/// CT-LB2-02, the Net half (LB2.3: Net tests the end of the complete operation). With the orbits a generator `g`
/// moves protected, the words `g -g` and `g h -h -g` have identity net effect while every proper prefix moves a
/// protected orbit. Through the tool's route, on either host, the whole word previews with no sticker moved, a
/// protection row at level labels in mode Net with verdict clear and no conflict, and an end state equal to its
/// base; the shell's chord commits it as one transaction that leaves the state where it was. The moving prefix `g`
/// alone previews with verdict moves and no end state, and its commit is refused with the kernel's protection code.
///
/// The Strict half is the next test, on the same words and protected set. Neither test runs "an incomplete prefix
/// check never returns safe" or the captured lock through the tool's route; the next test says why.
#[test]
fn ct_lb2_02_net_half_an_identity_word_with_moving_prefixes_passes_net_through_the_tools_route() {
    let kernel = kernel();
    let tool = Tool::full_600_cell().unwrap();
    let (g, h) = (1i64, 2i64);
    let effect = |word: &[i64]| kernel.effect(&kernel.word(word).unwrap()).unwrap();
    let orbits: Vec<i64> = effect(&[g]).support.iter().map(|row| row.orbit as i64).collect();
    assert!(!orbits.is_empty(), "the generator moves an orbit");
    let protected = kernel.protected_set(&orbits).unwrap();
    for word in [&[g, -g][..], &[g, h, -h, -g]] {
        for end in 1..word.len() {
            let prefix = effect(&word[..end]);
            assert!(prefix.moved_stickers() > 0, "{word:?}: the prefix of {end} letters moves");
            assert_eq!(kernel.protection(&prefix, &protected).verdict, Verdict::Moves, "{word:?}: prefix {end}");
        }
        assert_eq!(effect(word).moved_stickers(), 0, "{word:?}: identity net effect");
    }

    let mut process = process_host();
    for host in HOSTS {
        for word in [&[g, -g][..], &[g, h, -h, -g]] {
            let (mut layer, mut shell) = session_with("net", protected.clone());
            let before = fingerprint(&layer);
            let admitted = tool_preview(on(host, &mut process), &tool, &mut layer, &narrow(word));
            let preview = present(&mut layer, &mut shell, &admitted);
            assert_eq!(preview.literal, literal_digest(&narrow(word)), "{host} {word:?}");
            assert_eq!((preview.effect.moved_stickers(), preview.effect.primitive_count), (0, word.len()), "{host}");
            let row = &preview.protection;
            assert!(matches!(row.mode, ProtectionMode::Net), "{host}: a Net row");
            assert_eq!((row.level, row.verdict, row.conflicts.len()), (Level::Labels, Verdict::Clear, 0), "{host}");
            assert_eq!(row.protected.len(), orbits.len(), "{host}: the row names the protected orbits");
            let end = preview.end.as_ref().expect("a clear preview has an end state");
            assert_eq!((preview.base.digest(), end.digest()), (&*before.0, &*before.0), "{host} {word:?}");
            let committed = commit(&mut layer, &mut shell);
            let Status::Committed(record) = &committed.status else { panic!("{host} {word:?}: {committed:?}") };
            assert_eq!((&record.pre, &record.post), (&before.0, &before.0), "{host} {word:?}");
            assert_eq!(layer.snapshot().digest(), before.0, "{host} {word:?}: the state is where it was");
            assert_eq!(layer.store().log.len(), before.3 + 1, "{host} {word:?}: one transaction");
        }

        let (mut layer, mut shell) = session_with("net", protected.clone());
        let before = fingerprint(&layer);
        let admitted = tool_preview(on(host, &mut process), &tool, &mut layer, &[g as i32]);
        let preview = present(&mut layer, &mut shell, &admitted);
        assert_eq!(preview.protection.verdict, Verdict::Moves, "{host}");
        assert!(preview.end.is_none() && !preview.protection.conflicts.is_empty(), "{host}");
        let refused = commit(&mut layer, &mut shell);
        assert_eq!(refused.status.code(), Some(code::ORBIT_WOULD_MOVE), "{host}: {refused:?}");
        assert_eq!(fingerprint(&layer), before, "{host}: a refused commit changes nothing");
    }
}

/// CT-LB2-02 (part: the Strict half through the 600-cell-Full tool's route, on both hosts; not run: the captured lock,
/// LB2.3's coverage report and `LB2:protection.incomplete_coverage`, which are LP3's library rows), the Strict half
/// (LB2.3: Strict tests every executed literal prefix; KN2.4, KN2.7). The protected set is the Net half's, the orbits
/// a generator `g` moves, and the shell sets the session's policy to Strict with `protection.policy.set` before the
/// tool runs. On either host the tool's proposal of `g -g` or `g h -h -g`, which pass Net above, builds a preview
/// whose protection row is the kernel's `strict_protection` of the word over the layer's protected set: mode Strict
/// at level labels, verdict moves, the conflicts of the first refused prefix and its length, 1, the shortest prefix
/// whose net map moves a protected orbit. The preview's summary adds `mode: "strict"` and `first_refused_prefix: 1`
/// to verdict `moves` and a null end, and the shell's chord is refused with the kernel's protection code: the state
/// and the journal stay where they were. The command route's preview of the same recipe under Strict has the same
/// summary, protection row and operation (literal, expansion and net digests, count and end state), and its commit
/// gets the same refusal. Without a protected orbit Strict clears every prefix: on either host the word previews
/// clear with a null `first_refused_prefix` and commits as one transaction that leaves the state where it was.
///
/// Not built through the tool's route: "an incomplete prefix check never returns safe" and "the relevant captured
/// lock". The captured lock of LB2's row "Net and Strict protection", LB2.3's coverage report and
/// `LB2:protection.incomplete_coverage` are LP3's library rows (the LP3 plan, section 7), as LP2's Strict test in
/// `c600-command/tests/strict_policy.rs` says: library entries over kernel rows, which wait for a snapshot read that
/// takes a run's input as its argument. The IR draft has that read since LP2's pull request 199
/// (`read_snapshot<T>("q", x)`), and no host answers it yet: the bridge grants a run no state read. Here the
/// session's protected set stands for the lock, as in the Net half. `c600-library/tests/ct_lb2_02.rs` runs the
/// Strict half at the kernel, on words the library builds.
#[test]
fn ct_lb2_02_strict_half_an_identity_word_with_moving_prefixes_fails_strict_through_the_tools_route() {
    let kernel = kernel();
    let tool = Tool::full_600_cell().unwrap();
    let (g, h) = (1i64, 2i64);
    let effect = |word: &[i64]| kernel.effect(&kernel.word(word).unwrap()).unwrap();
    let orbits: Vec<i64> = effect(&[g]).support.iter().map(|row| row.orbit as i64).collect();
    let protected = kernel.protected_set(&orbits).unwrap();
    let words = [&[g, -g][..], &[g, h, -h, -g]];
    // The shortest prefix whose net map moves a protected orbit, by Net's verdict on each prefix.
    let moves = |prefix: &[i64]| kernel.protection(&effect(prefix), &protected).verdict == Verdict::Moves;
    let first_refused = |word: &[i64]| (1..=word.len()).find(|&end| moves(&word[..end])).map(|end| end as u64);
    for word in words {
        assert_eq!(effect(word).moved_stickers(), 0, "{word:?}: identity net effect");
        assert_eq!(first_refused(word), Some(1), "{word:?}: the first letter moves a protected orbit");
    }

    let mut process = process_host();
    for word in words {
        let first = first_refused(word).map(|prefix| Json::Int(prefix as i64));
        let (mut layer, mut shell) = strict_session("strict", protected.clone());
        let before = fingerprint(&layer);
        let admitted = command_preview(&mut layer, &mut shell, &chunked(&[word]));
        let command = present(&mut layer, &mut shell, &admitted);
        let refusal = commit(&mut layer, &mut shell);
        assert_eq!(refusal.status.code(), Some(code::ORBIT_WOULD_MOVE), "{word:?}: {refusal:?}");
        assert_eq!(fingerprint(&layer), before, "{word:?}: a refused commit changes nothing");

        for host in HOSTS {
            let (mut layer, mut shell) = strict_session("strict", protected.clone());
            let before = fingerprint(&layer);
            let admitted = tool_preview(on(host, &mut process), &tool, &mut layer, &narrow(word));
            let preview = present(&mut layer, &mut shell, &admitted);
            let strict = kernel.strict_protection(&kernel.word(word).unwrap(), layer.protected()).unwrap();
            assert_eq!(preview.protection, strict, "{host} {word:?}: the kernel's Strict row");
            let row = &preview.protection;
            assert_eq!((row.mode, row.level, row.verdict), (ProtectionMode::Strict, Level::Labels, Verdict::Moves));
            assert_eq!(row.first_refused_prefix, first_refused(word), "{host} {word:?}");
            assert_eq!(row.protected.len(), orbits.len(), "{host}: the row names the protected orbits");
            assert!(!row.conflicts.is_empty() && preview.end.is_none(), "{host} {word:?}");
            assert_eq!(preview.literal, literal_digest(&narrow(word)), "{host} {word:?}");
            assert_eq!((preview.effect.moved_stickers(), preview.effect.primitive_count), (0, word.len()), "{host}");
            let summary = preview.summary();
            let keys = ["mode", "first_refused_prefix", "verdict", "end"].map(|key| summary.get(key).cloned());
            let expected = [Some(text("strict")), first.clone(), Some(text("moves")), Some(Json::Null)];
            assert_eq!(keys, expected, "{host} {word:?}: the Strict summary");

            assert_eq!(summary, command.summary(), "{host} {word:?}: the command route's summary");
            assert_eq!(preview.protection, command.protection, "{host} {word:?}: the command route's row");
            assert_eq!(whole(&preview), whole(&command), "{host} {word:?}: the command route's operation");
            let refused = commit(&mut layer, &mut shell);
            assert_eq!(refused.status, refusal.status, "{host} {word:?}: the command route's refusal");
            assert_eq!(fingerprint(&layer), before, "{host} {word:?}: a refused commit changes nothing");
            assert_eq!(layer.policy(), ProtectionMode::Strict, "{host} {word:?}");
        }
    }

    // Without a protected orbit Strict clears every prefix.
    for host in HOSTS {
        for word in words {
            let (mut layer, mut shell) = strict_session("strict", ProtectedSet::none());
            let before = fingerprint(&layer);
            let admitted = tool_preview(on(host, &mut process), &tool, &mut layer, &narrow(word));
            let preview = present(&mut layer, &mut shell, &admitted);
            let strict = kernel.strict_protection(&kernel.word(word).unwrap(), layer.protected()).unwrap();
            assert_eq!(preview.protection, strict, "{host} {word:?}: the kernel's Strict row");
            let row = &preview.protection;
            assert_eq!(
                (row.mode, row.verdict, row.first_refused_prefix),
                (ProtectionMode::Strict, Verdict::Clear, None)
            );
            assert!(row.protected.is_empty() && row.conflicts.is_empty(), "{host} {word:?}");
            let summary = preview.summary();
            let keys = ["mode", "first_refused_prefix", "verdict"].map(|key| summary.get(key).cloned());
            assert_eq!(keys, [Some(text("strict")), Some(Json::Null), Some(text("clear"))], "{host} {word:?}");
            let end = preview.end.as_ref().expect("a clear preview has an end state");
            assert_eq!((preview.base.digest(), end.digest()), (&*before.0, &*before.0), "{host} {word:?}");
            let committed = commit(&mut layer, &mut shell);
            let Status::Committed(record) = &committed.status else { panic!("{host} {word:?}: {committed:?}") };
            assert_eq!((&record.pre, &record.post), (&before.0, &before.0), "{host} {word:?}");
            assert_eq!(layer.snapshot().digest(), before.0, "{host} {word:?}: the state is where it was");
            assert_eq!(layer.store().log.len(), before.3 + 1, "{host} {word:?}: one transaction");
        }
    }
}

/// CT-LB2-03, the oversize refusal (LB2.3: a program is never split silently; LB2.4: word chunks of at most 100,000
/// primitives). A recipe of one word of 100,001 letters is refused with the kernel's recipe-shape code on the
/// command route. On the source route the recipe has no checked expansion, the tool proposes the word whole, and the
/// one invocation that reaches the layer, from either host, carries one word step of 100,001 letters and is refused
/// with the same code and detail: no job, no preview, no change. A word of exactly 100,000 letters is admitted on
/// both routes.
#[test]
fn ct_lb2_03_an_oversized_word_is_refused_whole_on_the_command_and_source_routes_with_the_same_code() {
    let kernel = kernel();
    let tool = Tool::full_600_cell().unwrap();
    let mut process = process_host();
    let over = letters(WORD_LIMIT + 1);
    let recipe = chunked(&[&over]);

    let (mut layer, mut shell) = session("oversize");
    let before = fingerprint(&layer);
    let command = command_preview(&mut layer, &mut shell, &recipe);
    let Status::Refused(error) = &command.status else { panic!("{command:?}") };
    assert_eq!((error.code, command.job), (RECIPE_SHAPE, None));
    assert!(layer.preview().is_none() && layer.run_jobs() == 0);
    assert_eq!(fingerprint(&layer), before);

    let Expansion::Unchecked { letters: literal, refusal } = expand(&kernel, &recipe) else {
        panic!("the kernel refuses the recipe, which is one word of integer moves")
    };
    assert_eq!((refusal.code, literal.len()), (RECIPE_SHAPE, WORD_LIMIT + 1));
    assert_eq!(&Error::from(refusal), error, "the expansion's refusal is the command route's");
    for host in HOSTS {
        let (mut layer, _) = session("oversize");
        let (ran, sent) = run_on(on(host, &mut process), &tool, &mut layer, &literal, "tool");
        assert_eq!(ran.unwrap().run.summary.outcome, Outcome::Completed, "{host}");
        let [one] = &sent[..] else { panic!("{host}: one invocation, not {}", sent.len()) };
        assert_eq!(one.invocation.args.canonical(), draft_args(&recipe).canonical(), "{host}: one unsplit word step");
        assert_eq!(one.receipt.status, command.status, "{host}: the same refusal as the command route's");
        assert_eq!(one.receipt.job, None, "{host}");
        assert!(layer.preview().is_none() && layer.run_jobs() == 0, "{host}");
        assert_eq!(fingerprint(&layer), before, "{host}");
    }

    let most = letters(WORD_LIMIT);
    let (mut layer, mut shell) = session("oversize");
    let admitted = command_preview(&mut layer, &mut shell, &chunked(&[&most]));
    assert!(matches!(admitted.status, Status::Running(_)), "{admitted:?}");
    for host in HOSTS {
        let (mut layer, _) = session("oversize");
        let on = on(host, &mut process);
        let admitted = tool_preview(on, &tool, &mut layer, &narrow(&most));
        assert!(matches!(admitted.status, Status::Running(_)), "{host}: {admitted:?}");
    }
}

/// CT-LB2-03, chunking (LB2.4: a total of at most 3,000,000 primitives, and a chunk cannot raise the total; LB1.11:
/// chunking transports one operation and creates no new commit or protection boundary). The command layer's
/// chunking is the 0.4 recipe's word steps.
///
/// The total bound: 30 word steps of 100,000 letters, 3,000,000 in all, are admitted on the command route, and one
/// more letter in a further step is refused with the kernel's primitive-ceiling code. The source route refuses that
/// recipe at its expansion with the same code and detail, before the tool runs, so nothing is proposed.
///
/// The whole operation: a word cut into one, three and twelve word steps previews on the command route with the
/// same literal, expansion and net digests and the same end state, and commits as one transaction. With the orbits
/// of the first chunk protected, the two chunks `g` and `-g` preview clear: a chunk boundary is no protection
/// boundary. On the source route, on either host, the chunked recipe expands to the same literal word, and the
/// tool's proposal of it previews with the same literal, effect and end state. A literal word of at most one chunk
/// goes in one word step, whatever word steps the solver wrote, as before chunked transport.
///
/// Chunked transport on the source route: a recipe of two full chunks (200,000 letters) previews on the command route
/// and commits as one transaction. On either host the tool runs on the recipe's literal word through a bridge given
/// the recipe's word steps, as the runner gives them; the one invocation carries the solver's two word steps
/// unchanged, the preview has the command route's literal, expansion and net digests, count and end state, and the
/// shell's chord commits it as one transaction.
///
/// Not built: "Named work-sheet reuse invalidates the old preview": this route has no named work sheet.
#[test]
fn ct_lb2_03_chunking_keeps_the_total_admission_bound_and_the_whole_operation() {
    let kernel = kernel();
    let tool = Tool::full_600_cell().unwrap();
    let mut process = process_host();

    let chunk = letters(WORD_LIMIT);
    let mut chunks: Vec<&[i64]> = vec![&chunk; TOTAL_LIMIT / WORD_LIMIT];
    let at_bound = chunked(&chunks);
    let (mut layer, mut shell) = session("chunks");
    let before = fingerprint(&layer);
    let admitted = command_preview(&mut layer, &mut shell, &at_bound);
    assert!(matches!(admitted.status, Status::Running(_)), "3,000,000 primitives in 30 chunks: {:?}", admitted.status);
    drop(at_bound);
    chunks.push(&[1]);
    let above = chunked(&chunks);
    let (mut layer, mut shell) = session("chunks");
    let refused = command_preview(&mut layer, &mut shell, &above);
    let Status::Refused(error) = &refused.status else { panic!("{refused:?}") };
    assert_eq!((error.code, refused.job), (PRIMITIVE_CEILING, None));
    assert!(layer.preview().is_none() && layer.run_jobs() == 0);
    assert_eq!(fingerprint(&layer), before);
    let Expansion::Refused(refusal) = expand(&kernel, &above) else { panic!("no literal word above the total") };
    assert_eq!(refusal.code, PRIMITIVE_CEILING);
    assert_eq!(&Error::from(refusal), error, "the expansion's refusal is the command route's");
    drop(above);

    let word = letters(12);
    let mut seen = Vec::new();
    for size in [12, 4, 1] {
        let pieces: Vec<&[i64]> = word.chunks(size).collect();
        let recipe = chunked(&pieces);
        let (mut layer, mut shell) = session("chunks");
        let admitted = command_preview(&mut layer, &mut shell, &recipe);
        let preview = present(&mut layer, &mut shell, &admitted);
        assert_eq!(preview.recipe.steps().len(), pieces.len());
        let committed = commit(&mut layer, &mut shell);
        assert!(matches!(committed.status, Status::Committed(_)), "{committed:?}");
        assert_eq!(layer.store().log.len(), before.3 + 1, "{} chunks commit as one transaction", pieces.len());
        let Expansion::Literal(literal) = expand(&kernel, &recipe) else { panic!("the recipe expands") };
        assert_eq!(literal, narrow(&word), "{} chunks expand to the one word", pieces.len());
        seen.push(whole(&preview));
    }
    assert!(seen.iter().all(|operation| operation == &seen[0]), "the chunking changes the operation: {seen:?}");
    assert_eq!(seen[0].0, literal_digest(&narrow(&word)));
    let flat = narrow(&word);
    let solver = Steps::new(&flat, vec![4; 3]);
    let cut = solver.as_ref().and_then(|steps| steps.cut(&flat)).map(|cut| cut.len());
    assert_eq!(cut, Some(3), "the solver's three steps fit the literal word");
    assert_eq!(proposal_args(&flat, solver.as_ref()), word_args(&flat), "one chunk: one word step");
    for host in HOSTS {
        let (mut layer, mut shell) = session("chunks");
        let on = on(host, &mut process);
        let admitted = tool_preview(on, &tool, &mut layer, &narrow(&word));
        assert_eq!(whole(&present(&mut layer, &mut shell, &admitted)), seen[0], "{host}");
    }

    let g = 1i64;
    let moved = kernel.effect(&kernel.word(&[g]).unwrap()).unwrap();
    let orbits: Vec<i64> = moved.support.iter().map(|row| row.orbit as i64).collect();
    let protected: ProtectedSet = kernel.protected_set(&orbits).unwrap();
    let (mut layer, mut shell) = session_with("chunks", protected);
    let admitted = command_preview(&mut layer, &mut shell, &chunked(&[&[g], &[-g]]));
    let preview = present(&mut layer, &mut shell, &admitted);
    assert_eq!((preview.protection.verdict, preview.effect.moved_stickers()), (Verdict::Clear, 0));
    assert!(matches!(commit(&mut layer, &mut shell).status, Status::Committed(_)));

    let two = chunked(&[&chunk, &chunk]);
    let (mut layer, mut shell) = session("chunks");
    let admitted = command_preview(&mut layer, &mut shell, &two);
    let preview = present(&mut layer, &mut shell, &admitted);
    assert_eq!(preview.recipe.steps().len(), 2);
    let operation = whole(&preview);
    let committed = commit(&mut layer, &mut shell);
    assert!(matches!(committed.status, Status::Committed(_)), "{committed:?}");
    assert_eq!(layer.store().log.len(), before.3 + 1, "two full chunks commit as one transaction");
    let Expansion::Literal(literal) = expand(&kernel, &two) else { panic!("two full chunks expand") };
    assert_eq!(literal.len(), 2 * WORD_LIMIT);
    assert_eq!(operation.0, literal_digest(&literal));
    let steps = word_steps(&two).and_then(|lengths| Steps::new(&literal, lengths)).expect("the solver's word steps");
    assert!(proposal_args(&literal, Some(&steps)) == draft_args(&two), "the solver's two word steps, unchanged");
    for host in HOSTS {
        let (mut layer, mut shell) = session("chunks");
        let (ran, sent) = run_with(on(host, &mut process), &tool, &mut layer, &literal, "tool", Some(steps.clone()));
        assert_eq!(ran.unwrap().run.summary.outcome, Outcome::Completed, "{host}");
        let [one] = &sent[..] else { panic!("{host}: one invocation, not {}", sent.len()) };
        assert!(one.invocation.args == draft_args(&two), "{host}: the solver's word steps");
        let preview = present(&mut layer, &mut shell, &one.receipt);
        assert_eq!(preview.recipe.steps().len(), 2, "{host}");
        assert_eq!(whole(&preview), operation, "{host}: the command route's operation");
        let committed = commit(&mut layer, &mut shell);
        let Status::Committed(record) = &committed.status else { panic!("{host}: {committed:?}") };
        assert_eq!(Some(&record.post), operation.4.as_ref(), "{host}");
        assert_eq!(layer.store().log.len(), before.3 + 1, "{host}: one transaction");
    }
}

/// CT-LB2-03, no silent split on the source route (LB2.3: a program is never split silently; LB2.4: `core.py`'s
/// historical suggestion to split a batch is no automatic split; LB1.11: chunking transports one operation). The
/// bridge sends the solver's word steps only for a proposal of exactly the literal word of the input recipe the host
/// gave it, letter for letter. A built-in tool proposes only the word the solver named (`tool.rs`,
/// `a_built_in_previews_only_the_word_the_solver_named`), so on this route a word of the program's own is a run word
/// that is not the input recipe's literal, as `--tamper` makes one.
///
/// The bridge is given the word steps of a recipe of two full chunks, and the tool runs on its literal word with the
/// first letter negated: the same length, so the solver's step lengths would fit it. On either host the one
/// invocation carries one unsplit word step of 200,000 letters and is refused whole with the kernel's recipe-shape
/// code and detail, the command route's answer to the same draft: no job, no preview, no change. Without the
/// solver's word steps, as for a recipe with a star step or a word step with another field (`word_steps` gives none
/// for either), the bridge does not cut even the literal word itself, which two full chunks would carry: it is
/// refused whole the same way.
#[test]
fn ct_lb2_03_a_programs_own_word_over_one_chunk_is_refused_whole_on_the_source_route() {
    let tool = Tool::full_600_cell().unwrap();
    let mut process = process_host();
    let chunk = narrow(&letters(WORD_LIMIT));
    let literal = [&chunk[..], &chunk[..]].concat();
    let steps = Steps::new(&literal, vec![WORD_LIMIT; 2]).expect("two full chunks");
    let mut own = literal.clone();
    own[0] = -own[0];
    assert!(steps.cut(&own).is_none(), "a word of the same length that is not the literal is not cut");
    // The host gives word steps only for a recipe of plain word steps: a star step or a word step with another field
    // leaves the bridge none.
    assert_eq!(word_steps(&chunked(&[&[1], &[2, 3]])), Some(vec![1, 2]));
    let star = json::parse(br#"[{"kind": "word", "moves": [1]}, {"kind": "star", "orbit": 0, "node": 0}]"#);
    assert_eq!(word_steps(&star.expect("a recipe")), None, "a recipe with a star step");
    let noted = json::parse(br#"[{"kind": "word", "moves": [1], "note": "n"}]"#);
    assert_eq!(word_steps(&noted.expect("a recipe")), None, "a word step with another field");
    for (word, given) in [(&own, Some(steps)), (&literal, None)] {
        let single = chunked(&[&word.iter().map(|&letter| i64::from(letter)).collect::<Vec<_>>()]);
        assert!(proposal_args(word, given.as_ref()) == draft_args(&single), "one word step");
        let (mut layer, mut shell) = session("own");
        let before = fingerprint(&layer);
        let command = command_preview(&mut layer, &mut shell, &single);
        assert_eq!(command.status.code(), Some(RECIPE_SHAPE), "{command:?}");
        for host in HOSTS {
            let (mut layer, _) = session("own");
            let (ran, sent) = run_with(on(host, &mut process), &tool, &mut layer, word, "tool", given.clone());
            assert_eq!(ran.unwrap().run.summary.outcome, Outcome::Completed, "{host}");
            let [one] = &sent[..] else { panic!("{host}: one invocation, not {}", sent.len()) };
            assert!(one.invocation.args == draft_args(&single), "{host}: one word step");
            assert_eq!(one.receipt.status, command.status, "{host}: refused whole, as on the command route");
            assert_eq!(one.receipt.job, None, "{host}");
            assert!(layer.preview().is_none() && layer.run_jobs() == 0, "{host}");
            assert_eq!(fingerprint(&layer), before, "{host}");
        }
    }
}

/// What makes two previews the same operation: the literal, expansion and net digests, the count and the end state.
fn whole(preview: &Preview) -> (String, String, String, usize, Option<String>) {
    let effect = &preview.effect;
    let end = preview.end.as_ref().map(|state| state.digest().to_owned());
    (preview.literal.clone(), effect.expansion_sha256.clone(), effect.net_sha256.clone(), effect.primitive_count, end)
}
