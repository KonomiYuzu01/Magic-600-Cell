using System;
using System.Buffers.Binary;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using LookLab.Core;

internal static partial class Program
{
    private static void ManifestFailures()
    {
        using var temp = new TestFolder();
        string folder = Path.Combine(temp.Path, "assets"); Directory.CreateDirectory(folder);
        byte[] data = Encoding.UTF8.GetBytes("verified asset");
        string path = Path.Combine(folder, "mesh.json");
        string manifest = Path.Combine(folder, "manifest.json");
        File.WriteAllText(manifest, JsonSerializer.Serialize(new { model_id = "test", files = new System.Collections.Generic.Dictionary<string, string> { ["assets/mesh.json"] = AssetCatalogue.Digest(data) } }));
        var catalogue = new AssetCatalogue(temp.Path);
        Refuse(() => catalogue.ReadVerified("mesh.json"), "mesh.json");
        Refuse(() => catalogue.ReadVerified("mesh_vertices.f32"), "mesh_vertices.f32");
        File.WriteAllBytes(path, data);
        Sequence(catalogue.ReadVerified("mesh.json"), data);
        File.WriteAllText(path, "different");
        Refuse(() => catalogue.ReadVerified("mesh.json"), "mesh.json");
        Refuse(() => catalogue.ReadVerified("../model.npz"), "model.npz");
    }

    private static byte[] Npy(string dtype, string shape = "(2,)", string order = "False", int dataBytes = 16)
    {
        string header = "{'descr': '" + dtype + "', 'fortran_order': " + order + ", 'shape': " + shape + ", }\n";
        byte[] bytes = new byte[10 + header.Length + dataBytes];
        new byte[] { 147, 78, 85, 77, 80, 89, 1, 0 }.CopyTo(bytes, 0);
        BinaryPrimitives.WriteUInt16LittleEndian(bytes.AsSpan(8, 2), (ushort)header.Length);
        Encoding.ASCII.GetBytes(header).CopyTo(bytes, 10);
        return bytes;
    }

    private static void NpyRefusals()
    {
        Sequence(NpyReader.Read(Npy("<f8"), "<f8", "normals.npy").Doubles(), new[] { 0.0, 0.0 });
        Sequence(NpyReader.Read(Npy("<i2", dataBytes: 4), "<i2", "face_values.npy").Integers(), new[] { 0, 0 });
        foreach (string dtype in new[] { "<f4", ">f8", "|O", "<i4" }) Refuse(() => NpyReader.Read(Npy(dtype), "<f8", "normals.npy"), "normals.npy");
        Refuse(() => NpyReader.Read(Npy("<f8", order: "True"), "<f8", "normals.npy"), "normals.npy");
        Refuse(() => NpyReader.Read(Npy("<f8", "(3,)"), "<f8", "normals.npy"), "normals.npy");
        Refuse(() => NpyReader.Read(Npy("<f8").Take(9).ToArray(), "<f8", "normals.npy"), "normals.npy");
        byte[] nan = Npy("<f8"); BinaryPrimitives.WriteDoubleLittleEndian(nan.AsSpan(nan.Length - 8, 8), double.NaN);
        Refuse(() => NpyReader.Read(nan, "<f8", "normals.npy").Doubles(), "normals.npy");
    }

    private static StructureValues ReferenceValues(JsonElement fixture)
    {
        var p = fixture.GetProperty("parameters");
        return new StructureValues(p.GetProperty("cs").GetDouble(), p.GetProperty("ss").GetDouble(), p.GetProperty("d4").GetDouble(),
            p.GetProperty("zoom").GetDouble(), fixture.GetProperty("aspect").GetDouble());
    }

    private static double[] Camera(JsonElement data)
    {
        double[] q = Geometry.Identity();
        foreach (var r in data.GetProperty("rotations").EnumerateArray()) Geometry.Rotate(q, r[0].GetInt32(), r[1].GetInt32(), r[2].GetDouble());
        return q;
    }

