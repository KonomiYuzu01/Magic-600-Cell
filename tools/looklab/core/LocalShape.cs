using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace LookLab.Core;

// What the shader needs to draw one Local frame: the 4D rotation, the cap of cells that is drawn and the scale.
public sealed record LocalFrame(double[] Q, double[] Axis, double CapCos, double Cs, double Ss, double D4, double Zoom, ProjectionKind Projection, double Spread);

// The shape of the Local view (ADR-D01.40). Local is not a zoomed Global: it draws only the focus cells and the
// cells around them, centred and at a fixed size, with its own projection, gap and sticker size. The shape is a
// data file, so a script can replace it for a solving workflow.
public sealed record LocalShape(ProjectionKind Projection, double D4, double Gap, double StickerShrink, double Margin, double Fill, double Spread)
{
    public const string Format = "magic600-look-local-view";
    public const double MaxCap = 1.45;

    public static LocalShape Parse(string text, ParameterSchema schema)
    {
        using var document = Json.Parse(text);
        JsonElement root = document.RootElement;
        Json.Keys(root, "format", "version", "projection", "d4", "gap", "stickerShrink", "margin", "fill", "spread");
        Json.Header(root, Format);
        double Ranged(string id)
        {
            ParameterDefinition p = schema[id];
            ParameterValue value = ParameterSchema.ReadValue(p, Json.Get(root, id));
            ParameterSchema.ValidateValue(p, value);
            return Json.Number(Json.Get(root, id), id);
        }
        ProjectionKind projection = Json.Text(root, "projection") switch
        {
            "perspective" => ProjectionKind.Perspective,
            "orthographic" => ProjectionKind.Orthographic,
            _ => throw Json.Error("projection", "expected perspective or orthographic")
        };
        double margin = Json.Number(Json.Get(root, "margin"), "margin"), fill = Json.Number(Json.Get(root, "fill"), "fill");
        if (!(margin >= 0 && margin <= 1)) throw Json.Error("margin", "outside 0..1");
        if (!(fill >= .2 && fill <= 1)) throw Json.Error("fill", "outside 0.2..1");
        double spread = Json.Number(Json.Get(root, "spread"), "spread");
        if (!(spread >= 1 && spread <= 3)) throw Json.Error("spread", "outside 1..3");
        return new LocalShape(projection, Ranged("d4"), Ranged("gap"), Ranged("stickerShrink"), margin, fill, spread);
    }

    // The frame for a set of focus cells, given every cell's centre (four numbers per cell, any length) and the
    // 3D turn the linked rotation gives the view. Matrices are column-major, as the shader reads them.
    public LocalFrame Frame(ReadOnlySpan<float> centres, IReadOnlyCollection<int> focus, ReadOnlySpan<double> spin, double aspect)
    {
        if (focus.Count == 0) throw new ArgumentException("the Local view needs at least one focus cell");
        var unit = new List<double[]>();
        foreach (int cell in focus.Distinct())
        {
            double[] c = { centres[cell * 4], centres[cell * 4 + 1], centres[cell * 4 + 2], centres[cell * 4 + 3] };
            double length = Math.Sqrt(c.Sum(x => x * x));
            unit.Add(c.Select(x => x / length).ToArray());
        }
        double[] axis = Enumerable.Range(0, 4).Select(i => unit.Sum(c => c[i])).ToArray();
        double norm = Math.Sqrt(axis.Sum(x => x * x));
        axis = norm > 1e-9 ? axis.Select(x => x / norm).ToArray() : unit[0];
        double widest = unit.Max(c => Math.Acos(Math.Clamp(c.Zip(axis, (a, b) => a * b).Sum(), -1, 1)));
        double cap = Math.Min(widest + Margin, MaxCap);
        // The rotation that takes the axis to +w, then the linked 3D turn.
        double[] toW = Geometry.Identity();
        double dot = axis[3];
        if (dot > -1 + 1e-9)
        {
            double[] sum = { axis[0], axis[1], axis[2], axis[3] + 1 };
            for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++)
                toW[j * 4 + i] += -sum[i] * sum[j] / (1 + dot) + (i == 3 ? 2 * axis[j] : 0);
        }
        else { toW[10] = -1; toW[15] = -1; }
        var q = new double[16];
        for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++)
            for (int k = 0; k < 4; k++) q[j * 4 + i] += spin[k * 4 + i] * toW[j * 4 + k];
        // The cap is drawn as a ball of this radius, seen from the camera at distance 5.
        double edge = Projection == ProjectionKind.Perspective ? D4 * Math.Sin(cap) / (D4 - Math.Cos(cap)) : Math.Sin(cap);
        double centre = Projection == ProjectionKind.Perspective ? D4 / (D4 - 1) : 1;
        double radius = Math.Min(edge * Spread + .12 * centre, 4);
        double zoom = Fill * Math.Sqrt(25 - radius * radius) / radius * Math.Min(1, aspect);
        return new LocalFrame(q, axis, Math.Cos(cap), 1 - Gap, StickerShrink, D4, zoom, Projection, Spread);
    }
}
