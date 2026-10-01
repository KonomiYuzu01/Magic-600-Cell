// Root-operated file/confirmation checks; file picker gestures are not simulated.
using System;using System.Collections.Generic;using System.IO;using System.Linq;using System.Reflection;using System.Runtime.InteropServices;using System.Text;using System.Threading.Tasks;using System.Web.Script.Serialization;using System.Windows.Forms;
internal static class KeymapFileNativeChecks {
 static readonly JavaScriptSerializer json=new JavaScriptSerializer{MaxJsonLength=67108864};
 static Dictionary<string,object> Map(object value){return value as Dictionary<string,object>;}
 static object Ordered(object value){var map=value as IDictionary<string,object>;if(map!=null){var sorted=new SortedDictionary<string,object>(StringComparer.Ordinal);foreach(var pair in map)sorted[pair.Key]=Ordered(pair.Value);return sorted;}var items=value as IEnumerable<object>;return items==null?value:items.Select(Ordered).ToArray();}
 static string Json(object value){return json.Serialize(Ordered(value));}
 static Dictionary<string,object> Copy(object value){return Map(json.DeserializeObject(json.Serialize(value)));}
 static Dictionary<string,object> Workspace(ExperimentShell shell){return Map(shell.Work["workspace"]);}
 static string Stable(ExperimentShell shell){var w=Copy(Workspace(shell));w.Remove("keybinds");return Json(new object[]{shell.Work["hash"],w,shell.Work["protected"],shell.Work["pending"]});}
 static string Bindings(ExperimentShell shell){return Json(Workspace(shell)["keybinds"]);}
 static IEnumerable<Control> Children(Control parent){foreach(Control c in parent.Controls){yield return c;foreach(Control nested in Children(c))yield return nested;}}
 [DllImport("user32.dll")]static extern IntPtr GetForegroundWindow();
 static string Context(ExperimentShell shell,Form host){var feedback=(Label)typeof(ExperimentShell).GetField("feedback",BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);return "ActiveForm="+(Form.ActiveForm==null?"null":Form.ActiveForm.Text)+" foreground=0x"+GetForegroundWindow().ToInt64().ToString("x")+" host=0x"+host.Handle.ToInt64().ToString("x")+" feedback="+feedback.Text;}
 static async Task<Tuple<Form,Task<bool>>> Open(ExperimentShell shell,Form host,string path){
  var activation=new TaskCompletionSource<bool>();host.BeginInvoke((Action)delegate{host.Activate();activation.SetResult(true);});await activation.Task;var activationDeadline=DateTime.UtcNow.AddSeconds(30);
  while(Form.ActiveForm!=host||GetForegroundWindow()!=host.Handle){if(DateTime.UtcNow>=activationDeadline)throw new TimeoutException("Keymap host activation precondition failed. "+Context(shell,host));await Task.Delay(30);}
  var started=new TaskCompletionSource<Task<bool>>();host.BeginInvoke((Action)delegate{if(Form.ActiveForm!=host||GetForegroundWindow()!=host.Handle){started.SetException(new InvalidOperationException("Keymap host lost focus before inspection. "+Context(shell,host)));return;}Console.WriteLine("KEYMAP BEFORE "+Context(shell,host));Console.Out.Flush();started.SetResult(shell.ShowKeymapFileImport(path));});var operation=await started.Task;var until=DateTime.UtcNow.AddSeconds(60);
  while(DateTime.UtcNow<until){var dialog=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text=="Import keybindings"&&Form.ActiveForm==f);if(dialog!=null)return Tuple.Create(dialog,operation);if(operation.IsCompleted)throw new InvalidOperationException("Keymap inspection finished without its confirmation dialog. "+Context(shell,host));await Task.Delay(30);}throw new TimeoutException("Keymap confirmation did not activate. "+Context(shell,host));
 }
 static async Task<bool> Apply(Tuple<Form,Task<bool>> opened,Func<Task> ready){
  Children(opened.Item1).OfType<Button>().Single(b=>b.AccessibleName=="Apply inspected keymap").PerformClick();await ready();if(opened.Item1.IsDisposed||!opened.Item1.Visible||opened.Item2.IsCompleted)return await opened.Item2;return false;
 }
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image,string output){
  await ready();string stable=Stable(shell),original=Bindings(shell);string saved=Path.Combine(output,"keymap-original.json"),candidate=Path.Combine(output,"keymap-candidate.json"),invalid=Path.Combine(output,"keymap-invalid.json"),exported=Path.Combine(output,"keymap-after.json");
  check(await shell.ExportKeymapFile(saved)&&File.Exists(saved),"Export keymap writes a real versioned file");await ready();
  var document=Map(json.DeserializeObject(File.ReadAllText(saved,Encoding.UTF8)));check(Convert.ToString(document["scope"])=="keybindings-only"&&Json(document["bindings"])==original&&Stable(shell)==stable,"Export retains exact overrides and leaves all solve state unchanged");
  var edited=Copy(document);var bindings=Map(edited["bindings"]);var commands=Map(bindings.ContainsKey("commands")?bindings["commands"]:null);if(commands==null){commands=new Dictionary<string,object>();bindings["commands"]=commands;}commands["Control+Alt+KeyU"]="undo";File.WriteAllText(candidate,json.Serialize(edited),new UTF8Encoding(false));
  var cancelled=await Open(shell,host,candidate);check(Children(cancelled.Item1).OfType<Label>().Any(l=>l.AccessibleName=="Keymap replacement scope"&&l.Text.Contains("Replace all saved bindings")),"Import states the whole override replacement before Apply");image("keymap-import-preview",cancelled.Item1);((IButtonControl)cancelled.Item1.CancelButton).PerformClick();check(!await cancelled.Item2&&Bindings(shell)==original&&Stable(shell)==stable,"Cancel file import preserves bindings, Current, Next, drafts and labels");
  var bad=Copy(edited);bad["model"]="wrong-model";File.WriteAllText(invalid,json.Serialize(bad),new UTF8Encoding(false));check(!await shell.ShowKeymapFileImport(invalid)&&Bindings(shell)==original&&Stable(shell)==stable,"Wrong-model file is rejected without partial application");await ready();
  var stale=await Open(shell,host,candidate);var concurrent=Copy(Workspace(shell)["keybinds"]);var shared=Map(concurrent.ContainsKey("commands")?concurrent["commands"]:null);if(shared==null){shared=new Dictionary<string,object>();concurrent["commands"]=shared;}shared["Control+Alt+KeyJ"]="redo";
  if(!await shell.Send(LocalApi.D("action","settings","keybinds",concurrent)))throw new InvalidOperationException("Fixture concurrent binding edit was rejected.");await ready();await Apply(stale,ready);
  check(stale.Item1.Visible&&!Children(stale.Item1).OfType<Button>().Single(b=>b.AccessibleName=="Apply inspected keymap").Enabled&&Bindings(shell)==Json(concurrent)&&Stable(shell)==stable,"A real binding edit after inspection rejects stale replacement without losing that edit");((IButtonControl)stale.Item1.CancelButton).PerformClick();await stale.Item2;
  var applied=await Open(shell,host,candidate);check(await Apply(applied,ready)&&Bindings(shell)==Json(bindings)&&Stable(shell)==stable,"Apply replaces exactly the inspected saved overrides while preserving all solve state");
  check(await shell.ExportKeymapFile(exported)&&Json(Map(json.DeserializeObject(File.ReadAllText(exported,Encoding.UTF8)))["bindings"])==Bindings(shell),"Applied bindings survive actual file save and reload");await ready();
  var restore=await Open(shell,host,saved);check(await Apply(restore,ready)&&Bindings(shell)==original&&Stable(shell)==stable,"Importing the original file restores exact prior overrides without changing work");
 }
}