    private static void SbReference()
    {
        using var document = JsonDocument.Parse(Read("fixtures/sb-reference.json"));
        var fixture = document.RootElement;
        Equal(fixture.GetProperty("cases").GetArrayLength(), 9);
        int total = 0, moving = 0;
        foreach (var data in fixture.GetProperty("cases").EnumerateArray())
        {
            Equal(data.GetProperty("samples").GetArrayLength(), 300);
            double[] q = Camera(data);
            int previous = -1;
            foreach (var sample in data.GetProperty("samples").EnumerateArray())
            {
                int g = sample.GetProperty("vertex").GetInt32(); Check(g > previous, "samples not spread in source order"); previous = g;
                Projection actual = Mesh.Project(g, ReferenceValues(fixture), q, Turn, data.GetProperty("theta").GetDouble());
                var reference = sample.GetProperty("reference");
                Near(actual.NdcX, reference[0].GetDouble(), 1e-5, 1e-5);
                Near(actual.NdcY, reference[1].GetDouble(), 1e-5, 1e-5);
                Near(actual.W, reference[2].GetDouble(), 1e-5, 1e-5);
                total++;
                if (data.GetProperty("pose").GetString() == "mid" && Math.Abs(Mesh.Project(g, ReferenceValues(fixture), q, Turn, 0).W - actual.W) > 1e-8) moving++;
            }
        }
        Equal(total, 2700); Check(moving > 0, "fixture must exercise animated vertices");
    }

