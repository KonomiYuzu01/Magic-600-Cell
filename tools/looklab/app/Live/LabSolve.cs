using System;
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
    public AnchorLayer? GlobalAnchors { get; private set; }
    public AnchorLayer? LocalAnchors { get; private set; }
    private readonly ChromeFrame frame;

    public static bool Handles(string command) => command is "target.current.set" or "target.next.set" or "target.next.activate" or "target.next.clear" or
        "buffer.assign" or "target.frame.set" or "orbit.switch";

    public LabSolve(ChromeFrame frame, string root, Func<string, double> seconds, Action<string, string?> run)
    {
        this.frame = frame;
        Demo = new SolvingDemo(OrbitAtlas.Parse(File.ReadAllText(Path.Combine(root, "assets/seed_atlas.json"))));
        Strip = new RelationStrip(Demo, () => frame.Tokens, seconds);
        Strip.Command += run; Strip.DemoNext += Demo.NextState;
        frame.SetSeam(Strip); frame.HostKeys["F11"] = Demo.NextState;
        frame.Rebuilt += Attach; frame.Recoloured += Strip.Recolour;
    }

    // The view borders are built again with the frame, so the anchors are laid over the new ones.
    private void Attach()
    {
        Strip.Recolour();
        GlobalAnchors = new AnchorLayer(Strip, () => frame.Tokens, true, () => frame.PairSideBySide ? new Vector2(1, .5f) : new Vector2(.5f, 1));
        LocalAnchors = new AnchorLayer(Strip, () => frame.Tokens, false, () => frame.PairSideBySide ? new Vector2(0, .5f) : new Vector2(.5f, 0));
        frame.Global.Slot.AddChild(GlobalAnchors); frame.Local.Slot.AddChild(LocalAnchors);
    }

    public void Execute(string command, string? argument, string label)
    {
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
