using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;

namespace Magic600.Chrome;

public enum DiagnosticState { Pending, Ok, Failed }
public enum DiagnosticCellKind { Piece, Cap, Cell }
public readonly record struct DiagnosticCell(DiagnosticCellKind Kind, int Id);

public sealed record ErrorDiagnostic(string Source, string Label, DiagnosticState State,
    IReadOnlyList<DiagnosticCell> Names, IReadOnlyList<int> Pieces);

// ADR-D01.25: this model holds the view's diagnostic inputs and the union of the failed items' resolved pieces.
public sealed class ErrorOverlay
{
    private readonly Dictionary<string, ErrorDiagnostic> diagnostics = new(StringComparer.Ordinal);
    public int PieceCount { get; }
    public IReadOnlyList<int> Pieces { get; private set; } = Array.Empty<int>();
    public event Action? Changed;

    public ErrorOverlay(int pieceCount) => PieceCount = pieceCount;

    public void Set(string source, string label, DiagnosticState state, IEnumerable<DiagnosticCell>? names = null,
        Func<DiagnosticCell, IEnumerable<int>>? resolve = null)
    {
        var named = (names ?? Array.Empty<DiagnosticCell>()).Distinct().OrderBy(c => c.Kind).ThenBy(c => c.Id).ToArray();
        var pieces = named.SelectMany(resolve ?? DirectPiece).Where(p => p >= 0 && p < PieceCount).Distinct().Order().ToArray();
        if (diagnostics.TryGetValue(source, out var prior) && prior.Label == label && prior.State == state &&
            prior.Names.SequenceEqual(named) && prior.Pieces.SequenceEqual(pieces)) return;
        diagnostics[source] = new ErrorDiagnostic(source, label, state, Array.AsReadOnly(named), Array.AsReadOnly(pieces));
        Update();
    }

    public static IEnumerable<int> DirectPiece(DiagnosticCell cell) =>
        cell.Kind == DiagnosticCellKind.Piece ? new[] { cell.Id } : Array.Empty<int>();

    public void Clear(string source)
    {
        if (diagnostics.Remove(source)) Update();
    }

    public void ClearAll()
    {
        if (diagnostics.Count == 0) return;
        diagnostics.Clear();
        Update();
    }

    private void Update()
    {
        Pieces = Array.AsReadOnly(diagnostics.Values.Where(d => d.State == DiagnosticState.Failed)
            .SelectMany(d => d.Pieces).Distinct().Order().ToArray());
        Safe.Raise(Changed);
    }

    public static string StateName(DiagnosticState state) => state.ToString().ToLowerInvariant();

    // Sorted piece ids, encoded as little-endian int32s, give an order-independent digest.
    public void Dump(Utf8JsonWriter json)
    {
        json.WriteStartArray("diagnostics");
        foreach (var item in diagnostics.Values.OrderBy(d => d.Source, StringComparer.Ordinal))
        {
            json.WriteStartObject();
            json.WriteString("source", item.Source);
            json.WriteString("label", item.Label);
            json.WriteString("state", StateName(item.State));
            WriteIds(json, "caps", item.Names.Where(c => c.Kind == DiagnosticCellKind.Cap).Select(c => c.Id));
            WriteIds(json, "cells", item.Names.Where(c => c.Kind == DiagnosticCellKind.Cell).Select(c => c.Id));
            WriteIds(json, "named_pieces", item.Names.Where(c => c.Kind == DiagnosticCellKind.Piece).Select(c => c.Id));
            json.WriteNumber("pieces", item.Pieces.Count);
            json.WriteEndObject();
        }
        json.WriteEndArray();
        var bytes = new byte[Pieces.Count * sizeof(int)];
        for (int i = 0; i < Pieces.Count; i++) BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(i * sizeof(int)), Pieces[i]);
        json.WriteNumber("pieces", Pieces.Count);
        json.WriteString("sha256", Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant());
        WriteIds(json, "first_ids", Pieces.Take(16));
    }

    private static void WriteIds(Utf8JsonWriter json, string property, IEnumerable<int> ids)
    {
        json.WriteStartArray(property);
        foreach (int id in ids) json.WriteNumberValue(id);
        json.WriteEndArray();
    }
}