    private static void FixtureProvenance()
    {
        using var document = JsonDocument.Parse(Read("fixtures/sb-reference.json")); var f = document.RootElement;
        Equal(f.GetProperty("version").GetInt32(), 2);
        Equal(f.GetProperty("sourceBranch").GetString(), "main"); Check(f.GetProperty("sourceCommit").GetString()!.Length == 40, "source commit missing");
        Equal(f.GetProperty("sourceSampleCount").GetInt32(), 9066);
        int[] positions = f.GetProperty("samplePositions").EnumerateArray().Select(x => x.GetInt32()).ToArray();
        Sequence(positions, Enumerable.Range(0, 300).Select(i => i * 9065 / 299));
        string digest = AssetCatalogue.Digest(File.ReadAllBytes(FileAt("fixtures/w3-turn.json")));
        Equal(digest, f.GetProperty("referenceInputs").GetProperty("work/experiments/renderer-sb/workload/turn.json").GetString());
        Equal(digest, f.GetProperty("inputDigests").GetProperty("workload/turn.json").GetString());
        foreach (var p in f.GetProperty("referenceInputs").EnumerateObject().Where(p => p.Name.StartsWith("assets/", StringComparison.Ordinal)))
            Equal(AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(Root, p.Name))), p.Value.GetString(), p.Name);
        Equal(f.GetProperty("inputDigests").EnumerateObject().Count(), 16);
        foreach (var digestEntry in f.GetProperty("inputDigests").EnumerateObject()) Check(Read("fixtures/PROVENANCE.md").Contains(digestEntry.Value.GetString()!, StringComparison.Ordinal), "provenance digest absent");
    }

    private static void StickerAnchorRule()
    {
        Equal(Mesh.StickerAnchors.Length, Geometry.StickersPerCell * 4);
        Equal(Geometry.AnchorDigest(Mesh.StickerAnchors.Span), Geometry.AnchorSha256);
        using var document = JsonDocument.Parse(Read("fixtures/sb-reference.json"));
        Equal(document.RootElement.GetProperty("anchors").GetProperty("sha256").GetString(), Geometry.AnchorSha256);
        // Areas 1/2 and 1 with centroids (1/3, 1/3, 0, 0) and (8/3, 1/3, 0, 1) give the anchor (17/9, 1/3, 0, 2/3).
        double[] anchor = Geometry.Anchors(new[] { 0, 6 }, new double[] { 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 2, 0, 0, 1, 4, 0, 0, 1, 2, 1, 0, 1 });
        Equal(anchor.Length, 4);
        foreach (var (actual, expected) in anchor.Zip(new[] { 17.0 / 9, 1.0 / 3, 0, 2.0 / 3 })) Near(actual, (float)expected, 1e-7);
        Refuse(() => Geometry.Anchors(new[] { 0, 3 }, new double[] { 0, 0, 0, 0, 1, 1, 0, 0, 2, 2, 0, 0 }), "anchors");
        Refuse(() => Geometry.Anchors(new[] { 0, 4 }, new double[16]), "anchors");
    }

    private static void SbFixturePortable()
    {
        using var temp = new TestFolder();
        string checkout = Path.Combine(temp.Path, "checkout"), fixtures = Path.Combine(checkout, "tools/looklab/fixtures");
        Directory.CreateDirectory(fixtures); Directory.CreateDirectory(Path.Combine(checkout, "assets"));
        File.WriteAllText(Path.Combine(checkout, "assets/model.npz"), "synthetic model input for provenance only");
        string script = Path.Combine(fixtures, "make_sb_fixture.py"); File.Copy(FileAt("fixtures/make_sb_fixture.py"), script);
        var missing = Run("python", new[] { "-B", script }, workingDirectory: temp.Path);
        Equal(missing.Code, 2); Check(missing.Error.Contains("input_folder", StringComparison.Ordinal), "input folder is not a required argument");
        Equal(Directory.GetFiles(fixtures).Length, 1, "missing argument wrote fixtures");
        string source = Path.Combine(temp.Path, "inputs"); Directory.CreateDirectory(Path.Combine(source, "reference")); Directory.CreateDirectory(Path.Combine(source, "workload"));
        void Input(string name, byte[] bytes) => File.WriteAllBytes(Path.Combine(source, name), bytes);
        Input("SOURCE_COMMIT", Encoding.UTF8.GetBytes(new string('1', 40) + "\n"));
        Input("SPEC.md", Encoding.UTF8.GetBytes("synthetic specification\n")); Input("reference_geometry.py", Encoding.UTF8.GetBytes("# synthetic reference\n"));
        Input("workload/turn.json", File.ReadAllBytes(FileAt("fixtures/three-cycle-turn.json")));
        var cameras = new JsonArray();
        for (int c = 0; c < 3; c++) cameras.Add(new JsonObject { ["name"] = "c" + c, ["rotations"] = new JsonArray() });
        Input("cameras.json", Encoding.UTF8.GetBytes(new JsonObject { ["cameras"] = cameras }.ToJsonString()));
        const int count = 9066; var sample = new byte[count * 4];
        for (int i = 0; i < count; i++) BinaryPrimitives.WriteUInt32LittleEndian(sample.AsSpan(i * 4, 4), (uint)(i * 7));
        Input("reference/sample.u32", sample);
        var files = new JsonObject { ["sample.u32"] = AssetCatalogue.Digest(sample) };
        for (int c = 0; c < 3; c++) foreach (string pose in new[] { "start", "mid", "end" })
        {
            var bytes = new byte[count * 12];
            for (int i = 0; i < count; i++)
            {
                BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 12, 4), i);
                BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 12 + 4, 4), c);
                BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 12 + 8, 4), pose == "start" ? 0 : pose == "mid" ? 1 : 2);
            }
            string name = $"c{c}_{pose}.f32"; Input("reference/" + name, bytes); files[name] = AssetCatalogue.Digest(bytes);
        }
        var index = new JsonObject
        {
            ["sample_count"] = count, ["files"] = files,
            ["inputs"] = new JsonObject
            {
                ["work/experiments/renderer-sb/cameras.json"] = AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(source, "cameras.json"))),
                ["work/experiments/renderer-sb/workload/turn.json"] = AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(source, "workload/turn.json")))
            },
            ["poses"] = new JsonObject { ["start"] = 0, ["mid"] = 1, ["end"] = 2 },
            ["parameters"] = new JsonObject { ["cs"] = .76, ["ss"] = .82, ["d4"] = 1.18, ["zoom"] = 1.15 }, ["aspect"] = 1.6
        };
        Input("reference/index.json", Encoding.UTF8.GetBytes(index.ToJsonString()));
        var noAnchors = Run("python", new[] { "-B", script, "inputs" }, workingDirectory: temp.Path);
        Check(noAnchors.Code != 0 && noAnchors.Error.Contains("anchors", StringComparison.Ordinal), "an index without anchors was accepted");
        Equal(Directory.GetFiles(fixtures).Length, 1, "an index without anchors wrote fixtures");
        index["anchors"] = new JsonObject { ["rule"] = "synthetic rule", ["sha256"] = new string('2', 64) };
        Input("reference/index.json", Encoding.UTF8.GetBytes(index.ToJsonString()));
        var run = Run("python", new[] { "-B", script, "inputs" }, workingDirectory: temp.Path);
        Check(run.Code == 0, "portable fixture generation failed: " + run.Out + run.Error);
        using var result = JsonDocument.Parse(File.ReadAllText(Path.Combine(fixtures, "sb-reference.json")));
        Equal(result.RootElement.GetProperty("sourceCommit").GetString(), new string('1', 40));
        Equal(result.RootElement.GetProperty("anchors").GetProperty("sha256").GetString(), new string('2', 64));
        Equal(result.RootElement.GetProperty("cases").GetArrayLength(), 9);
        foreach (var data in result.RootElement.GetProperty("cases").EnumerateArray())
        {
            var samples = data.GetProperty("samples"); Equal(samples.GetArrayLength(), 300);
            for (int i = 0; i < 300; i++)
            {
                int position = i * 9065 / 299;
                Equal(samples[i].GetProperty("vertex").GetInt32(), position * 7);
                Near(samples[i].GetProperty("reference")[0].GetDouble(), position);
            }
        }
        Sequence(File.ReadAllBytes(Path.Combine(fixtures, "w3-turn.json")), File.ReadAllBytes(FileAt("fixtures/three-cycle-turn.json")));
        Sequence(File.ReadAllBytes(Path.Combine(fixtures, "three-cycle-turn.json")), File.ReadAllBytes(FileAt("fixtures/three-cycle-turn.json")));
        Check(File.ReadAllText(Path.Combine(fixtures, "PROVENANCE.md")).Contains("make_sb_fixture.py <input_folder>", StringComparison.Ordinal), "provenance omits input argument");
    }

    private static void ProjectionKinds()
    {
        double[] q = Geometry.Identity(); Geometry.Rotate(q, 0, 2, .4); Geometry.Rotate(q, 1, 3, -.7);
        var v = new StructureValues(.76, .82, 1.18, 1.15, 1.6);
        Projection stereo = Mesh.Project(67890, v with { Projection = ProjectionKind.Stereographic }, q, Turn, .6);
        Projection perspectiveOne = Mesh.Project(67890, v with { D4 = 1 }, q, Turn, .6);
        Near(stereo.NdcX, perspectiveOne.NdcX); Near(stereo.NdcY, perspectiveOne.NdcY); Near(stereo.W, perspectiveOne.W);
        Projection ortho = Mesh.Project(67890, v with { Projection = ProjectionKind.Orthographic }, q, Turn, .6);
        Projection orthoOtherD = Mesh.Project(67890, v with { Projection = ProjectionKind.Orthographic, D4 = 3 }, q, Turn, .6);
        Equal(ortho, orthoOtherD); Check(Math.Abs(ortho.W - stereo.W) > 1e-6, "projection modes must differ");
        double[] reversed = Geometry.Identity(); Geometry.Rotate(reversed, 1, 3, -.7); Geometry.Rotate(reversed, 0, 2, .4);
        // A coupled plane tests chronological rotate order (the two above commute).
        Geometry.Rotate(q, 2, 3, .3); Geometry.Rotate(reversed, 2, 3, .3);
        Sequence(q, reversed);
        double[] orderA = Geometry.Identity(), orderB = Geometry.Identity();
        Geometry.Rotate(orderA, 0, 3, .6); Geometry.Rotate(orderA, 2, 3, .3);
        Geometry.Rotate(orderB, 2, 3, .3); Geometry.Rotate(orderB, 0, 3, .6);
        Check(orderA.Zip(orderB).Any(p => Math.Abs(p.First - p.Second) > 1e-3), "rotate order lost");
        var preset = Schema.Defaults.Structure(1.6); Near(preset.Cs, .76); Near(preset.Ss, .82); Near(preset.Zoom, 1.15);
        Refuse(() => Mesh.Project(0, v with { Aspect = 0 }, q, Turn, 0), "structure");
        Refuse(() => Mesh.Project(0, v, new double[3], Turn, 0), "Q");
    }

    private static void CellIncidence()
    {
        var s = Mesh.CellStructure; Equal(s.VertexPositions.Length, 120); Equal(s.CellVertices.Length, 600);
        Check(s.VertexPositions.SequenceEqual(s.VertexPositions.OrderBy(v => v)), "engine vertex order changed");
        for (int c = 0; c < 600; c++)
        {
            Equal(s.FaceNeighbors[c].Length, 4); Equal(s.CellVertices[c].Length, 4);
            foreach (int n in s.FaceNeighbors[c]) { Check(s.FaceNeighbors[n].Contains(c), "adjacency not symmetric"); Equal(s.CellVertices[c].Intersect(s.CellVertices[n]).Count(), 3); }
        }
        Check(s.VertexCells.All(v => v.Length == 20 && v.Distinct().Count() == 20), "vertex valence changed");
        for (int v = 0; v < 120; v++) foreach (int c in s.VertexCells[v]) Check(s.CellVertices[c].Contains(v), "incidence not symmetric");
    }

    private static void RingFixture()
    {
        var s = Mesh.CellStructure;
        using var document = JsonDocument.Parse(Read("fixtures/rings.json")); var f = document.RootElement;
        Equal(f.GetProperty("candidateCount").GetInt32(), 240);
        Equal(AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(Root, "assets/model.npz"))), f.GetProperty("modelDigest").GetString());
        Sequence(s.VertexPositions, f.GetProperty("vertexPositions").EnumerateArray().Select(x => x.GetInt32()));
        Sequence(s.RingOf, f.GetProperty("ringOf").EnumerateArray().Select(x => x.GetInt32()));
        Sequence(s.RingGraph, f.GetProperty("graph").EnumerateArray().Select(p => (p[0].GetInt32(), p[1].GetInt32())));
        Equal(s.Rings.Length, 20); Sequence(s.Rings.SelectMany(x => x).OrderBy(c => c), Enumerable.Range(0, 600));
        for (int r = 0; r < 20; r++)
        {
            Sequence(s.Rings[r], f.GetProperty("rings")[r].EnumerateArray().Select(x => x.GetInt32()));
            int[] chain = s.RingChains[r]; Equal(chain.Length, 30); Sequence(chain.OrderBy(c => c), s.Rings[r]);
            for (int i = 0; i < 30; i++) Check(s.FaceNeighbors[chain[i]].Contains(chain[(i + 1) % 30]), "ring chain not closed");
            Equal(s.RingGraph.Count(p => p.A == r || p.B == r), 7);
        }
    }

    private static void RingColouring()
    {
        var s = Mesh.CellStructure;
        for (int k = 4; k <= 8; k++)
        {
            int[] colours = CellStructure.ProperColouring(20, s.RingGraph, k) ?? throw new Exception("colouring failed");
            Check(colours.All(c => c >= 0 && c < k), "class out of range");
            Check(s.RingGraph.All(p => colours[p.A] != colours[p.B]), "ring edge has equal classes");
            Sequence(colours, CellStructure.ProperColouring(20, s.RingGraph, k)!);
        }
        Check(CellStructure.ProperColouring(1, new[] { (0, 0) }, 4) == null, "self loop colouring must fail");
    }

    private static void RingColouringJs()
    {
        var s = Mesh.CellStructure;
        using var result = Node(new { task = "colouring", pairs = s.RingGraph.Select(p => new[] { p.A, p.B }) });
        for (int k = 4; k <= 8; k++) Sequence(CellStructure.ProperColouring(20, s.RingGraph, k)!, result.RootElement[(k - 4)].EnumerateArray().Select(x => x.GetInt32()));
    }

    private static void SlotCells()
    {
        Equal(Geometry.CellOfSlot(0), 0); Equal(Geometry.CellOfSlot(432), 0); Equal(Geometry.CellOfSlot(433), 1); Equal(Geometry.CellOfSlot(259799), 599);
        for (int c = 0; c < 600; c++) { Equal(Geometry.CellOfSlot(c * 433), c); Equal(Geometry.CellOfSlot(c * 433 + 432), c); }
    }

    private static void TurnSnapshots()
    {
        Sequence(Turn.Snapshot(0).ToArray(), Enumerable.Range(0, 259800).Select(i => (uint)i));
        Equal(TurnData.LabelsDigest(Turn.Snapshot(0).Span), Turn.EvenDigest); Equal(TurnData.LabelsDigest(Turn.Snapshot(1).Span), Turn.OddDigest);
        Sequence(Turn.Apply(Turn.Snapshot(0).Span, false), Turn.Snapshot(1).ToArray());
        Sequence(Turn.Apply(Turn.Snapshot(1).Span, true), Turn.Snapshot(0).ToArray());
        Equal(Turn.Snapshot(1).Span.ToArray().Zip(Turn.Snapshot(0).ToArray()).Count(p => p.First != p.Second), 4600);
        uint[] corrupted = Turn.Snapshot(1).ToArray(); (corrupted[0], corrupted[1]) = (corrupted[1], corrupted[0]);
        Refuse(() => Turn.ValidateBinding(1, corrupted), "revision_odd_sha256");
    }

    private static void TurnCycle()
    {
        const string digest = "702e667319e5cbd463baaa823e43d5c16660abbb9019964efa43cd8e5c297c07";
        string path = FileAt("fixtures/three-cycle-turn.json");
        Equal(AssetCatalogue.Digest(File.ReadAllBytes(path)), digest);
        Check(Read("fixtures/PROVENANCE.md").Contains(digest, StringComparison.Ordinal), "synthetic fixture digest absent");
        var cycle = new TurnData(path);
        Near(cycle.Angle, 2 * Math.PI / 3);
        uint[] even = cycle.Snapshot(0).ToArray(), odd = cycle.Apply(even, false), twice = cycle.Apply(odd, false);
        Sequence(odd.Take(3), new uint[] { 2, 0, 1 }); Sequence(twice.Take(3), new uint[] { 1, 2, 0 });
        Check(!twice.SequenceEqual(even), "synthetic fixture must not be an involution");
        Sequence(cycle.Apply(odd, true), even); Sequence(cycle.Apply(cycle.Apply(even, true), false), even);
        Equal(TurnData.LabelsDigest(even), cycle.EvenDigest); Equal(TurnData.LabelsDigest(odd), cycle.OddDigest);
        Sequence(odd.Skip(3), even.Skip(3));
        double now = 0; var clock = new TurnClock(cycle, 190, .25, .25, () => now);
        now = 190; Sequence(clock.Frame().Labels.ToArray(), odd);
        now = 380; Sequence(clock.Frame().Labels.ToArray(), even);
        using var temp = new TestFolder(); JsonNode wrong = JsonNode.Parse(Read("fixtures/three-cycle-turn.json"))!;
        wrong["inverse_src"] = wrong["move_src"]!.DeepClone(); wrong["inverse_dst"] = wrong["move_dst"]!.DeepClone();
        string malformed = Path.Combine(temp.Path, "wrong-inverse.json"); File.WriteAllText(malformed, wrong.ToJsonString());
        Refuse(() => new TurnData(malformed), "inverse");
    }

    private static void TurnBoundaries()
    {
        double now = 1000, duration = 190;
        var clock = new TurnClock(Turn, duration, 1.0 / 3, 1.0 / 3, () => now);
        foreach (var (elapsed, revision) in new[] { (0.0, 0L), (duration - .00001, 0L), (duration, 1L), (2 * duration - .00001, 1L), (2 * duration, 2L) })
        {
            now = 1000 + elapsed;
            TurnFrame frame = clock.Frame(); Equal(frame.TurnIndex, revision); Equal(frame.BoundRevision, revision);
            Check(frame.Phase >= 0 && frame.Phase < 1, "phase outside turn interval");
            Equal(TurnData.LabelsDigest(frame.Labels.Span), revision % 2 == 0 ? Turn.EvenDigest : Turn.OddDigest);
            if (elapsed == duration || elapsed == 2 * duration) { Near(frame.Phase, 0); Near(frame.Theta, 0); }
            else if (elapsed > 0) Near(frame.Theta, revision % 2 == 0 ? Turn.Angle : -Turn.Angle, 1e-10);
        }
        Sequence(clock.Frame().Labels.ToArray(), Turn.Snapshot(0).ToArray());
    }

    private static void TurnMidpoint()
    {
        double now = 0; var clock = new TurnClock(Turn, 190, .25, .25, () => now);
        now = 95; TurnFrame frame = clock.Frame(); Near(frame.Phase, .5); Near(frame.Theta, Turn.Angle / 2, 1e-11);
        now = 285; frame = clock.Frame(); Near(frame.Theta, -Turn.Angle / 2, 1e-11); Equal(frame.BoundRevision, 1L);
        Refuse(() => clock.Frame(_ => Turn.Snapshot(0)), "revision_odd_sha256");
        now = 380; Sequence(clock.Frame().Labels.ToArray(), Turn.Snapshot(0).ToArray());
        now = -1; Refuse(() => clock.Frame(), "clock");
        using var temp = new TestFolder(); JsonNode malformed = JsonNode.Parse(Read("fixtures/w3-turn.json"))!;
        malformed["move_dst"]![0] = malformed["move_dst"]![1]!.GetValue<int>();
        string path = Path.Combine(temp.Path, "turn.json"); File.WriteAllText(path, malformed.ToJsonString());
        Refuse(() => new TurnData(path), "move_dst");
    }

    private static void TurnFractionalBoundary()
    {
        double now = 0; const double duration = 333.3;
        var clock = new TurnClock(Turn, duration, .25, .25, () => now);
        foreach (var (elapsed, revision) in new[] { (999.9, 3L), (2 * duration, 2L) })
        {
            now = elapsed; TurnFrame frame = clock.Frame();
            Equal(frame.TurnIndex, revision); Equal(frame.BoundRevision, revision);
            Check(frame.Phase >= 0 && frame.Phase < 1, "fractional-duration phase outside [0,1)");
            Equal(frame.Phase, 0.0); Equal(frame.Theta, 0.0);
            Equal(TurnData.LabelsDigest(frame.Labels.Span), revision % 2 == 0 ? Turn.EvenDigest : Turn.OddDigest);
        }
        now = Math.BitDecrement(999.9); TurnFrame before = clock.Frame();
        Equal(before.TurnIndex, 2L); Check(before.Phase >= 0 && before.Phase < 1 && before.Phase > .999, "phase before rounded boundary inconsistent");
    }
}
