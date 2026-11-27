using System.Linq;
using Godot;
using LookLab.Core;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The lab's inspector: the shared sample inspector (SampleInspect) with the lab's own parameters as its View fields
// (the shared ViewFields), which act on both views: the lab's two views share one set, so Both is its only scope. The
// selection, the preview parameters, the problems and the task are sample data, because the lab has no engine and no
// runtime.
internal static class LabInspect
{
    public static readonly (string Id, string Label)[] Parameters =
    {
        ("gap", "Cell gap"), ("stickerShrink", "Sticker size"), ("fieldOfView", "Field of view"), ("d4", "4D distance"), ("edgeWeight", "Edge weight"), ("fog", "Fog"),
    };

    public static SampleInspect Create(LabApp app, ChromeFrame frame, LabDemo demo)
    {
        static double Default(ParameterDefinition p) => ((NumberValue)p.Default).Value;
        ViewFields views = null!;
        var inspect = new SampleInspect(frame, demo, app.Linked.FocusCells, inspector =>
        {
            ViewFieldSpec[] specs = Parameters.Select(f => { ParameterDefinition p = app.State.Schema[f.Id]; return new ViewFieldSpec("view." + f.Id, f.Label, p.Min!.Value, p.Max!.Value, p.Step ?? .01); }).ToArray();
            views = new ViewFields(inspector, specs, false, (_, id) => app.State.Preset.Params.Number(id.Substring(5)), (_, id) => Default(app.State.Schema[id.Substring(5)]));
        });
        views.Changed += change => app.State.Set(change.Id.Substring(5), new NumberValue(change.Value));
        app.State.Changed += (id, value) =>
        {
            if (GodotObject.IsInstanceValid(frame) && value is NumberValue number && Parameters.Any(f => f.Id == id)) views.Sync("Global", "view." + id, number.Value);
        };
        return inspect;
    }
}
