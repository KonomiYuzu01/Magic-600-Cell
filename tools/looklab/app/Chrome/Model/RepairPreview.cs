using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

public sealed record RepairPreviewFacts(
    ulong Id, string AfterDigest, IReadOnlyList<int> MovedPerStep, IReadOnlyList<OrbitEffect> Effects,
    string Protection, int Moved, string? Refusal = null);

// The product's staged word is the source of truth. Only a word edit rebuilds its preview.
public sealed class RepairPreview
{
    private readonly StagedWord word;
    private readonly BoundaryLedger ledger;
    private readonly Func<double> now;
    private readonly Func<IReadOnlyList<Letter>, RepairPreviewFacts> build;
    public PreviewModel Model { get; }
    public RepairPreviewFacts? Facts { get; private set; }
    public bool Overlay { get; private set; }

    public RepairPreview(RepairStage stage, BoundaryLedger ledger, PreviewModel model, Func<double> now,
        Func<IReadOnlyList<Letter>, RepairPreviewFacts> build)
    {
        word = stage.Word; this.ledger = ledger; Model = model; this.now = now; this.build = build;
        word.Changed += Rebuild;
        Model.SetSide(PreviewSide.After);
    }

    private void Rebuild()
    {
        if (word.Letters.Count == 0)
        {
            Facts = null;
            Model.Discard();
            return;
        }
        string[] moves = word.Letters.Select(letter => letter.ToString()).ToArray();
        var facts = Facts = build(word.Letters);
        Model.Show(Origin.Solver, word.Text(), "Repair list", moves,
            facts.Refusal == null ? PreviewStatus.Ready : PreviewStatus.Refused, facts.Refusal,
            facts.Effects, facts.Protection, now(), wholeOnly: true,
            movedPerStep: facts.MovedPerStep, movedTotal: facts.Refusal == null ? facts.Moved : null);
    }

    public void Cancel() => word.Clear();
    public void ToggleOverlay() => Overlay = !Overlay;

    public void Scrub(string? argument, UserEvent? asked)
    {
        if (Model.Pending is not { Status: PreviewStatus.Ready } pending) return;
        int step = argument switch
        {
            "back" => Model.Scrub - 1,
            "forward" => Model.Scrub + 1,
            "start" => 0,
            "end" => pending.Moves.Count,
            _ => int.TryParse(argument, NumberStyles.None, CultureInfo.InvariantCulture, out int position) ? position : Model.Scrub,
        };
        if (Math.Clamp(step, 0, pending.Moves.Count) != Model.Scrub && ledger.Record("scrub", "repair preview", asked))
            Model.SetScrub(step);
    }

    public string? CommitRefusal(UserEvent? asked)
    {
        Model.Follow();
        string? refusal = Model.CommitRefusal(now());
        if (refusal != null) return refusal;
        return ledger.Record("apply", "whole repair", asked) ? null : "Commit needs the solver's action.";
    }
}
