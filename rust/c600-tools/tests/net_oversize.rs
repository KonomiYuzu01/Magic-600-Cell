//! CT-LB2-02's Net half and CT-LB2-03's oversize and chunking parts through the 600-cell-Full tool's route (LP3,
//! C5's next piece, acceptance items 3 and 4). Each test runs the tool on the in-process host and on the process
//! host. Neither ID runs whole here: each test's comment names the part it covers and the part that is not built.

mod process_support;

use c600_command::{code, draft_args, CommandLayer, Error, Preview, Receipt, Shell, Status};
use c600_kernel::codes::{PRIMITIVE_CEILING, RECIPE_SHAPE};
use c600_kernel::json::Value as Json;
use c600_kernel::{Level, ProtectedSet, ProtectionMode, Verdict};
use c600_run::meter::Outcome;
use c600_tools::bridge::word_args;
use c600_tools::expand::{expand, literal_digest, Expansion};
use c600_tools::process::ProcessHost;
use c600_tools::tool::{Tool, ROW};
use process_support::{chunked, fingerprint, kernel, on, process_host, run_on, session, session_with, HOSTS};

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

/// Runs the admitted preview job and presents it.
fn present(layer: &mut CommandLayer, shell: &mut Shell, admitted: &Receipt) -> Preview {
    let Status::Running(id) = admitted.status else { panic!("not admitted: {admitted:?}") };
    assert_eq!(layer.run_jobs(), 1);
    shell.present(layer, id).expect("the built preview is presented");
    layer.preview().filter(|preview| preview.id == id).cloned().expect("the presented preview")
}

fn commit(layer: &mut CommandLayer, shell: &mut Shell) -> Receipt {
    shell.press(layer, "Ctrl+Shift+Enter", false).expect("the chord is sent").expect("not a repeat")
}

/// CT-LB2-02, the Net half (LB2.3: Net tests the end of the complete operation). With the orbits a generator `g`
/// moves protected, the words `g -g` and `g h -h -g` have identity net effect while every proper prefix moves a
/// protected orbit. Through the tool's route, on either host, the whole word previews with no sticker moved, a
/// protection row at level labels in mode Net with verdict clear and no conflict, and an end state equal to its
/// base; the shell's chord commits it as one transaction that leaves the state where it was. The moving prefix `g`
/// alone previews with verdict moves and no end state, and its commit is refused with the kernel's protection code.
///
/// Not built through the tool's route: the Strict half and "an incomplete prefix check never returns safe", because
/// the command layer's preview has no Strict mode (KN5, LP2). `c600-library/tests/ct_lb2_02.rs` runs both at the
/// kernel, on words the library builds.
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
/// tool's proposal of it previews with the same literal, effect and end state.
///
/// Not built: chunked transport on the source route. The tool's entry takes one flat word and the bridge sends one
/// word step, so a valid recipe of two full chunks (200,000 letters), which the command route admits, is refused
/// whole on the source route with the recipe-shape code; it is never split. The last assertion pins that gap.
/// "Named work-sheet reuse invalidates the old preview" is not built either: this route has no named work sheet.
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
    assert!(matches!(command_preview(&mut layer, &mut shell, &two).status, Status::Running(_)));
    let Expansion::Literal(literal) = expand(&kernel, &two) else { panic!("two full chunks expand") };
    assert_eq!(literal.len(), 2 * WORD_LIMIT);
    for host in HOSTS {
        let (mut layer, _) = session("chunks");
        let on = on(host, &mut process);
        let whole = tool_preview(on, &tool, &mut layer, &literal);
        assert_eq!(whole.status.code(), Some(RECIPE_SHAPE), "{host}: refused whole, not split: {:?}", whole.status);
        assert!(layer.preview().is_none() && layer.run_jobs() == 0, "{host}");
    }
}

/// What makes two previews the same operation: the literal, expansion and net digests, the count and the end state.
fn whole(preview: &Preview) -> (String, String, String, usize, Option<String>) {
    let effect = &preview.effect;
    let end = preview.end.as_ref().map(|state| state.digest().to_owned());
    (preview.literal.clone(), effect.expansion_sha256.clone(), effect.net_sha256.clone(), effect.primitive_count, end)
}
