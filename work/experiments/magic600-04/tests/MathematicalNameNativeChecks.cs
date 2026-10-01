// Real native editors over a fresh isolated Session; not a human trial.
using System;using System.Collections.Generic;using System.Linq;using System.Reflection;using System.Threading.Tasks;using System.Web.Script.Serialization;using System.Windows.Forms;
internal static class MathematicalNameNativeChecks {
 [System.Runtime.InteropServices.DllImport("user32.dll")] static extern IntPtr GetOpenClipboardWindow();
 [System.Runtime.InteropServices.DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
 static void ClipboardState(string phase){uint process;var owner=GetOpenClipboardWindow();GetWindowThreadProcessId(owner,out process);Console.WriteLine("Clipboard "+phase+": open-owner="+owner+" process="+process+" thread="+System.Threading.Thread.CurrentThread.ManagedThreadId+" apartment="+System.Threading.Thread.CurrentThread.GetApartmentState()+" foreground="+(Form.ActiveForm==null?"none":Form.ActiveForm.GetType().Name));}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static IEnumerable<Control> Children(Control c){foreach(Control child in c.Controls){yield return child;foreach(var next in Children(child))yield return next;}}
 static void Invoke(ExperimentShell shell,string id){typeof(ExperimentShell).GetMethod("RunCommand",BindingFlags.NonPublic|BindingFlags.Instance).Invoke(shell,new object[]{id});}
 static string Context(ExperimentShell shell){var w=Map(shell.Work["workspace"]);return new JavaScriptSerializer().Serialize(new object[]{shell.Work["hash"],w["current"],w["target"],w["next"],w["draft"],w["roles"],shell.Work["protected"]});}
 static async Task<Form> Open(ExperimentShell shell,Form host){host.BeginInvoke((Action)delegate{Invoke(shell,"focus");});var deadline=DateTime.UtcNow.AddSeconds(20);while(DateTime.UtcNow<deadline){var dialog=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text=="Current identity and destination");if(dialog!=null)return dialog;await Task.Delay(25);}throw new TimeoutException("Mathematical address editor did not open");}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check){
  await ready();var w=Map(shell.Work["workspace"]);int identity=Convert.ToInt32(w["current"]),position=Convert.ToInt32(w["target"]);string before=Context(shell);
  ClipboardState("before snapshot");var savedClipboard=NativeClipboardSnapshot.Capture();Exception failure=null;ClipboardState("after snapshot");
  try{
   var next=Map(w["next"]);int nextIdentity=Convert.ToInt32(next["identity"]);
   check(await shell.Send(LocalApi.D("action","inspect","identity",nextIdentity)),"Inspect the legal fixture identity with an inverse in its Home address");await ready();Invoke(shell,"copy-selection");string inverseName=Clipboard.GetText();
   check(inverseName.Contains("T⁻¹")&&!inverseName.Contains("T^-1"),"Native mathematical copy emits a real superscript inverse");
   check(await shell.Send(LocalApi.D("action","inspect","identity",identity)),"Inspect canonical piece before copying its mathematical identity");await ready();Invoke(shell,"copy-selection");string piece=Clipboard.GetText();check(piece.StartsWith("Magic600.Piece|"),"Native Copy preserves explicit Piece type, model and naming version");
   Invoke(shell,"copy-selection-canonical");check(Clipboard.GetText()=="I"+identity,"Canonical piece copy remains available");
   check(await shell.Send(LocalApi.D("action","inspect-position","position",position)),"Inspect fixed position before copying its address");await ready();Invoke(shell,"copy-selection");string target=Clipboard.GetText();check(target.StartsWith("Magic600.Position|"),"Native Copy distinguishes a fixed Position from its occupant");
   var dialog=await Open(shell,host);var fields=Children(dialog).OfType<TextBox>().OrderBy(c=>c.Top).ToArray();check(fields.Length==2,"Current editor exposes separate identity and destination inputs");fields[0].Text=piece;fields[1].Text=piece;
   Children(dialog).OfType<Button>().Single(b=>b.Text=="Apply explicitly").PerformClick();await ready();
   check(dialog.Visible&&fields[0].Enabled&&fields[1].Text==piece,"Wrong address type is rejected without closing or discarding typed input");check(Context(shell)==before,"Wrong address type cannot change mechanics, target, Next or draft");
   fields[1].Text=target;Children(dialog).OfType<Button>().Single(b=>b.Text=="Apply explicitly").PerformClick();await ready();
   check(dialog.IsDisposed||!dialog.Visible,"Correct Piece and Position addresses apply through the existing editor");check(Context(shell)==before,"Mathematical copy/input round trip resolves the same canonical working objects");
   dialog=await Open(shell,host);fields=Children(dialog).OfType<TextBox>().OrderBy(c=>c.Top).ToArray();fields[0].Text=piece.Replace("Magic600.Piece|","Magic600.Position|");((IButtonControl)dialog.CancelButton).PerformClick();await ready();check(Context(shell)==before,"Cancel mathematical input preserves the working context");
  }catch(Exception error){ClipboardState("failure");failure=error;Console.WriteLine("Name roundtrip primary failure: "+error);}
  try{await savedClipboard.RestoreAsync();check(true,"Original clipboard formats and contents restored (contents not logged)");}catch(Exception restore){if(failure!=null)throw new AggregateException("Name roundtrip and clipboard restoration failed",failure,restore);throw;}
  savedClipboard.Dispose();
  if(failure!=null)throw failure;
 }
}

