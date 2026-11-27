using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Godot;

namespace Magic600.Chrome;

internal static class ErrorOverlays
{
    public static HBoxContainer WithState(Control item, TokenSet tokens, DiagnosticState state)
    {
        var row = new HBoxContainer { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        row.AddChild(ScriptList.StateGlyph(tokens, state));
        item.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill;
        row.AddChild(item);
        return row;
    }

    public static void MarkLine(Control item, TokenSet tokens, DiagnosticState state)
    {
        Node parent = item.GetParent();
        int index = item.GetIndex();
        parent.RemoveChild(item);
        HBoxContainer row = WithState(item, tokens, state);
        parent.AddChild(row);
        parent.MoveChild(row, index);
    }
}

public sealed partial class JumbleHost
{
    private const string CertificateSource = "jumble.cap.certificate", RepairSource = "jumble.repair.list",
        CoversSourceId = "jumble.cap.covers";
    private ErrorOverlay? errors;
    private ResearchModel? errorResearch;
    private readonly HashSet<string> researchSources = new(StringComparer.Ordinal);
    private string? errorPoseDigest;
    private CapReadout? certificate;
    private string? coversError;
    private NuCorrections? checkedCorrections;

    public void BindErrors(ErrorOverlay overlay, ResearchModel? research)
    {
        errors = overlay;
        errorResearch = research;
        if (research != null) research.DiagnosticsChanged += PublishResearchErrors;
        Local.Changed += MarkMissingChecks;
        frame.Rebuilt += MarkCorrectionCheck;
        WatchCorrectionCheck();
        PublishResearchErrors();
    }

    private void PublishResearchErrors()
    {
        if (errors is not { } overlay || errorResearch is not { } research) return;
        errorPoseDigest = Style?.Digest;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var item in research.Diagnostics())
        {
            // A result is pinned to its input. Keep the named ids even when its matching geometry is unavailable.
            overlay.Set(item.Source, item.Label, item.State, item.Names, cell =>
                item.PoseDigest is { Length: > 0 } digest && digest == errorPoseDigest
                    ? ResolveCell(cell, overlay.PieceCount) : Array.Empty<int>());
            seen.Add(item.Source);
        }
        foreach (string source in researchSources.Except(seen)) overlay.Clear(source);
        researchSources.Clear();
        researchSources.UnionWith(seen);
    }

    private IEnumerable<int> ResolveCell(DiagnosticCell cell, int pieceCount) => cell.Kind == DiagnosticCellKind.Cap
        ? JumbleOverlay.PiecesOfCap(cell.Id, RecordSource?.Invoke(cell.Id), pieceCount)
        : ErrorOverlay.DirectPiece(cell);

    private void PublishErrorReadouts(CapReadout? readout, string? record)
    {
        if (errors is not { } overlay) return;
        if (errorResearch != null && errorPoseDigest != Style?.Digest) PublishResearchErrors();
        certificate = Source.State.Mode ? readout : null;
        coversError = null;
        if (certificate == null)
        {
            overlay.Clear(CertificateSource);
            overlay.Clear(RepairSource);
            overlay.Clear(CoversSourceId);
            return;
        }

        int cap = certificate.Cap;
        var name = new[] { new DiagnosticCell(DiagnosticCellKind.Cap, cap) };
        // Both refusals concern the same cut. Decode its mask once when an item needs it.
        int[]? resolved = null;
        IEnumerable<int> Resolve(DiagnosticCell _) => resolved ??= JumbleOverlay.PiecesOfCap(cap, record, overlay.PieceCount).ToArray();
        overlay.Set(CertificateSource, $"Cap {cap} certificate", certificate.CertificateState, name,
            certificate.CertificateState == DiagnosticState.Failed ? Resolve : null);
        if (Repair is { } repair)
            overlay.Set(RepairSource, $"Repair list at cap {repair.Cap}", repair.CheckState,
                new[] { new DiagnosticCell(DiagnosticCellKind.Cap, repair.Cap) },
                repair.CheckState == DiagnosticState.Failed && repair.Cap == cap ? Resolve : null);
        else overlay.Clear(RepairSource);

        int? refusedCoverCap = null;
        if (record != null)
        {
            using var document = JsonDocument.Parse(record);
            if (document.RootElement.TryGetProperty("covers", out JsonElement covers) && J.Text(covers, "status") == "error")
            {
                coversError = J.Text(covers, "message");
                if (covers.TryGetProperty("parameters", out JsonElement args) &&
                    args.TryGetProperty("cap", out JsonElement named) && named.TryGetInt32(out int namedCap)) refusedCoverCap = namedCap;
            }
        }
        if (coversError is { } refusal)
            overlay.Set(CoversSourceId, refusal, DiagnosticState.Failed,
                refusedCoverCap is int id ? new[] { new DiagnosticCell(DiagnosticCellKind.Cap, id) } : null,
                refusedCoverCap == cap ? Resolve : cell => ResolveCell(cell, overlay.PieceCount));
        else overlay.Clear(CoversSourceId);
        MarkMissingChecks();
        MarkCertificate();
    }

    private void MarkCertificate()
    {
        if (certificate is not { } readout || frame.Local?.Readouts.GetNodeOrNull<HBoxContainer>("JumbleCap/Readout0") is not { } item) return;
        if (item.GetNodeOrNull<Label>("CertificateState") is { } old) { item.RemoveChild(old); old.QueueFree(); }
        var mark = ScriptList.StateGlyph(frame.Tokens, readout.CertificateState);
        mark.Name = "CertificateState";
        mark.TooltipText = "Cap certificate: " + ErrorOverlay.StateName(readout.CertificateState);
        item.AddChild(mark);
    }

    private void MarkMissingChecks()
    {
        if (errors == null) return;
        if (coversError is { } refusal && Surface.FindChild("LocalBody_cap_covers", true, false) is Label covers && covers.GetParent() is VBoxContainer)
        {
            covers.Text = refusal;
            ErrorOverlays.MarkLine(covers, frame.Tokens, DiagnosticState.Failed);
        }
        if (Checks is { Pending: not null } && Surface.FindChild("LocalBody_twist_check", true, false) is Label pending && pending.GetParent() is VBoxContainer)
            ErrorOverlays.MarkLine(pending, frame.Tokens, DiagnosticState.Pending);
    }

    private void WatchCorrectionCheck()
    {
        if (errors == null) return;
        if (checkedCorrections != null) checkedCorrections.Changed -= MarkCorrectionCheck;
        checkedCorrections = Corrections.Model;
        if (checkedCorrections != null) checkedCorrections.Changed += MarkCorrectionCheck;
        MarkCorrectionCheck();
    }

    private void MarkCorrectionCheck()
    {
        var line = Corrections.GetNodeOrNull<Label>("CorrectionCheck") ?? Corrections.GetNodeOrNull<Label>("CorrectionResult/CorrectionCheck");
        if (line == null) return;
        if (line.GetParent() is not HBoxContainer)
        {
            ErrorOverlays.MarkLine(line, frame.Tokens, DiagnosticState.Pending);
            line.GetParent().Name = "CorrectionResult";
        }
        var row = (HBoxContainer)line.GetParent();
        var old = row.GetChild<Label>(0);
        row.RemoveChild(old); old.QueueFree();
        var state = checkedCorrections?.Checked?.After != null ? DiagnosticState.Ok : DiagnosticState.Pending;
        row.AddChild(ScriptList.StateGlyph(frame.Tokens, state));
        row.MoveChild(line, 1);
        row.Visible = checkedCorrections?.Checked != null;
    }
}
