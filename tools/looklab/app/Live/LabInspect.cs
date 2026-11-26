using System.Linq;
using Godot;
using LookLab.Core;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The lab's inspector: the shared sample inspector (SampleInspect) with the lab's own parameters as its View fields,
// which act on both views. The selection, the preview parameters, the problems and the task are sample data, because
// the lab has no engine and no runtime.
internal static class LabInspect
{
    public static readonly (string Id, string Label)[] ViewFields =
    {
        ("gap", "Cell gap"), ("stickerShrink", "Sticker size"), ("fieldOfView", "Field of view"), ("d4", "4D distance"), ("edgeWeight", "Edge weight"), ("fog", "Fog"),
    };

    public static SampleInspect Create(LabApp app, ChromeFrame frame, LabDemo demo)
    {
        static double Default(ParameterDefinition p) => ((NumberValue)p.Default).Value;
        var inspect = new SampleInspect(frame, demo, app.Linked.FocusCells, inspector =>
        {
            foreach ((string id, string label) in ViewFields)
            {
                ParameterDefinition p = app.State.Schema[id]; double value = app.State.Preset.Params.Number(id);
                inspector.AddNumber(InspectorModel.View, "view." + id, label, p.Min!.Value, p.Max!.Value, p.Step ?? .01, Default(p), value, false, value == Default(p) ? Provenance.Default : Provenance.Session);
            }
        });
        inspect.ViewEdited += field => app.State.Set(field.Id.Substring(5), new NumberValue(field.Value));
        app.State.Changed += (id, value) =>
        {
            if (GodotObject.IsInstanceValid(frame) && value is NumberValue number && ViewFields.Any(f => f.Id == id))
                inspect.Inspector.Sync("view." + id, number.Value, number.Value == Default(app.State.Schema[id]) ? Provenance.Default : Provenance.Session);
        };
        return inspect;
    }
}
