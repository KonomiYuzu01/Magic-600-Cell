using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using LookLab.Core;

namespace LookLab.App.LL3;

// These rules have no Godot dependency and are also compiled into the core tests.
internal static class OutputPaths
{
    public static string FileAt(string root, string folder, string name)
    {
        root = Path.GetFullPath(root);
        string path = Path.GetFullPath(Path.Combine(root, folder, name));
        if (name != Path.GetFileName(name) || !Below(root, path)) throw new IOException("Output must stay inside the checkout.");
        for (string? current = path; current != null && Below(root, current); current = Path.GetDirectoryName(current))
            if ((File.Exists(current) || Directory.Exists(current)) && (File.GetAttributes(current) & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Look Lab output paths must not pass through links.");
        return path;
    }

    public static bool Below(string root, string path)
    {
        string relative = Path.GetRelativePath(Path.GetFullPath(root), Path.GetFullPath(path));
        return relative != "." && relative != ".." && !Path.IsPathRooted(relative) &&
            !relative.StartsWith(".." + Path.DirectorySeparatorChar, StringComparison.Ordinal);
    }
}

internal sealed class SwipeSession
{
    public const string Folder = "work/loop-memory/looklab/swipe";
    public Preset Best { get; private set; }
    public Preset Candidate { get; private set; }
    public IReadOnlyList<string> ChangedIds { get; private set; } = Array.Empty<string>();
    public string RecordPath { get; }
    private readonly string root;
    private readonly ParameterSchema schema;
    private uint random;

    public SwipeSession(string outputRoot, ParameterSchema schema, Preset best, int seed)
    {
        Presets.Validate(best, schema);
        root = outputRoot; this.schema = schema; Best = best; Candidate = best;
        random = unchecked((uint)seed);
        RecordPath = OutputPaths.FileAt(root, Folder, "swipe-seed-" + seed.ToString(CultureInfo.InvariantCulture) + "-" + Guid.NewGuid().ToString("N") + ".jsonl");
        NextCandidate();
    }

    public static string Digest(Preset preset, ParameterSchema schema) => AssetCatalogue.Digest(Presets.CanonicalBytes(preset, schema));

    // An explicit LCG makes replay independent of System.Random's implementation.
    private int Next(int count)
    {
        random = unchecked(1664525U * random + 1013904223U);
        return (int)((random >> 1) % (uint)count);
    }

    private static double Nudge(double value, double min, double max, double step, int direction)
    {
        double next = Math.Clamp(value + direction * step, min, max);
        return next == value ? Math.Clamp(value - direction * step, min, max) : next;
    }

    private ParameterValue Change(ParameterDefinition p, ParameterValue value)
    {
        int direction = Next(2) == 0 ? -1 : 1;
        switch (value)
        {
            case NumberValue n: return new NumberValue(Nudge(n.Value, p.Min!.Value, p.Max!.Value, p.Step!.Value, direction));
            case IntegerValue n: return new IntegerValue((int)Nudge(n.Value, p.Min!.Value, p.Max!.Value, p.Step!.Value, direction));
            case EnumValue e:
                IReadOnlyList<string> options = p.Options!;
                int index = options.ToList().IndexOf(e.Value);
                return new EnumValue(options[(index + direction + options.Count) % options.Count]);
            case BooleanValue b: return new BooleanValue(!b.Value);
            case ColourValue c:
                return new ColourValue(Next(3) switch
                {
                    0 => c.Value with { L = Nudge(c.Value.L, 0, 1, .01, direction) },
                    1 => c.Value with { C = Nudge(c.Value.C, 0, 1, .01, direction) },
                    _ => c.Value with { H = Nudge(c.Value.H, 0, 360, 1, direction) }
                });
            case CurveValue c:
                return new CurveValue(Next(2) == 0 ? c.Value with { A = Nudge(c.Value.A, 0, 1, .01, direction) }
                    : c.Value with { B = Nudge(c.Value.B, 0, 1, .01, direction) });
            default: throw new FormatException(p.Id + ": unsupported candidate parameter");
        }
    }

    private void NextCandidate()
    {
        var available = schema.Parameters.Where(p => p.Kind != ParameterKind.Enum || p.Options!.Count > 1).ToList();
        int count = 1 + Next(Math.Min(3, available.Count));
        ParameterSet values = Best.Params;
        var changed = new HashSet<string>(StringComparer.Ordinal);
        for (int i = 0; i < count; i++)
        {
            int index = Next(available.Count); ParameterDefinition p = available[index]; available.RemoveAt(index);
            ParameterValue value = Change(p, values.Values[p.Id]);
            ParameterSchema.ValidateValue(p, value); values = values.With(p.Id, value); changed.Add(p.Id);
        }
        Candidate = Best with { Params = values };
        ChangedIds = schema.Parameters.Where(p => changed.Contains(p.Id)).Select(p => p.Id).ToArray();
    }

    public void Answer(bool better)
    {
        // Persist the compared pair before promotion; failed IO leaves the pair intact.
        string record = JsonSerializer.Serialize(new
        {
            time = DateTimeOffset.UtcNow.ToString("O", CultureInfo.InvariantCulture),
            best_sha256 = Digest(Best, schema), candidate_sha256 = Digest(Candidate, schema),
            verdict = better ? "better" : "worse", changed_parameter_ids = ChangedIds
        });
        string path = OutputPaths.FileAt(root, Folder, Path.GetFileName(RecordPath));
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.AppendAllText(path, record + "\n", new UTF8Encoding(false));
        if (better) Best = Candidate;
        NextCandidate();
    }
}

// The drawing shader's camera is the 4D Q matrix. Each Godot viewport retains
// its local 3D culling camera, while every pane binds this one exploration camera.
internal sealed class CompareCamera
{
    public double[] Q { get; } = Geometry.Identity();
    public void Drag(double x, double y)
    {
        Geometry.Rotate(Q, 0, 3, x * .004); Geometry.Rotate(Q, 1, 3, y * .004);
    }
}

internal sealed class ComparePane
{
    public CompareCamera Camera { get; }
    public TurnClock Clock { get; }
    public TurnFrame Frame { get; internal set; }
    public ComparePane(CompareCamera camera, TurnClock clock) { Camera = camera; Clock = clock; Frame = clock.Frame(); }
}

internal sealed class CompareSession
{
    public CompareCamera Camera { get; } = new();
    public TurnClock Clock { get; }
    public IReadOnlyList<ComparePane> Panes { get; }
    public bool ClockEnabled { get; set; } = true;

    public CompareSession(ParameterSchema schema, IReadOnlyList<Preset> presets, TurnData turn, Func<double> nowMs)
    {
        if (presets.Count < 2 || presets.Count > 4) throw new FormatException("compare: choose two to four presets");
        foreach (Preset preset in presets) Presets.Validate(preset, schema);
        ParameterSet timing = presets[0].Params;
        Clock = new TurnClock(turn, timing.Number("turnMs"), timing.Number("easeA"), timing.Number("easeB"), nowMs);
        Panes = Enumerable.Range(0, presets.Count).Select(_ => new ComparePane(Camera, Clock)).ToArray();
        Advance();
    }

    public void Advance()
    {
        if (!ClockEnabled) return;
        TurnFrame frame = Clock.Frame();
        foreach (ComparePane pane in Panes) pane.Frame = frame;
    }
}
