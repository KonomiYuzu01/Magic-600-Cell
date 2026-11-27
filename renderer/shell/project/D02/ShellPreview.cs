using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02
{
    /// <summary>The product's repair preview (ADR-D01.10), bound to the staged word and the host's copied poses.</summary>
    internal sealed class ShellPreview
    {
        internal const string Pane = "preview", CommitRow = "operation.preview.commit", CancelRow = "operation.preview.cancel",
            ScrubRow = "view.preview.scrub", OpenRow = "view.preview.open", OverlayRow = "preview.overlay.script.toggle";
        private const string TwistRow = "jumble.twist.menu";
        private const string SidePrefix = "preview.side.";
        private static ShellPreview? active;

        private readonly ChromeFrame frame;
        private readonly HostLibrary library;
        private readonly IntPtr host;
        private readonly PreviewPanel panel;
        private readonly CommitBar bar;
        private readonly RepairPreview preview;
        private readonly Func<(double Seconds, double[] Curve)> motion;
        private readonly Action<string, ulong> committed;
        private readonly List<Control> marks = new();
        private string[] digests = Array.Empty<string>();
        private bool[] sweeps = Array.Empty<bool>();
        private int shownId, requested;
        private (int Prefix, float Phase) sent;
        private (int Prefix, bool AtEnd, double Started)? turn;
        private PreviewSide comparison = PreviewSide.After;
        private byte[]? moved;
        private byte[]? uncheckedPrefix;
        private byte[]? overlay => preview.Overlay && Model.Pending is { Status: PreviewStatus.Ready } ? moved : null;
        private CommitResult? lastCommit;
        private sealed record CommitResult(int Letters, string Expected, string Actual, string? Error);

        private PreviewModel Model => preview.Model;
        internal bool Pending => Model.Pending != null;
        internal static byte[]? Overlay => active?.overlay;
        internal static byte[]? MovingAt(ulong revision) =>
            active is { } current && current.Model.Pending is { Status: PreviewStatus.Ready } pending &&
            (ulong)pending.Head == revision ? current.moved : null;
        internal static StatusPatterns.PreviewFeed StatusAt(ulong revision) =>
            active is { } current
                ? StatusPatterns.RepairFeed(current.Model.Pending, (long)revision, current.moved, current.uncheckedPrefix) : default;
        internal static Rgb? OverlayColour => active?.frame.Tokens.Colour("pattern.preview");
        internal static Rgb? PreviewColour => active is { Pending: true } current ? current.frame.Tokens.Colour("state.preview") : null;
        private static double Now() => Stopwatch.GetTimestamp() / (double)Stopwatch.Frequency;

        internal ShellPreview(ChromeFrame frame, JumbleHost jumble, CommitBar bar, HostLibrary library, IntPtr host,
            Func<ulong> revision, Func<(double, double[])> motion, Action<string, ulong> committed, Action<string, string?> execute)
        {
            this.frame = frame; this.bar = bar; this.library = library; this.host = host;
            this.motion = motion; this.committed = committed;
            bar.Model.Head = () => (int)revision();
            bar.Clock = Now;
            preview = new RepairPreview(jumble.Stage, jumble.Ledger, bar.Model, Now, Build);
            panel = new PreviewPanel(Model, () => frame.Tokens, drawn: true);
            panel.Command += execute;
            Model.Changed += Changed;
            frame.Model.Changed += () =>
            {
                if (Pending && !frame.Model.IsOpen(Pane)) preview.Cancel();
            };
            frame.Rebuilt += Dress;
            frame.Recoloured += () => { Dress(); ShellStyle.Invalidate(); };
            var contexts = frame.Contexts;
            frame.Contexts = () => Pending ? new[] { "preview" }.Concat(contexts?.Invoke() ?? Array.Empty<string>()).ToArray()
                : contexts?.Invoke() ?? Array.Empty<string>();
            active = this;
            frame.TreeExiting += () => { if (active == this) active = null; };
        }

        internal static IReadOnlyList<string> AddRows(CommandRegistry registry, string folder)
        {
            var draft = new CommandRegistry();
            draft.AddRows(File.ReadAllText(Path.Combine(folder, "command-rows-draft.json")));
            var ids = draft.Order.Where(id => id.StartsWith(SidePrefix, StringComparison.Ordinal)).ToArray();
            foreach (string id in ids) registry.Add(draft.Commands[id]);
            using var d02 = JsonDocument.Parse(File.ReadAllText(Path.Combine(folder, "command-rows-d02.json")));
            registry.AddRows("{\"commands\":" + d02.RootElement.GetProperty("preview").GetRawText() + "}");
            return [.. ids, OverlayRow];
        }

        internal static bool Handles(string command) => command is CommitRow or CancelRow or ScrubRow or OpenRow or OverlayRow
            || command.StartsWith(SidePrefix, StringComparison.Ordinal);

        internal string? Unavailable(string command) => command switch
        {
            OverlayRow => null,
            CommitRow => Model.CommitRefusal(Now()),
            _ => Pending ? null : "No preview is staged.",
        };

        internal void Execute(string command, string? argument, UserEvent? asked)
        {
            switch (command)
            {
                case CommitRow: Commit(asked); break;
                case CancelRow: preview.Cancel(); break;
                case ScrubRow when asked != null: preview.Scrub(argument, asked); break;
                case OpenRow when Pending: Open(); break;
                case "preview.side.both": Model.SetSide(PreviewSide.Both); break;
                case "preview.side.before": Model.SetSide(PreviewSide.Before); break;
                case "preview.side.after": Model.SetSide(PreviewSide.After); break;
                case "preview.side.next": Model.CycleSide(); break;
                case OverlayRow: preview.ToggleOverlay(); SetOverlay(false); Dress(); break;
            }
        }

        private RepairPreviewFacts Build(IReadOnlyList<Letter> letters)
        {
            lastCommit = null;
            beforeDigest = "";
            digests = Array.Empty<string>(); sweeps = Array.Empty<bool>();
            int code = library.BuildPreview(host, letters);
            if (code != HostLibrary.Ok)
                return new(0, "", Array.Empty<int>(), Array.Empty<OrbitEffect>(), "Protection follows the preview check.", 0, library.LastError(host));
            using var answer = JsonDocument.Parse(library.Answer(host));
            var root = answer.RootElement;
            beforeDigest = root.GetProperty("before").GetString()!;
            var steps = root.GetProperty("steps").EnumerateArray().ToArray();
            digests = steps.Select(step => step.GetProperty("digest").GetString()!).ToArray();
            sweeps = steps.Select(step => step.GetProperty("sweep").GetBoolean()).ToArray();
            var effects = root.GetProperty("orbits").EnumerateArray().Select(orbit =>
            {
                int id = orbit.GetProperty("orbit").GetInt32();
                var name = id < 0 ? "Centres" : $"Orbit {id}";
                return new OrbitEffect(name, orbit.GetProperty("moved").GetInt32(), orbit.GetProperty("total").GetInt32())
                    { Net = orbit.GetProperty("net").GetInt32() };
            }).ToArray();
            string protection = root.GetProperty("protection").GetString()!;
            if (sweeps.Contains(false)) protection += " · A sweep waits for the J03 plane.";
            return new(root.GetProperty("id").GetUInt64(), root.GetProperty("after").GetString()!,
                steps.Select(step => step.GetProperty("moved").GetInt32()).ToArray(), effects, protection, root.GetProperty("moved").GetInt32());
        }

        private void Open()
        {
            if (!frame.Model.IsOpen(Pane)) frame.Model.Drop(Pane, FrameModel.Pair, DropZone.Right);
            else frame.Model.Activate(Pane);
            if (frame.Model.OnlyPairShown)
            {
                frame.Model.SetFocusMode(false);
                frame.Model.SetPairMaximised(false);
            }
        }

        private void Changed()
        {
            var pending = Model.Pending;
            if (pending == null)
            {
                shownId = 0; turn = null;
                moved = uncheckedPrefix = null;
                library.ClearPreview(host);
                frame.RemovePane(Pane);
                SetOverlay(); Dress();
                return;
            }
            if (shownId != pending.Id)
            {
                shownId = pending.Id; requested = Model.Scrub; sent = (requested, 0); turn = null;
                moved = uncheckedPrefix = null;
                if (pending.Status == PreviewStatus.Ready && preview.Facts is { } facts)
                {
                    var mask = new byte[ShellStyle.PieceCount];
                    if (library.PreviewMoved(host, facts.Id, mask) == HostLibrary.Ok) moved = mask;
                }
                if (frame.Model.IsOpen(Pane)) frame.Bind(Pane, ViewBinding.PreviewOf(pending.Id));
                else frame.AddPane(Pane, "Preview", panel, ViewBinding.PreviewOf(pending.Id), () => panel.DrawnOpenings().Select(p => p.Rect));
                Open();
                SetOverlay(); Dress();
                return;
            }
            if (pending.Status != PreviewStatus.Ready)
            {
                turn = null; SetOverlay(); Dress();
            }
            else if (Model.Scrub != requested)
            {
                int to = Model.Scrub, prefix = Math.Min(to, requested);
                turn = Math.Abs(to - requested) == 1 && sweeps[prefix] && comparison != PreviewSide.Before
                    && panel.IsVisibleInTree() ? (prefix, to > requested, Now()) : null;
                requested = to;
                uncheckedPrefix = null;
                if (to > 0 && to < pending.Moves.Count && preview.Facts is { } facts)
                {
                    var mask = new byte[ShellStyle.PieceCount];
                    if (library.PreviewPrefixMoved(host, facts.Id, to, mask) == HostLibrary.Ok) uncheckedPrefix = mask;
                }
            }
        }

        internal void Follow() => Model.Follow();

        internal void Tick()
        {
            bar.FollowSettle();
            if (Model.Pending is not { Status: PreviewStatus.Ready } || preview.Facts is not { } facts) return;
            var draw = (Prefix: requested, Phase: 0f);
            if (turn is { } step)
            {
                var (seconds, curve) = motion();
                double t = seconds > 0 ? (Now() - step.Started) / seconds : 1;
                if (t < 1 && comparison != PreviewSide.Before && panel.IsVisibleInTree())
                {
                    float excursion = (float)MotionTable.Ease(curve, 1 - Math.Abs(2 * t - 1));
                    draw = (step.Prefix, step.AtEnd ? 1 - excursion : excursion);
                }
                else turn = null;
            }
            if (draw == sent) return;
            if (library.ScrubPreview(host, facts.Id, draw.Prefix, draw.Phase) == HostLibrary.Ok) sent = draw;
            else Model.MarkStale(library.LastError(host));
        }

        internal IEnumerable<PaneSpec> Openings(int mainPanes)
        {
            comparison = ViewPanes.Comparison(Model.Side, mainPanes);
            panel.SetComparison(comparison, comparison == Model.Side ? null : "Showing After. The split uses three panes.");
            foreach (var (before, rect) in panel.DrawnOpenings())
            {
                var pixels = new PaneRect((int)Math.Round(rect.Position.X), (int)Math.Round(rect.Position.Y),
                    (int)Math.Floor(rect.Size.X), (int)Math.Floor(rect.Size.Y));
                if (!pixels.IsEmpty) yield return new(before ? "Preview before" : "Preview after", pixels);
            }
        }

        private void SetOverlay(bool movementChanged = true)
        {
            if (movementChanged) ShellJumble.Host?.Filter?.RefreshFields();
            ShellTint.RefreshFocus();
            ShellStyle.Invalidate();
        }

        private void Dress()
        {
            foreach (var mark in marks)
                if (GodotObject.IsInstanceValid(mark)) { ChromeWidgets.Detach(mark); mark.QueueFree(); }
            marks.Clear();
            if (!Pending) return;
            foreach (var view in new[] { frame.Global, frame.Local })
            {
                if (view == null || !GodotObject.IsInstanceValid(view)) continue;
                var chip = SessionWidgets.Chip("Preview pending", frame.Tokens.C("state.preview"));
                chip.Name = "PreviewPending";
                var border = new StyleBoxFlat { BgColor = frame.Tokens.C("panel.background"), BorderColor = frame.Tokens.C("pattern.preview") };
                border.SetBorderWidthAll(frame.Tokens.Px("border.width")); chip.AddThemeStyleboxOverride("panel", border);
                view.Tools.AddChild(chip); marks.Add(chip);
                if (overlay == null) continue;
                var script = SessionWidgets.Chip("script", frame.Tokens.C("script.frame"), "Pending preview overlay");
                script.ThemeTypeVariation = "ScriptFrame";
                view.Tools.AddChild(script); marks.Add(script);
                var edge = new Control { MouseFilter = Control.MouseFilterEnum.Ignore };
                view.Slot.AddChild(edge); edge.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);
                edge.Draw += () => edge.DrawRect(new Rect2(Vector2.Zero, edge.Size).Grow(-1), frame.Tokens.C("script.frame"), false, frame.Tokens.Px("border.width"));
                marks.Add(edge);
            }
        }

        private string HeadDigest() => ReadAnswer(library.StateDigest, root => root.GetProperty("digest").GetString()!) ?? "";

        private T? ReadAnswer<T>(Func<IntPtr, int> call, Func<JsonElement, T> read) where T : class
        {
            if (call(host) != HostLibrary.Ok) return null;
            using var answer = JsonDocument.Parse(library.Answer(host));
            return read(answer.RootElement);
        }

        private void Commit(UserEvent? asked)
        {
            if (asked == null) { frame.Say("Use the commit key or button.", true); return; }
            if (preview.CommitRefusal(asked) is { } refusal) { frame.Say(refusal, true); return; }
            var facts = preview.Facts!;
            var pending = Model.Pending!;
            int done = 0;
            string? error = null;
            while (done < pending.Moves.Count)
            {
                int code = library.PreviewArgs(host, facts.Id, done);
                if (code != HostLibrary.Ok)
                {
                    error = library.LastError(host);
                    if (done == 0) { Model.MarkStale(error); frame.Say(error, true); return; }
                    break;
                }
                string args = library.Answer(host);
                code = library.Invoke(host, TwistRow, args, out _);
                if (code != HostLibrary.Ok) { error = library.LastError(host); break; }
                using var receipt = JsonDocument.Parse(library.Answer(host));
                JsonElement status = receipt.RootElement.GetProperty("status");
                if (status.GetProperty("name").GetString() != "committed")
                {
                    error = status.TryGetProperty("error", out var why) && why.TryGetProperty("detail", out var detail)
                        ? detail.GetString() : "The twist did not commit.";
                    break;
                }
                committed(pending.Moves[done], (ulong)pending.Head + (ulong)done + 1);
                done++;
            }
            string actual = HeadDigest();
            if (done != pending.Moves.Count) error ??= "The repair stopped before all twists committed.";
            else if (actual != facts.AfterDigest) error = "The committed digest differs from the preview.";
            lastCommit = new(done, facts.AfterDigest, actual, error);
            if (error != null)
            {
                ChromeErrors.Log.Add($"Repair preview #{pending.Id}: {error}");
                frame.Say(error, true);
                if (done == 0 && actual == beforeDigest)
                {
                    Model.MarkStale(error);
                    return;
                }
            }
            preview.Cancel();
        }

        private string beforeDigest = "";

        internal static void Dump(Utf8JsonWriter json)
        {
            if (active is not { } current) return;
            json.WriteStartObject("preview");
            var pending = current.Model.Pending;
            json.WriteBoolean("pane", current.frame.Model.IsOpen(Pane));
            json.WriteBoolean("overlay", current.overlay != null);
            json.WriteString("side", current.comparison.ToString());
            json.WriteNumber("scrub", current.Model.Scrub);
            if (pending != null)
            {
                json.WriteNumber("id", pending.Id);
                json.WriteString("status", pending.Status.ToString().ToLowerInvariant());
                json.WriteString("reason", current.Model.CommitRefusal(Now()));
                // ADR-D01.9/.66: the settle interval and what the commit bar shows of it.
                json.WriteBoolean("settling", current.Model.Settling(Now()));
                json.WriteStartObject("bar");
                json.WriteString("status", GodotObject.IsInstanceValid(current.bar.State) ? current.bar.State.Text : "");
                json.WriteBoolean("lamp", current.bar.Settling);
                json.WriteBoolean("commit", GodotObject.IsInstanceValid(current.bar.Commit) && !current.bar.Commit.Disabled);
                json.WriteString("commit_tip", GodotObject.IsInstanceValid(current.bar.Commit) ? current.bar.Commit.TooltipText : "");
                json.WriteEndObject();
                json.WriteBoolean("whole_only", pending.WholeOnly);
                json.WriteNumber("letters", pending.Moves.Count);
                if (current.preview.Facts is { } facts)
                {
                    json.WriteNumber("host_id", facts.Id);
                    json.WriteString("after", facts.AfterDigest);
                    json.WriteNumber("moved", facts.Moved);
                    json.WriteStartArray("prefix_digests");
                    foreach (string digest in current.digests) json.WriteStringValue(digest);
                    json.WriteEndArray();
                    json.WriteStartArray("step_moved");
                    foreach (int moved in facts.MovedPerStep) json.WriteNumberValue(moved);
                    json.WriteEndArray();
                }
            }
            if (current.lastCommit is { } commit)
            {
                json.WriteStartObject("commit");
                json.WriteNumber("letters", commit.Letters);
                json.WriteString("expected", commit.Expected);
                json.WriteString("actual", commit.Actual);
                json.WriteString("error", commit.Error);
                json.WriteEndObject();
            }
            current.ReadAnswer(current.library.PreviewReport, root => { json.WritePropertyName("host"); root.WriteTo(json); return ""; });
            current.ReadAnswer(current.library.StateDigest, root => { json.WritePropertyName("head"); root.WriteTo(json); return ""; });
            json.WriteEndObject();
        }
    }
}