// Test-only detached backup. Never retain a live OLE data object across the test.
internal sealed class NativeClipboardSnapshot:IDisposable {
 [System.Runtime.InteropServices.DllImport("user32.dll")] static extern uint GetClipboardSequenceNumber();
 [System.Runtime.InteropServices.DllImport("user32.dll")] static extern IntPtr GetOpenClipboardWindow();
 [System.Runtime.InteropServices.DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
 readonly DataObject data=new DataObject();
 readonly Dictionary<string,object> values=new Dictionary<string,object>();
 static void RequireSta(){if(System.Threading.Thread.CurrentThread.GetApartmentState()!=System.Threading.ApartmentState.STA)throw new InvalidOperationException("Clipboard preservation requires STA");}
 internal static NativeClipboardSnapshot Capture(){RequireSta();uint sequence=GetClipboardSequenceNumber();var saved=FromData(Clipboard.GetDataObject());if(sequence!=GetClipboardSequenceNumber()){saved.Dispose();throw new InvalidOperationException("Clipboard changed during backup; test has not modified it");}return saved;}
 internal static NativeClipboardSnapshot FromData(IDataObject source){var saved=new NativeClipboardSnapshot();try{if(source!=null)foreach(var format in source.GetFormats(false)){object value=CopyValue(source.GetData(format,false));saved.values.Add(format,value);saved.data.SetData(format,false,value);}return saved;}catch{saved.Dispose();throw;}}
 static object CopyValue(object value){
  if(value is string)return value;
  if(value is byte[])return ((byte[])value).Clone();
  if(value is string[])return ((string[])value).Clone();
  var stream=value as System.IO.Stream;if(stream!=null){if(!stream.CanSeek)throw new NotSupportedException("Cannot preserve a nonseekable clipboard stream; test has not modified it");long position=stream.Position;try{stream.Position=0;var copy=new System.IO.MemoryStream();stream.CopyTo(copy);copy.Position=0;return copy;}finally{stream.Position=position;}}
  var image=value as System.Drawing.Image;if(image!=null)return image.Clone();
  if(value!=null&&value.GetType().IsPrimitive)return value;
  throw new NotSupportedException("Clipboard contains an unsupported or unavailable format; test has not modified it");
 }
 static bool EqualImage(System.Drawing.Image first,System.Drawing.Image second){if(first.Width!=second.Width||first.Height!=second.Height)return false;using(var a=new System.Drawing.Bitmap(first))using(var b=new System.Drawing.Bitmap(second)){for(int y=0;y<a.Height;y++)for(int x=0;x<a.Width;x++)if(a.GetPixel(x,y).ToArgb()!=b.GetPixel(x,y).ToArgb())return false;}return true;}
 static bool EqualValue(object expected,object actual){
  if(expected is byte[])return actual is byte[]&&((byte[])expected).SequenceEqual((byte[])actual);
  if(expected is string[])return actual is string[]&&((string[])expected).SequenceEqual((string[])actual);
  if(expected is System.IO.MemoryStream){var copy=CopyValue(actual) as System.IO.MemoryStream;if(copy==null)return false;using(copy)return ((System.IO.MemoryStream)expected).ToArray().SequenceEqual(copy.ToArray());}
  if(expected is System.Drawing.Image)return actual is System.Drawing.Image&&EqualImage((System.Drawing.Image)expected,(System.Drawing.Image)actual);
  return Object.Equals(expected,actual);
 }
 internal void Verify(){var restored=Clipboard.GetDataObject();var formats=restored==null?new string[0]:restored.GetFormats(false);if(values.Count==0&&formats.Length!=0)throw new InvalidOperationException("Empty clipboard was not restored");int index=0;foreach(var entry in values){object actual=formats.Contains(entry.Key)?restored.GetData(entry.Key,false):null;if(!EqualValue(entry.Value,actual))throw new InvalidOperationException("Clipboard restoration verification failed at format index "+index+" expected-type="+entry.Value.GetType().Name+" actual-type="+(actual==null?"null":actual.GetType().Name)+"; contents withheld");index++;}}
 internal async Task RestoreAsync(int timeoutMilliseconds=10000){RequireSta();var clock=System.Diagnostics.Stopwatch.StartNew();int retries=0;while(true){try{RequireSta();Clipboard.SetDataObject(data,true,0,0);Verify();Console.WriteLine("Clipboard restore verified: formats="+values.Count+" retries="+retries+" elapsed-ms="+clock.ElapsedMilliseconds);return;}catch(System.Runtime.InteropServices.ExternalException error){if(error.ErrorCode!=unchecked((int)0x800401D0))throw;uint process;var owner=GetOpenClipboardWindow();GetWindowThreadProcessId(owner,out process);if(retries==0||clock.ElapsedMilliseconds>=timeoutMilliseconds)Console.WriteLine("Clipboard restore busy: owner="+owner+" process="+process+" hresult=800401D0 elapsed-ms="+clock.ElapsedMilliseconds);if(clock.ElapsedMilliseconds>=timeoutMilliseconds)throw;retries++;}await Task.Delay(100);}}
 public void Dispose(){foreach(var value in values.Values){var resource=value as IDisposable;if(resource!=null)resource.Dispose();}values.Clear();}
}
