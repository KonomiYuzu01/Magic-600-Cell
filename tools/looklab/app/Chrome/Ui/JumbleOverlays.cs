using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Godot;

namespace Magic600.Chrome;

// B16 (ADR-D02.26, .34; jumbling-observation.md rule 5): the host's overlays. While a cap is named it reads the cap's
// record for the cut's trace, the focused repair step's moving set and the never-hidden set; the overlay's line, with
// the exact count of its pieces a focus layer would hide, shows on Global's and Local's borders. The renderer draws
// the lines on the outline channel and keeps the never-hidden set out of every focus layer (the shell's ShellTint).
public sealed partial class JumbleHost
{
    public OverlayView Overlay { get; private set; } = OverlayView.Empty;
    public long OverlayHidden { get; private set; }
    public string OverlayLine { get; private set; } = "";
    // The overlay's pieces or its never-hidden set changed: the renderer sends the bytes again.
    public event Action? OverlayChanged;
    private (int? Cap, int Row, int Step, string? Record, IReadOnlyList<int>? Poses) drawnOverlay;

    private void WireOverlay()
    {
        Naming.Changed += _ => Reoverlay();
        Scrub.Changed += Reoverlay;
        Source.Changed += _ => Reoverlay();
    }

    // The overlay for the named cap and the focused step; the record is read again only when the cap, the step, the
    // record or the bound pose ids changed. A record that does not read draws no overlay and is reported.
    public void Reoverlay()
    {
        int? cap = Source.State.Mode ? Naming.Named : null;
        string? record = cap is int c && RecordSource != null && PosesFit ? RecordSource(c) : null;
        int row = Scrub.Row?.Number ?? 0, step = Scrub.Row == null ? 0 : Scrub.Step;
        var now = (cap, row, step, record, PoseOf);
        if (now == drawnOverlay)
        {
            OverlayWords();
            return;
        }
        drawnOverlay = now;
        try
        {
            Overlay = JumbleOverlay.Of(record == null ? null : cap, record, PoseOf?.Count ?? 0, row, step, Scrub.Label);
        }
        catch (Exception error) when (error is FormatException or JsonException or KeyNotFoundException or InvalidOperationException or InvalidDataException)
        {
            Safe.Report(error);
            Overlay = OverlayView.Empty;
        }
        OverlayWords();
        Safe.Raise(OverlayChanged);
    }

    // The overlay's line with the exact count of its pieces the dimming view's focus layer would hide (Global's first,
    // as the renderer's dim set), on both borders.
    public void OverlayWords()
    {
        FocusLayers? source = Layers.Count > 0 && Layers[0].Dimming ? Layers[0] : Layers.Count > 1 ? Layers[1] : null;
        OverlayHidden = source == null ? 0 : JumbleOverlay.Hidden(Overlay.Lines, source, FocusMember(source));
        OverlayLine = JumbleOverlay.Words(Overlay, OverlayHidden);
        if (!GodotObject.IsInstanceValid(frame)) return;
        foreach (ViewFrame? view in new[] { frame.Global, frame.Local })
        {
            if (view == null || !GodotObject.IsInstanceValid(view)) continue;
            ViewLine.Set(view, JumbleOverlay.LineName, OverlayLine.Length == 0 ? null : OverlayLine,
                "Overlays are lines: the named cap's cut, a focused repair step's moving set, the never-hidden set.",
                frame.Tokens.C(JumbleOverlay.Slot));
        }
    }
}
