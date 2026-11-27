using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static DiagnosticCell ErrorPiece(int id) => new(DiagnosticCellKind.Piece, id);

    private static void D02ErrorOverlayTracksFailedSources()
    {
        var overlay = new ErrorOverlay(10);
        int changes = 0;
        overlay.Changed += () => changes++;
        var first = new[] { ErrorPiece(4), ErrorPiece(2), ErrorPiece(4) };
        overlay.Set("one", "First check", DiagnosticState.Pending, first);
        Check(overlay.Pieces.Count == 0, "pending painted a piece");
        overlay.Set("one", "First check", DiagnosticState.Failed, first);
        Sequence(overlay.Pieces, new[] { 2, 4 }, "failed pieces");
        overlay.Set("two", "Second check", DiagnosticState.Failed, new[] { ErrorPiece(4), ErrorPiece(9) });
        overlay.Set("none", "No location", DiagnosticState.Failed);
        Sequence(overlay.Pieces, new[] { 2, 4, 9 }, "the union of two check sources");
        overlay.Set("one", "First check", DiagnosticState.Ok, first);
        Sequence(overlay.Pieces, new[] { 4, 9 }, "a passed check cleared only its own pieces");
        overlay.Clear("two");
        Check(overlay.Pieces.Count == 0, "clearing a failed check kept its pieces");
        overlay.ClearAll();
        Equal(changes, 7, "change events");
    }

    private static void D02ErrorOverlayUsesBlockedCapRecord()
    {
        const int pieceCount = 177_120;
        string record = Read("data/samples/jumble/wj-ia-300/cap-0.json");
        var cap = CapReadout.Read(record);
        var repair = RepairListModel.From(record, 300)!;
        Equal(cap.CertificateState, DiagnosticState.Failed, "the blocked certificate's status");
        Equal(repair.CheckState, DiagnosticState.Failed, "the blocked repair list's status");
        var overlay = new ErrorOverlay(pieceCount);
        overlay.Set("certificate", "Blocked cap", cap.CertificateState, new[] { new DiagnosticCell(DiagnosticCellKind.Cap, cap.Cap) },
            cell => JumbleOverlay.PiecesOfCap(cell.Id, record, pieceCount));
        Equal(overlay.Pieces.Count, 231, "the straddler count");
        using var stream = new MemoryStream();
        using (var json = new Utf8JsonWriter(stream))
        {
            json.WriteStartObject();
            overlay.Dump(json);
            json.WriteEndObject();
        }
        using var dump = JsonDocument.Parse(stream.ToArray());
        var item = dump.RootElement.GetProperty("diagnostics")[0];
        Equal(item.GetProperty("caps")[0].GetInt32(), 0, "the named cap");
        Equal(item.GetProperty("state").GetString(), "failed", "the diagnostic state");
        Equal(item.GetProperty("pieces").GetInt32(), 231, "the resolved piece count");
    }

    private static void D02ErrorOverlayKeepsResearchLocations()
    {
        var model = ResearchSample();
        var failed = model.Diagnostics().Where(d => d.State == DiagnosticState.Failed).ToArray();
        foreach (string prefix in new[] { ResearchModel.ClaimRef, ResearchRows.CheckRun + "/", ResearchRows.CheckReport + "/" })
        {
            var item = failed.Single(d => d.Source.StartsWith(prefix, StringComparison.Ordinal));
            Sequence(item.Names, new[] { new DiagnosticCell(DiagnosticCellKind.Cap, 17), new DiagnosticCell(DiagnosticCellKind.Cap, 18) },
                "the research location ids");
        }
        var noCells = new ResearchVerdict(VerdictKind.Fail, Reason: "the digests differed at byte 17, after 84 twists");
        Check(noCells.NamedCells().Count == 0, "the verdict's counts became location ids");
        Check(model.Lines().Any(line => line.Status == DiagnosticState.Failed), "the claim row has no failed state");
    }

    private static void D02ErrorOverlayIsViewOnly()
    {
        var feed = new JumbleFeed();
        feed.Adopt(new JumbleState(true, false, 0, 300, JumbleSessionKind.Solving));
        var before = feed.State;
        int adopted = 0;
        feed.Changed += _ => adopted++;
        var overlay = new ErrorOverlay(10);
        overlay.Set("check", "Piece check", DiagnosticState.Failed, new[] { ErrorPiece(5) });
        overlay.ClearAll();
        Check(ReferenceEquals(feed.State, before) && adopted == 0, "the overlay touched the adopted puzzle state");
    }
}