namespace Magic600.Shell
{
    public partial class Shell
    {
        private D02.ShellPreview? repairPreview;
        private PaneSpec[] previewPanes = Array.Empty<PaneSpec>(), panedPreview = Array.Empty<PaneSpec>();

        internal D02.ShellPreview? AttachPreview(ChromeFrame frame, JumbleHost jumble, CommitBar bar,
            Func<(double, double[])> motion, Action<string, ulong> committed, Action<string, string?> execute) =>
            _session && _hostLibrary is { } library
                ? repairPreview = new D02.ShellPreview(frame, jumble, bar, library, _host, () => _adoptedRevision, motion, committed, execute)
                : null;

        private (Rect2I Area, PaneLayout? Layout) PreviewOpenings(Rect2I global, Rect2I area, PaneLayout? main)
        {
            previewPanes = Array.Empty<PaneSpec>();
            if (repairPreview is not { Pending: true } preview || global.Size.X <= 0 || global.Size.Y <= 0) return (area, main);
            var primary = main is { } pair ? pair.Panes.Select(p => p with { Rect = p.Rect.Offset(pair.Surface.X, pair.Surface.Y) }).ToArray()
                : new[] { new PaneSpec("Global", new PaneRect(global.Position.X, global.Position.Y, global.Size.X, global.Size.Y)) };
            bool split = main != null && (D02.ShellRaw.On ? D02.ShellRaw.PaneKey.Antipode : D02.ShellSlice.Split);
            var extras = preview.Openings(primary.Length + (split ? 1 : 0)).ToArray();
            if (extras.Length == 0) return (area, main);
            // Containers can be between layouts while a pane is opening or the window is being resized.
            if (extras.Any(p => primary.Any(o => p.Rect.Overlaps(o.Rect))) ||
                (extras.Length == 2 && extras[0].Rect.Overlaps(extras[1].Rect))) return (area, main);
            var all = ViewPanes.Layout(primary.Concat(extras).ToArray());
            var s = all.Surface;
            previewPanes = all.Panes.Skip(primary.Length).ToArray();
            return (new Rect2I(s.X, s.Y, s.Width, s.Height), main == null ? null : new PaneLayout(s, all.Panes.Take(primary.Length).ToArray()));
        }

        private void SetPreviewPanes(HostLibrary library, uint first)
        {
            foreach (var pane in previewPanes)
            {
                var rect = pane.Rect;
                Require(library, "m6h_set_pane", library.SetPane(_host, first, HostLibrary.View,
                    rect.X, rect.Y, (uint)rect.Width, (uint)rect.Height, 4));
                uint flags = HostLibrary.PanePreview;
                if (pane.View == "Preview before") flags |= HostLibrary.PanePreviewBefore;
                Require(library, "m6h_set_pane_flags", library.SetPaneFlags(_host, first++, flags));
            }
            panedPreview = previewPanes;
        }
    }
}
