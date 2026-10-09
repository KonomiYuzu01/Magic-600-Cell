using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using LookLab.Core;

namespace LookLab.App.LL5;

internal sealed record CostRow(string Parameter, string Feature, string Setting, string Cost)
{
    public string Text => $"{Parameter} = {Setting}: {Cost}";
}

internal static class CostDisplay
{
    public static string Measurement(CostMeasurement? value, bool handleNull = true) => value == null
        ? handleNull ? "not measured" : ""
        : value.Milliseconds.ToString("R", CultureInfo.InvariantCulture) + " ms (source: " + value.Source + ")";

    public static IReadOnlyList<CostRow> Rows(ParameterSchema schema, ParameterSet parameters, CostTable table, bool handleNull = true) =>
        schema.Parameters.Where(p => p.CostFeature != null).Select(p => new CostRow(p.Id, p.CostFeature!,
            Setting(parameters.Values[p.Id]), Measurement(table.Features[p.CostFeature!], handleNull))).ToArray();

    private static string Setting(ParameterValue value) => value switch
    {
        NumberValue n => n.Value.ToString("R", CultureInfo.InvariantCulture),
        IntegerValue n => n.Value.ToString(CultureInfo.InvariantCulture),
        EnumValue e => e.Value,
        BooleanValue b => b.Value ? "enabled" : "disabled",
        ColourValue c => FormattableString.Invariant($"OKLCH({c.Value.L:R}, {c.Value.C:R}, {c.Value.H:R})"),
        CurveValue c => FormattableString.Invariant($"Bezier({c.Value.A:R}, {c.Value.B:R})"),
        _ => throw new FormatException("cost: unsupported parameter value")
    };

    // Measurements are independent table entries, never summed or scaled.
    public static string Summary(IReadOnlyList<CostRow> rows) => rows.All(r => r.Cost == "not measured")
        ? "cost not measured"
        : "cost: " + string.Join("; ", rows.DistinctBy(r => r.Feature).Select(r => r.Feature + " " + r.Cost));
}
