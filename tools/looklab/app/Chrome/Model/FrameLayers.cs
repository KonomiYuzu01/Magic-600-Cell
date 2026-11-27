using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D01.77: the frame is a canvas in two layers. Surfaces are for the design and art line to paint; instruments keep
// a meaning fixed by name in every theme, and the number of their values is framework while their drawing is the art
// line's.
public enum FrameLayer { Surface, Instrument }

// One part of the frame. Names are role ids, or a prefix ending in a dot for every role under it; Roles are the role
// ids they resolve to. Styles are the chrome's type variations that paint a surface, one name per surface so the art
// line repaints it alone. An instrument with no roles is not drawn in this build yet.
public sealed record FramePart(string Id, string Words, FrameLayer Layer, IReadOnlyList<string> Names, IReadOnlyList<string> Roles,
    IReadOnlyList<string> Styles);

public sealed class FrameLayers
{
    public IReadOnlyList<FramePart> Surfaces { get; }
    public IReadOnlyList<FramePart> Instruments { get; }
    private readonly IReadOnlyDictionary<string, ColourRole> roles;
    private readonly Dictionary<string, FramePart> byRole = new(StringComparer.Ordinal);
    private readonly Dictionary<string, FramePart> byStyle = new(StringComparer.Ordinal);

    // A role a part names takes that part's layer. Any other role with a cue that is not colour (a state or encoding
    // role) is an instrument: its meaning is fixed by its name. The rest are surfaces.
    public FrameLayer Layer(string role) => byRole.TryGetValue(role, out FramePart? part) ? part.Layer
        : roles.TryGetValue(role, out ColourRole? found) && found.Cue != null ? FrameLayer.Instrument : FrameLayer.Surface;
    public FramePart? Part(string role) => byRole.TryGetValue(role, out FramePart? part) ? part : null;
    public FramePart? StylePart(string style) => byStyle.TryGetValue(style, out FramePart? part) ? part : null;

    // The role's place in words, for a tooltip: "Instrument: locks", "Surface: the shell", or the bare layer.
    public string Words(string role) => (Layer(role) == FrameLayer.Instrument ? "Instrument" : "Surface") + (Part(role) is { } part ? ": " + part.Words : "");

    private FrameLayers(IReadOnlyDictionary<string, ColourRole> roles, IReadOnlyList<FramePart> surfaces, IReadOnlyList<FramePart> instruments)
    {
        this.roles = roles; Surfaces = surfaces; Instruments = instruments;
        foreach (FramePart part in surfaces.Concat(instruments))
        {
            foreach (string role in part.Roles)
                if (!byRole.TryAdd(role, part)) throw J.Error(role, $"in two parts of the frame, {byRole[role].Id} and {part.Id}");
            foreach (string style in part.Styles)
                if (!byStyle.TryAdd(style, part)) throw J.Error(style, $"paints two parts of the frame, {byStyle[style].Id} and {part.Id}");
        }
    }

    internal static FrameLayers Parse(JsonElement value, IReadOnlyDictionary<string, ColourRole> roles)
    {
        J.Keys(value, "layers", "note", "surfaces", "instruments");
        J.Text(value, "note");
        var ids = new HashSet<string>(StringComparer.Ordinal);
        List<FramePart> Parts(string key, FrameLayer layer)
        {
            JsonElement list = value.GetProperty(key);
            if (list.ValueKind != JsonValueKind.Array) throw J.Error(key, "expected a list");
            var parts = new List<FramePart>();
            foreach (JsonElement item in list.EnumerateArray())
            {
                if (layer == FrameLayer.Surface) J.Keys(item, key, "id", "words", "roles", "styles");
                else J.Keys(item, key, "id", "words", "roles");
                string id = J.Text(item, "id");
                if (!ids.Add(id)) throw J.Error(id, "repeated part of the frame");
                string[] names = List(item.GetProperty("roles"), id);
                var resolved = new List<string>();
                foreach (string name in names)
                {
                    string[] matched = name.EndsWith('.') ? roles.Keys.Where(r => r.StartsWith(name, StringComparison.Ordinal)).ToArray()
                        : roles.ContainsKey(name) ? new[] { name } : Array.Empty<string>();
                    if (matched.Length == 0) throw J.Error(id, $"{name} names no role");
                    if (layer == FrameLayer.Instrument && matched.FirstOrDefault(r => roles[r].Cue == null) is { } plain)
                        throw J.Error(id, $"{plain} is an instrument's role, so it needs a cue that is not colour");
                    resolved.AddRange(matched);
                }
                string[] styles = layer == FrameLayer.Surface ? List(item.GetProperty("styles"), id) : Array.Empty<string>();
                if (layer == FrameLayer.Surface && names.Length == 0 && styles.Length == 0) throw J.Error(id, "a surface needs a role or a style to paint it");
                parts.Add(new FramePart(id, J.Text(item, "words"), layer, Array.AsReadOnly(names), resolved.Distinct(StringComparer.Ordinal).ToArray(), Array.AsReadOnly(styles)));
            }
            return parts;
        }
        return new FrameLayers(roles, Parts("surfaces", FrameLayer.Surface).AsReadOnly(), Parts("instruments", FrameLayer.Instrument).AsReadOnly());
    }

    // A list of text that may be empty (an instrument not drawn yet), with no entry twice.
    private static string[] List(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.Array) throw J.Error(id, "expected a list of text");
        string[] result = value.EnumerateArray().Select(v => J.String(v, id)).ToArray();
        return result.Distinct(StringComparer.Ordinal).Count() == result.Length ? result : throw J.Error(id, "repeated entry");
    }
}
