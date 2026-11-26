using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The state after a played part, from the samples: N, whether it is on the lattice, and its ν row there.
public readonly record struct WordState(long N, bool OnLattice, IReadOnlyList<int>? Nu);

// ADR-D02 build step B25 (ADR-D02.29, .31): a stored word, folded in word order with expanded lengths, played by part
// and twist by twist inside an opened part. The commit bar's line gives its exact length and alphabet; the signature,
// the log tab's mark and the crossing sentence follow the state the playback stands at. Right steps forward, Left
// back, Enter opens the next part.
public sealed partial class WordPanel : VBoxContainer, IOwnKeys
{
    public const string PanelId = "jumble.word";
    public WordPlayback? Playback { get; private set; }
    private Func<int, WordState?> stateAfter = _ => null;
    private readonly VBoxContainer tree = new() { Name = "WordTree" };
    private readonly Label length = ChromeWidgets.Text("", "", true), counter = ChromeWidgets.Text("", "", true), state = ChromeWidgets.Text("", "MicroLabel", true),
        signature = ChromeWidgets.Text("", "MicroLabel", true), log = ChromeWidgets.Text("", "MicroLabel", true), crossing = ChromeWidgets.Text("", "", true);

    public WordPanel()
    {
        Name = "WordPanel"; FocusMode = FocusModeEnum.All;
        length.Name = "WordLength"; counter.Name = "WordCounter"; state.Name = "WordState"; signature.Name = "WordSignature"; log.Name = "WordLog"; crossing.Name = "WordCrossing";
        AddChild(length); AddChild(tree);
        var keys = new HBoxContainer { Name = "WordKeys" };
        var back = new OwnedButton("WordBack", "◀", "Back one part, or one twist of the opened part (Left)");
        back.Pressed += () => Playback?.Back();
        var open = new OwnedButton("WordOpen", "Open part", "Open the next part to step it twist by twist (Enter)");
        open.Pressed += () => Playback?.Open();
        var forward = new OwnedButton("WordForward", "▶", "Forward one part, or one twist of the opened part (Right)");
        forward.Pressed += () => Playback?.Forward();
        keys.AddChild(back); keys.AddChild(open); keys.AddChild(forward); keys.AddChild(counter);
        AddChild(keys);
        foreach (Label l in new[] { state, signature, log, crossing }) AddChild(l);
        TreeEntered += () => { if (Playback != null) Playback.Changed += Refresh; Refresh(); };
        TreeExiting += () => { if (Playback != null) Playback.Changed -= Refresh; };
    }

    public void Bind(WordTree? word, Func<int, WordState?> after)
    {
        if (Playback != null) Playback.Changed -= Refresh;
        Playback = word == null ? null : new WordPlayback(word);
        stateAfter = after;
        if (Playback != null && IsInsideTree()) Playback.Changed += Refresh;
        Refresh();
    }

    public override void _GuiInput(InputEvent e)
    {
        if (e is InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: true }) { GrabFocus(); return; }
        if (Playback == null || e is not InputEventKey { Pressed: true } key) return;
        switch (key.Keycode)
        {
            case Key.Right: Playback.Forward(); break;
            case Key.Left: Playback.Back(); break;
            case Key.Enter or Key.KpEnter when !key.Echo: Playback.Open(); break;
            default: return;
        }
        AcceptEvent();
    }

    // The panel as text, for the shell's dump: the lines a screen reader would read.
    public IReadOnlyDictionary<string, string> Lines() => new Dictionary<string, string>
    {
        ["length"] = length.Text, ["counter"] = counter.Text, ["state"] = state.Text, ["signature"] = signature.Text, ["log"] = log.Text,
        ["crossing"] = crossing.Text, ["digest"] = Playback?.Digest ?? "",
    };

    private void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        foreach (Node child in tree.GetChildren()) { tree.RemoveChild(child); child.QueueFree(); }
        if (Playback is not { } play)
        {
            length.Text = "No stored word"; counter.Text = state.Text = signature.Text = log.Text = crossing.Text = "";
            return;
        }
        WordTree word = play.Tree;
        length.Text = JumbleWords.LengthLine(word.ExpandedLength, word.Alphabet);
        string[] folded = word.Folded().ToArray();
        for (int i = 0; i < folded.Length; i++)
        {
            bool current = i > 0 && (play.Opened ? i == play.Done + 1 : i == play.Done);
            tree.AddChild(ChromeWidgets.Text((current ? "▸ " : "  ") + folded[i].TrimStart(), i == 0 ? "" : "MicroLabel", true));
        }
        counter.Text = play.Counter() + (play.ShowsLetters ? "" : " · numerals only (" + JumbleWords.LongWordBoundLabel + ")");
        WordState? at = stateAfter(play.Done);
        WordState? start = stateAfter(0), end = stateAfter(play.Parts);
        string letters = play.ShowsLetters && play.Opened && play.Shown() is { Count: > 0 } shown ? " · " + string.Join(" ", shown) : "";
        state.Text = (at is { } s ? "N " + s.N.ToString("N0", CultureInfo.InvariantCulture) : "N pending") + " · state " + play.Digest[..Math.Min(12, play.Digest.Length)] + letters;
        signature.Text = WordSignature.Of(start?.OnLattice ?? false, end?.OnLattice ?? false, null, null, word.ExpandedLength, word.Alphabet).Line();
        log.Text = at is { } l ? "Log tab: " + LogTabMark.Of(l.N, l.OnLattice ? l.Nu : null).Line() : "";
        // The crossing sentence, with the reachability that makes it true shown beside it.
        bool crosses = at is { OnLattice: true, Nu: { } nu } && nu.All(v => v == 0) && at.Value.N == 0;
        crossing.Text = JumbleWords.CrossingSentence(crosses, crosses) is { } sentence ? sentence + " (reachable: N = 0 and ν = 0 here)" : "";
    }
}
