using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Godot;

namespace Magic600.Chrome;

// B16 (ADR-D02.26, .34; jumbling-observation.md rule 5): named-cut and repair-step data, hover references and
// each pane's count of pieces kept as outlines under Hide.
public sealed partial class JumbleHost
{
    public OverlayView Overlay { get; private set; } = OverlayView.Empty;
    public long OverlayHidden { get; private set; }
    public string OverlayLine { get; private set; } = "";
    public Func<byte[]?>? PreviewMoving { private get; set; }
    public Func<int, bool>? FocusConflict { private get; set; }
    // The overlay's pieces or its never-hidden set changed: the renderer sends the bytes again.
    public event Action? OverlayChanged;
    private (int? Cap, int Row, int Step, string? Record, string? HoveredRecord, IReadOnlyList<int>? Poses) drawnOverlay;

    private void WireOverlay()
    {
        Naming.Changed += _ => Reoverlay();
        Naming.HoveredChanged += _ => Reoverlay();
        Scrub.Changed += Reoverlay;
        Source.Changed += _ => Reoverlay();
    }

    // The overlay for the named cap and the focused step; the record is read again only when the cap, the step, the
    // record or the bound pose ids changed. A record that does not read draws no overlay and is reported.
    public void Reoverlay()
    {
        int? cap = Source.State.Mode ? Naming.Named : null;
        string? record = cap is int c && RecordSource != null && PosesFit ? RecordSource(c) : null;
        string? hovered = Source.State.Mode && Naming.Hovered is int h && h != cap && PosesFit ? RecordSource?.Invoke(h) : null;
        int row = Scrub.Row?.Number ?? 0, step = Scrub.Row == null ? 0 : Scrub.Step;
        var now = (cap, row, step, record, hovered, PoseOf);
        if (now == drawnOverlay)
        {
            OverlayWords();
            return;
        }
        var previous = drawnOverlay;
        drawnOverlay = now;
        try
        {
            Overlay = JumbleOverlay.Of(cap, record, PoseOf?.Count ?? 0, row, step, Scrub.Label, hovered);
        }
        catch (Exception error) when (error is FormatException or JsonException or KeyNotFoundException or InvalidOperationException or InvalidDataException)
        {
            Safe.Report(error);
            Overlay = OverlayView.Empty;
        }
        if ((cap, row, step, record, PoseOf) != (previous.Cap, previous.Row, previous.Step, previous.Record, previous.Poses))
            Filter?.RefreshFields();
        OverlayWords();
        Safe.Raise(OverlayChanged);
    }

    public void OverlayWords()
    {
        var moving = PreviewMoving?.Invoke();
        long Hidden(int index) => index < Layers.Count
            ? JumbleOverlay.Hidden(Overlay.NeverHidden, Layers[index], FocusMember(Layers[index]), moving, FocusConflict) : 0;
        OverlayHidden = Hidden(0);
        OverlayLine = JumbleOverlay.Words(Overlay, OverlayHidden);
        if (!GodotObject.IsInstanceValid(frame)) return;
        // SF5: while a cap stays named the line keeps its place, blank where the bound state has no record.
        bool named = Source.State.Mode && Naming.Named != null;
        foreach (var (view, line) in new[] { (frame.Global, OverlayLine), (frame.Local, JumbleOverlay.Words(Overlay, Hidden(1))) })
        {
            if (view == null || !GodotObject.IsInstanceValid(view)) continue;
            ViewLine.Set(view, JumbleOverlay.LineName, line.Length == 0 ? null : line,
                "Overlays are lines: the named cap's cut, a focused repair step's moving set, the never-hidden set.",
                frame.Tokens.C(JumbleOverlay.Slot), named);
        }
    }
}
