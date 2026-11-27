using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private const string DocumentTestModel = "600-cell-Full sample";

    private static DocumentSet DocumentTestSet(MemoryDocumentStore store) =>
        new(store, new SampleDocumentCompiler(DocumentTestModel, Array.Empty<SolvingState>()));

    private static void D02PortDocumentsSessionTab()
    {
        var store = new MemoryDocumentStore();
        DocumentSet set = DocumentTestSet(store);
        Document session = set.Active;
        Equal(session.Id, "session");
        Equal(session.Name, "Session");
        Equal(session.Kind, DocumentKind.Session);
        Check(!session.Dirty, "the session has a dirty mark");
        Check(set.Edit(session.Id, "edit") != null && session.Text == "", "the session can be edited");
        Check(set.Save(session.Id)?.Contains("saved at exit without a prompt", StringComparison.Ordinal) == true,
            "Save does not explain the session's exit save");
        Check(set.SaveAs(session.Id, "session.sample-log") != null && store.Writes == 0, "Save as wrote the session");
        Check(set.CompileDocument(session.Id) != null && session.Compile == null, "the session was compiled");
        Check(set.Close(session.Id, true) != null && set.Pin(session.Id) != null, "the session can close or be pinned");

        DocumentSignature blank = session.Signature(null, false);
        Check(blank.Recording && blank.Ticks == null && blank.Lamp == null, "the session without a live state has the wrong marks");
        SolvingState live = new SolvingDemo(Atlas).States[2];
        DocumentSignature signature = session.Signature(live, false);
        Check(signature.Recording && signature.Lamp == null, "the live signature lost its recording mark");
        Sequence(signature.Ticks!, OrbitRailModel.Cells(live, false), "the session does not use the live rail");
        Sequence(set.Documents, new[] { session });
    }

    private static void D02PortDocumentsDirtyAndSave()
    {
        var store = new MemoryDocumentStore();
        DocumentSet set = DocumentTestSet(store);
        Document script = set.New(DocumentKind.Script);
        Equal(script.Name, "Untitled script 1");
        Check(script.Path == null && script.SavedDigest == null && !script.Dirty, "an empty untitled document is dirty or saved");
        Equal(script.PathMark, "never saved");
        const string source = "edition sample\r\n# π\r\n";
        Equal(set.Edit(script.Id, source), null);
        Check(script.Dirty && set.Close(script.Id, false) != null, "closing lost an unsaved buffer");
        Check(set.Save(script.Id)?.Contains("Save as", StringComparison.Ordinal) == true && store.Writes == 0, "Save chose a path");

        Equal(set.SaveAs(script.Id, "folder/draft.6cs"), null);
        Equal(script.Path, "folder/draft.6cs");
        Equal(script.Name, "draft.6cs");
        Equal(script.PathMark, null);
        byte[] bytes = Encoding.UTF8.GetBytes(source);
        string digest = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
        Equal(script.Digest, digest, "digest of the exact UTF-8 bytes");
        Equal(script.SavedDigest, digest);
        Sequence(store.Read(script.Path!)!, bytes);
        Check(!script.Dirty && store.Writes == 1, "Save as did not leave a clean buffer and one write");

        Equal(set.Edit(script.Id, source + "# next"), null);
        Check(script.Dirty, "edit of a saved file is clean");
        Equal(set.Save(script.Id), null);
        Check(!script.Dirty && store.Writes == 2, "Save did not update the saved digest");
        Equal(set.Close(script.Id, false), null);
        Document reopened = set.Open("folder/draft.6cs")!;
        Equal(reopened.Text, source + "# next");
        Check(!reopened.Dirty && reopened.Compile == null, "Open compiled or dirtied a script");
        Equal(set.Open("FOLDER/DRAFT.6CS"), reopened, "a second Open duplicated the tab");
        Equal(set.Open("missing.6cs"), null);
        Equal(set.Active, reopened);

        Document discarded = set.New(DocumentKind.Script);
        Equal(set.Edit(discarded.Id, "dirty"), null);
        Check(set.SaveAs(discarded.Id, "folder/draft.6cs") != null && store.Writes == 2, "Save as overwrote another open file");
        Equal(set.Close(discarded.Id, true), null);
        Equal(set.Active, reopened);
    }

    private static void D02PortDocumentsLampAndStaleProblems()
    {
        var store = new MemoryDocumentStore();
        DocumentSet set = DocumentTestSet(store);
        Document script = set.New(DocumentKind.Script);
        Equal(script.Lamp.Word(), "not compiled");
        Equal(script.Signature(null, true), new DocumentSignature(null, CompileLamp.NotCompiled, false, "not compiled"));
        Equal(set.Edit(script.Id, "edition sample\n}"), null);
        Equal(set.CompileDocument(script.Id), null);
        CompileResult first = script.Compile!;
        Equal(first.Number, 1);
        Equal(first.Digest, script.Digest);
        Equal(script.Lamp.Word(), "compiled with problems");
        DocumentProblem row = set.Problems().Single();
        Check(row.DocumentId == script.Id && row.Number == 1 && !row.Dimmed && row.Text.Contains("closes nothing", StringComparison.Ordinal),
            "the current compile has no current diagnostic");

        Equal(set.Edit(script.Id, "edition sample\nlet sample;"), null); // A warning, with no error.
        Equal(script.Lamp, CompileLamp.NotCompiled);
        DocumentProblem stale = set.Problems().Single();
        Check(stale.Dimmed && stale.Number == 1 && stale.Text.Contains("compile 1", StringComparison.Ordinal), "the stale problem lost its compile number");
        Equal(set.CompileDocument(script.Id), null);
        Equal(script.Compile!.Number, 2);
        Equal(script.Lamp.Word(), "compiled clean");
        Check(set.Problems().Count == 0 && script.Dirty && script.Path == null && store.Writes == 0,
            "compiling changed something outside the compile result");
    }

    private static void D02PortDocumentsVerifiedLogUsesRail()
    {
        var demo = new SolvingDemo(Atlas);
        MemoryDocumentStore store = SampleDocuments.Store(DocumentTestModel, demo.States, new Dictionary<string, string> { ["example.6cs"] = "edition sample" });
        var set = new DocumentSet(store, new SampleDocumentCompiler(DocumentTestModel, demo.States));
        int writes = store.Writes;
        string hashInput = demo.States[0].Id + "\nmove 1\nmove 2";
        using JsonDocument sample = JsonDocument.Parse(SampleDocuments.LogText(DocumentTestModel, demo.States[0], new[] { "move 1", "move 2" }, "02:00:00"));
        Equal(sample.RootElement.GetProperty("hash").GetString(), Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(hashInput))).ToLowerInvariant());
        Equal(set.Open("example.6cs")!.Text, "edition sample");

        Document mid = set.Open(SampleDocuments.MidLogPath)!;
        Document endgame = set.Open(SampleDocuments.EndgameLogPath)!;
        CompileResult midCompile = mid.Compile!;
        Equal(mid.Kind, DocumentKind.Log);
        Check(mid.Verified && !mid.ReadOnly && !mid.Dirty && midCompile.Passed, "the sample log did not verify on open");
        Equal(midCompile.Number, 1);
        Equal(endgame.Compile!.Number, 2);
        Sequence(midCompile.Gates.Select(g => g.Name), new[] { "format", "model_match", "moves", "final_hash", "scripts" });
        Check(midCompile.Gates[2].Detail.Contains("legal witness", StringComparison.Ordinal), "the sample does not name LP4's replay check");
        Equal(mid.Binding, "saved");
        DocumentSignature signature = mid.Signature(demo.States[2], false);
        Check(signature.Lamp == null && !signature.Recording, "a verified log has the wrong marks");
        Sequence(signature.Ticks!, OrbitRailModel.Cells(demo.States[0], false), "the log does not use the rail of its saved state");
        Equal(signature.Tip, $"2 moves · 00:12:34 · {DocumentTestModel}");
        Sequence(endgame.Signature(null, true).Ticks!, OrbitRailModel.Cells(demo.States[1], true));
        Check(endgame.Signature(null, true).Ticks!.Any(cell => cell.Solved && cell.Level == 0), "the signature lost the solved mark");

        Document bad = set.Open(SampleDocuments.BadLogPath)!;
        Check(bad.ReadOnly && !bad.Verified && bad.Compile!.Facts == null && bad.Binding == null,
            "a log with a bad hash exposed a saved state");
        Equal(bad.Reason, bad.Compile!.Gates.Single(g => g.Name == "final_hash").Detail);
        Equal(bad.Lamp, CompileLamp.Problems);
        Equal(bad.Signature(demo.States[0], true).Lamp, CompileLamp.Problems);
        Check(bad.Signature(demo.States[0], true).Ticks == null && set.Edit(bad.Id, mid.Text) != null, "the failed log shows ticks or accepts edits");
        Equal(set.Problems().Single().Text, bad.Reason);
        Equal(store.Writes, writes);

        Equal(set.Edit(mid.Id, mid.Text + "\n"), null);
        Check(mid.Dirty && mid.ReadOnly && mid.Binding == null && mid.Reason == "not compiled yet", "an edit kept the verified binding");
        DocumentSignature changed = mid.Signature(null, false);
        Check(changed.Ticks == null && changed.Lamp == CompileLamp.NotCompiled, "an edited log kept its signature");
        Equal(set.CompileDocument(mid.Id), null);
        Check(mid.Verified && mid.Dirty && mid.Compile!.Number == 4, "compile of current log bytes did not restore verification");

        var compiler = new SampleDocumentCompiler(DocumentTestModel, demo.States);
        CompileResult malformed = compiler.Log(5, "", "{");
        Check(!malformed.Passed && malformed.Gates[0].Name == "format" && malformed.Facts == null, "malformed JSON passed the format gate");
        CompileResult wrongModel = compiler.Log(6, "", SampleDocuments.LogText("other model", demo.States[0], Array.Empty<string>(), "00:00:00"));
        Check(!wrongModel.Passed && !wrongModel.Gates.Single(g => g.Name == "model_match").Passed && wrongModel.Facts == null,
            "a different model identity exposed a saved state");
    }

    private static void D02PortDocumentsOverflowAndSearch()
    {
        var store = new MemoryDocumentStore();
        store.Write("examples/lesson.6cs", Encoding.UTF8.GetBytes("edition sample"));
        DocumentSet set = DocumentTestSet(store);
        Document session = set.Active;
        Document first = set.New(DocumentKind.Log);
        Document second = set.Open("examples/lesson.6cs")!;
        Document third = set.New(DocumentKind.Script);
        Equal(first.Reason, "not compiled yet");
        Equal(set.Pin(third.Id), null);
        Equal(set.Pin(second.Id), null);
        Sequence(set.Documents, new[] { session, second, third, first }, "pinned tabs are not in open order");
        Sequence(set.Visible(2), new[] { session, second });
        Sequence(set.Overflow(2), new[] { third, first });
        Sequence(set.Visible(0), new[] { session });
        Sequence(set.Search("EXAMPLES/"), new[] { second });
        Sequence(set.Search("UNTITLED"), new[] { third, first });
        Equal(set.Pin(second.Id), null);
        Sequence(set.Documents, new[] { session, third, first, second });
    }

    private static void D02PortDocumentsInventoryKeepsBothVersions()
    {
        var demo = new SolvingDemo(Atlas);
        MemoryDocumentStore store = SampleDocuments.Store(DocumentTestModel, demo.States, new Dictionary<string, string>());
        var compiler = new SampleDocumentCompiler(DocumentTestModel, demo.States);
        var set = new DocumentSet(store, compiler);
        Document script = set.New(DocumentKind.Script);
        Equal(set.Edit(script.Id, "edition saved"), null);
        Equal(set.SaveAs(script.Id, "draft.6cs"), null);
        Equal(set.Edit(script.Id, "edition at exit"), null);
        Document log = set.Open(SampleDocuments.MidLogPath)!;
        Equal(set.Pin(log.Id), null);
        Document untitled = set.New(DocumentKind.Script);
        Equal(set.Edit(untitled.Id, "# never saved"), null);
        string inventory = set.ExitInventory();
        using JsonDocument json = JsonDocument.Parse(inventory);
        Equal(json.RootElement.GetProperty("format").GetString(), "magic600-document-inventory/1");
        Equal(json.RootElement.GetProperty("active").GetString(), untitled.Id);
        JsonElement entries = json.RootElement.GetProperty("documents");
        Equal(entries.GetArrayLength(), 3);
        Equal(entries[0].GetProperty("saved_digest").GetString(), script.SavedDigest);
        Equal(entries[2].GetProperty("path").ValueKind, JsonValueKind.Null);
        Equal(entries[2].GetProperty("mark").GetString(), "never saved");

        store.Write("draft.6cs", Encoding.UTF8.GetBytes("edition on disk"));
        store.Write(SampleDocuments.MidLogPath, Encoding.UTF8.GetBytes("changed externally"));
        int writes = store.Writes;
        DocumentSet restored = DocumentSet.Restore(inventory, store, compiler);
        Sequence(restored.Documents.Select(doc => doc.Id), set.Documents.Select(doc => doc.Id));
        Equal(restored.Active.Id, untitled.Id);
        Check(restored.Active.Dirty && restored.Active.Path == null && restored.Active.Compile == null, "the untitled buffer did not survive");
        Equal(restored.Active.Text, untitled.Text);

        Document recovered = restored.Documents.Single(doc => doc.Id == script.Id);
        Equal(recovered.Text, "edition at exit");
        Equal(recovered.DiskText, "edition on disk");
        Equal(recovered.SavedDigest, script.SavedDigest);
        Check(recovered.Dirty && recovered.Signature(null, false).Tip.Contains("changed on disk since; both versions kept", StringComparison.Ordinal),
            "the recovered dirty document did not report the disk change");
        Document recoveredLog = restored.Documents.Single(doc => doc.Id == log.Id);
        Check(recoveredLog.Verified && !recoveredLog.Dirty && recoveredLog.Pinned, "the recovered log used the disk version or lost its pin");
        Equal(recoveredLog.DiskText, "changed externally");
        Sequence(recoveredLog.Signature(null, false).Ticks!, OrbitRailModel.Cells(demo.States[0], false));
        Equal(restored.ExitInventory(), inventory, "restore changed the exit contents");
        Equal(restored.New(DocumentKind.Log).Name, "Untitled log 3");
        Equal(store.Writes, writes, "restore wrote a document file");
        Equal(Encoding.UTF8.GetString(store.Read("draft.6cs")!), "edition on disk");
    }

    private static void D02PortDocumentsCommands()
    {
        const string prefix = "file.document.";
        var store = new MemoryDocumentStore();
        DocumentSet set = DocumentTestSet(store);
        CommandRegistry registry = LabRegistry();
        foreach ((string suffix, string label) in new[] {
            ("new", "New document"), ("open", "Open document"), ("save", "Save document"),
            ("save_as", "Save document as"), ("compile", "Compile document"), ("close", "Close document"),
        })
        {
            string id = prefix + suffix;
            Check(DocumentSet.Handles(id), "missing document handler " + id);
            Check(registry.Commands.TryGetValue(id, out ChromeCommand? row) && row.Label == label, "missing command table row " + id);
        }
        Check(!DocumentSet.Handles("file.log.import"), "document commands claimed log import");
        Check(set.Unavailable(prefix + "new", "session") != null && set.Unavailable(prefix + "open", null) != null, "an argument is missing");
        Check(set.Unavailable(prefix + "save", null) != null && set.Unavailable(prefix + "close", "discard") != null, "a session command is available");
        Equal(set.Execute(prefix + "new", "log"), "opened Untitled log 1");
        Equal(set.Active.Kind, DocumentKind.Log);
        Equal(set.Execute(prefix + "close", null), "closed Untitled log 1");
        Equal(set.Execute(prefix + "new", "script"), "opened Untitled script 2");
        Document script = set.Active;
        Equal(set.Edit(script.Id, "edition sample"), null);
        Check(set.Unavailable(prefix + "save_as", " ") != null && set.Unavailable(prefix + "save", null) != null, "Save is available without a path");
        Check(set.Unavailable(prefix + "close", null) != null && set.Unavailable(prefix + "close", "discard") == null,
            "close did not require discarding dirty changes");
        Equal(set.Execute(prefix + "compile", null), "compiled clean");
        Equal(set.Execute(prefix + "save_as", "command.6cs"), "saved command.6cs");
        Equal(set.Execute(prefix + "save", null), "saved command.6cs");
        Equal(set.Execute(prefix + "close", null), "closed command.6cs");
        Equal(set.Execute(prefix + "open", "missing.6cs"), "file missing · missing.6cs");
        Equal(set.Execute(prefix + "open", "command.6cs"), "opened command.6cs");
        Equal(set.Edit(set.Active.Id, "# discard this"), null);
        Equal(set.Execute(prefix + "close", "discard"), "closed command.6cs");
        Check(set.Active.Id == "session" && store.Writes == 2, "the command sequence lost the session or wrote on close");
    }
}
