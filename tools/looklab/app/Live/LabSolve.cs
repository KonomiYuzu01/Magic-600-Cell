using System;
using System.Collections.Generic;
using System.Linq;
using System.IO;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The solving surface in the lab: the relationship strip on the pair's seam and its anchors in the two views, on
// demo states. The orbit facts are read from assets/seed_atlas.json; every other number is demo data.
internal sealed class LabSolve
{
    public SolvingDemo Demo { get; }
    public RelationStrip Strip { get; }
    public OrbitRail Rail { get; }
    public OperationDraft Draft { get; } = new();
    public DraftLine Line { get; }
    private readonly LabDemo session; private int sample; private bool staged;
    public AnchorLayer? GlobalAnchors { get; private set; }
    public AnchorLayer? LocalAnchors { get; private set; }
    private readonly ChromeFrame frame;

    public static bool Handles(string command) => command is "target.current.set" or "target.next.set" or "target.next.activate" or "target.next.clear" or
        "buffer.assign" or "target.frame.set" or "orbit.switch" or "input.destination.set" or "intent.set" or "operation.phase.select" or "operation.phase.edit" or "operation.phase.append_twist";

    public LabSolve(ChromeFrame frame, string root, Func<string, double> seconds, Action<string, string?> run, LabDemo session)
    {
        this.frame = frame; this.session = session;
        Line = new DraftLine(Draft, () => frame.Tokens); Line.Command += run;
        // The draft is staged as the preview whenever it changes; the host's commit bar commits it. A commit or a
        // discard of that preview empties the draft.
        Draft.Changed += Stage;
        session.Preview.Changed += () => { if (staged && session.Preview.Pending == null) { staged = false; Draft.Clear(); } };
        session.Named = preview => preview.Source == "draft" ? ("hand", $"operation ({preview.Moves.Count} twists)") : null;
        // A parameter of a staged draft belongs to one of its twists: the draft changes and stages itself again.
        session.Reparam = (preview, index, value) =>
        {
            if (preview.Source != "draft") return false;
            for (int phase = 0, seen = 0; phase < OperationDraft.Phases.Count; phase++)
                for (int i = 0; i < Draft.Twists(phase).Count; i++)
                {
                    string twist = Draft.Twists(phase)[i]; IReadOnlyList<StatementParameter> own = InlineParameters.Find(twist);
                    if (index >= seen + own.Count) { seen += own.Count; continue; }
                    string[] twists = Draft.Twists(phase).ToArray(); twists[i] = InlineParameters.With(twist, own[index - seen], value);
                    Draft.Edit(OperationDraft.Phases[phase] + ": " + string.Join("; ", twists)); return true;
                }
            return true;
        };
        Draft.HeldCleared += () => { Rail!.Jump.Release(); Rail.Refresh(); };
        Draft.Changed += () => { LocalAnchors?.QueueRedraw(); };
        Demo = new SolvingDemo(OrbitAtlas.Parse(File.ReadAllText(Path.Combine(root, "assets/seed_atlas.json"))));
        Strip = new RelationStrip(Demo, () => frame.Tokens, seconds);
        Strip.Command += run; Strip.DemoNext += Demo.NextState;
        Rail = new OrbitRail(Demo, () => frame.Tokens); Rail.Command += run;
        frame.SetSeam(Strip); frame.SetPairHead(Rail); frame.HostKeys["F11"] = Demo.NextState;
        frame.Rebuilt += Attach; frame.Recoloured += () => { Strip.Recolour(); Rail.Recolour(); };
    }

    // The view borders are built again with the frame, so the anchors are laid over the new ones.
    private void Attach()
    {
        Strip.Recolour(); Rail.Recolour();
        GlobalAnchors = new AnchorLayer(Strip, () => frame.Tokens, true, () => frame.PairSideBySide ? new Vector2(1, .5f) : new Vector2(.5f, 1));
        LocalAnchors = new AnchorLayer(Strip, () => frame.Tokens, false, () => frame.PairSideBySide ? new Vector2(0, .5f) : new Vector2(.5f, 0));
        LocalAnchors.GripDraft = () => Draft.Route == InputRoute.Draft;
        frame.Global.Slot.AddChild(GlobalAnchors); frame.Local.Slot.AddChild(LocalAnchors); Line.Refresh();
    }

    private static readonly string[] SampleTwists = { "twist(cell 5, axis 0, +1)", "twist(cell 40, axis 2, -1)", "twist(cell 77, axis 3, +1)", "twist(cell 117, axis 1, -1)" };

    private void Stage()
    {
        if (Draft.CanCommit) { staged = true; session.StageDraft(Draft.Statement, Draft.Word); Line.Settle(PreviewModel.Settle); }
        else if (staged && session.Preview.Pending is { Source: "draft" }) { staged = false; session.Preview.Discard(); }
    }

    // The bar under the views: the draft line over the host's commit bar.
    public Control Bars(Control commitBar)
    {
        var both = new VBoxContainer { Name = "DraftAndCommit" }; both.AddThemeConstantOverride("separation", 0);
        both.AddChild(Line); both.AddChild(commitBar); return both;
    }

    private bool Drafting(string command, string? argument, string label)
    {
        string? why = null; string said = label;
        switch (command)
        {
            case "input.destination.set": why = Draft.SetRoute(argument); said = Draft.Route == InputRoute.Draft ? "Draft: twists join the active phase" : "Direct: a twist is one complete operation"; break;
            case "operation.phase.select": why = Draft.SelectPhase(argument); said = "Active phase: " + OperationDraft.Phases[Draft.Active]; break;
            case "operation.phase.edit": why = Draft.Edit(argument); said = $"{Draft.Count} twists in the draft"; break;
            case "intent.set": Draft.SetGoal(argument); said = Draft.Goal == null ? "Goal: unset" : $"Goal: {Draft.Goal} ({SolvingDemo.Mark})"; break;
            case "operation.phase.append_twist":
                string twist = argument ?? SampleTwists[sample++ % SampleTwists.Length];
                if (Draft.Route == InputRoute.Direct) { session.CommitDirect(twist); return true; }
                Draft.Append(twist); said = $"{OperationDraft.Phases[Draft.Active]}: {twist}"; break;
            default: return false;
        }
        frame.Say(why != null ? $"{label}: {why}" : said, why != null); return true;
    }

    public void Execute(string command, string? argument, string label)
    {
        if (Drafting(command, argument, label)) return;
        string? why = command switch
        {
            "target.current.set" => Demo.SetTarget(argument),
            "buffer.assign" => Demo.AssignBuffer(argument),
            "target.frame.set" => Demo.SetFrame(argument),
            "orbit.switch" => Demo.SwitchOrbit(argument),
            "target.next.set" => Demo.SetNext(argument),
            "target.next.clear" => Demo.ClearNext(),
            _ => Demo.ActivateNext(),
        };
        if (why != null) { frame.Say($"{label}: {why}", true); return; }
        // A fact, never an instruction.
        SolvingState s = Demo.State; var bad = s.Problems();
        frame.Say(bad.Count > 0 ? "invalid: " + bad[0].Text : s.Wanted != null && s.WantedPlace().Length > 0 ? "Wanted is " + s.WantedPlace() : label, bad.Count > 0);
    }
}
