// Agent-operated actual native controls and model; not physical typing or human trials.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class PostApprovalNativeRegression {
 static ExperimentShell shell;static Form form;static string output,mode;static int exitCode=1;
 static readonly List<string> checks=new List<string>();
 static readonly Dictionary<string,object> report=new Dictionary<string,object>();
 static readonly List<object> commandTimings=new List<object>();
 static Exception uiFailure;
 static Dictionary<string,object> Map(object o){return (Dictionary<string,object>)o;}
 static object[] Items(object o){return (object[])o;}
 static T Field<T>(string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static void Check(bool ok,string text){if(!ok)throw new InvalidOperationException(text);checks.Add(text);Console.WriteLine("PASS "+text);Console.Out.Flush();}
 [System.Runtime.InteropServices.DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
 static void CheckDiagnosticForeground(Form expected,string message){
  IntPtr current=GetForegroundWindow();bool matches=current==expected.Handle;
  if(!matches){uint process;GetWindowThreadProcessId(current,out process);report["diagnostic_foreground_failure"]=new{expected_handle=expected.Handle.ToInt64(),actual_handle=current.ToInt64(),actual_process=process,fixture_process=Process.GetCurrentProcess().Id,modal=Field<bool>("modal"),busy=Field<bool>("busy"),windows=Application.OpenForms.Cast<Form>().Select(f=>new{title=f.Text,type=f.GetType().FullName,handle=f.Handle.ToInt64(),foreground=f.Handle==current,active=Form.ActiveForm==f,visible=f.Visible,enabled=f.Enabled}).ToArray()};}
  Check(matches,message);
 }
 static bool handlingCompletion;
 static async Task Ready(){var start=DateTime.UtcNow;while(!shell.IsReady){if(uiFailure!=null)throw new InvalidOperationException("Native UI exception",uiFailure);if((DateTime.UtcNow-start).TotalSeconds>120)throw new TimeoutException(Field<Label>("feedback").Text);await Task.Delay(40);}await Task.Delay(70);if(uiFailure!=null)throw new InvalidOperationException("Native UI exception",uiFailure);
  string focus=Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS");
  if(!handlingCompletion&&focus!="session"&&focus!="continuous"&&focus!="compatibility"&&focus!="latency"&&focus!="display"&&(Field<Dictionary<string,object>>("pendingSessionCompletion")!=null||Field<bool>("sessionCompletionOpening"))){handlingCompletion=true;try{await SessionNativeChecks.CloseCompletion(shell,Ready,Check,null,null);}finally{handlingCompletion=false;}}
 }
 static async Task Command(string action,params object[] args){
  var data=LocalApi.D("action",action);for(int i=0;i<args.Length;i+=2)data[(string)args[i]]=args[i+1];
  var elapsed=Stopwatch.StartNew();bool accepted=false,ready=false;double sendMilliseconds=0;
  try{accepted=await shell.Send(data);sendMilliseconds=elapsed.Elapsed.TotalMilliseconds;Check(accepted,"Accepted "+action+(accepted?"":" · "+Field<Label>("feedback").Text));await Ready();ready=true;}
  finally{elapsed.Stop();commandTimings.Add(new{action=action,accepted=accepted,ready=ready,sendMilliseconds=sendMilliseconds,readyMilliseconds=elapsed.Elapsed.TotalMilliseconds});}
 }
 static void Invoke(string id){typeof(ExperimentShell).GetMethod("RunCommand",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,new object[]{id});}
 static Form Tool(string id){return Field<Dictionary<string,Form>>("workWindows")[id=="macro"||id=="operation"?"solve":id];}
 static string Hash(){return Convert.ToString(shell.Work["hash"]);}
 static Dictionary<string,object> Work(){return Map(shell.Work["workspace"]);}
 [System.Runtime.InteropServices.DllImport("user32.dll")]static extern IntPtr GetForegroundWindow();
 static void CaptureDesktop(string file,Form window){
  Check(GetForegroundWindow()==window.Handle,"Actual "+file+" capture owns Windows foreground input");string capture=Environment.GetEnvironmentVariable("MAGIC600_CAPTURE_FFMPEG");if(String.IsNullOrEmpty(capture)){Console.WriteLine("SKIP Actual "+file+" desktop frame: no recorder configured");Console.Out.Flush();return;}if(!File.Exists(capture))throw new FileNotFoundException("Configured desktop capture tool is missing");
  string target=Path.Combine(output,file);var start=new ProcessStartInfo(capture,"-hide_banner -loglevel error -f gdigrab -framerate 1 -i desktop -frames:v 1 -y "+(char)34+target+(char)34){UseShellExecute=false,CreateNoWindow=true,RedirectStandardError=true};using(var process=Process.Start(start)){if(!process.WaitForExit(15000)){process.Kill();throw new TimeoutException("Desktop capture did not finish");}Check(process.ExitCode==0&&File.Exists(target),"Actual Windows desktop "+file+" frame captured with FFmpeg");Check(GetForegroundWindow()==window.Handle,"Actual "+file+" capture retained Windows foreground ownership");}
 }
 static async Task CaptureDesktopAsync(string name,Form window){
  string file="desktop-"+name+".png";window.Activate();window.Refresh();await Task.Delay(250);
  Check(GetForegroundWindow()==window.Handle,"Actual "+file+" capture owns Windows foreground input");
  string capture=Environment.GetEnvironmentVariable("MAGIC600_CAPTURE_FFMPEG");if(String.IsNullOrEmpty(capture)){Console.WriteLine("SKIP Actual "+file+" desktop frame: no recorder configured");Console.Out.Flush();return;}if(!File.Exists(capture))throw new FileNotFoundException("Configured desktop capture tool is missing");
  string state=Hash(),context=Convert.ToString(Map(shell.Work["review_context"])["id"]),target=Path.Combine(output,file);
  await Task.Factory.StartNew(delegate{
   var start=new ProcessStartInfo(capture,"-hide_banner -loglevel error -f gdigrab -framerate 1 -i desktop -frames:v 1 -y "+(char)34+target+(char)34){UseShellExecute=false,CreateNoWindow=true,RedirectStandardError=true};
   using(var process=Process.Start(start)){if(!process.WaitForExit(15000)){process.Kill();throw new TimeoutException("Desktop capture did not finish");}if(process.ExitCode!=0||!File.Exists(target))throw new InvalidOperationException("Desktop capture failed: "+process.StandardError.ReadToEnd());}
  });
  Check(GetForegroundWindow()==window.Handle,"Actual "+file+" capture retained Windows foreground ownership");
  Check(Hash()==state&&Convert.ToString(Map(shell.Work["review_context"])["id"])==context,"Actual "+file+" capture preserved Session and ReviewContext");
  report[name]=new{capture="actual Windows desktop; UI message loop active",window=BoundsRecord(window.Bounds),workingArea=BoundsRecord(Screen.FromControl(window).WorkingArea)};
 }
 static void Image(string name,Form window){
  using(var image=new Bitmap(window.Width,window.Height)){window.DrawToBitmap(image,new Rectangle(Point.Empty,image.Size));image.Save(Path.Combine(output,name+".png"),ImageFormat.Png);}report[name]=new{width=window.Width,height=window.Height,clientWidth=window.ClientSize.Width,clientHeight=window.ClientSize.Height};
  if(name=="session-report"||name=="session-completion"||name=="session-log-checked-pending"||name=="keymap-import-preview")CaptureDesktop("desktop-"+name+".png",window);
  if(name=="12-help-compact"||name=="keyboard-compact-collapsed"||name=="solve-macro-use"||name=="cycles-complete"||name=="solve-work-intents"){
   CaptureDesktop(name=="12-help-compact"?"desktop-help.png":name=="solve-macro-use"?"desktop-solve.png":name=="cycles-complete"?"desktop-cycles.png":name=="solve-work-intents"?"desktop-solve-intents.png":"desktop-keyboard.png",window);
  }
 }
 static async Task Key(string code){var input=Field<ExperimentInput>("input");input.HandleKeyDown(code,false,false,false,false,false,false);input.HandleKeyUp(code);await Ready();}
 static IEnumerable<Control> Descendants(Control parent){foreach(Control child in parent.Controls){yield return child;foreach(var nested in Descendants(child))yield return nested;}}
 static object BoundsRecord(Rectangle bounds){return new{x=bounds.X,y=bounds.Y,width=bounds.Width,height=bounds.Height};}
 static void WindowWithinScreen(string key,Form window){var area=Screen.FromControl(window).WorkingArea;report[key]=new{bounds=BoundsRecord(window.Bounds),workingArea=BoundsRecord(area),client=BoundsRecord(window.ClientRectangle)};Check(area.Contains(window.Bounds),key+" outer bounds fit the screen working area");}
 static Rectangle ExposedBounds(Control control){
  if(control.IsDisposed||!control.Visible||!control.IsHandleCreated)return Rectangle.Empty;
  var visible=control.RectangleToScreen(control.ClientRectangle);
  for(Control parent=control.Parent;parent!=null;parent=parent.Parent){if(!parent.Visible)return Rectangle.Empty;visible=Rectangle.Intersect(visible,parent.RectangleToScreen(parent.ClientRectangle));}
  return Rectangle.Intersect(visible,Screen.FromControl(control).WorkingArea);
 }
 static bool FullyExposed(Control control){return !control.IsDisposed&&control.Visible&&control.IsHandleCreated&&control.ClientSize.Width>0&&control.ClientSize.Height>0&&ExposedBounds(control)==control.RectangleToScreen(control.ClientRectangle);}
 static void ExposedControl(Control control,string description){
  Check(FullyExposed(control),description+" is fully inside its window and ancestor viewports");
  var label=control as Label;if(label!=null){var preferred=label.GetPreferredSize(new Size(label.ClientSize.Width,0));Check(preferred.Height<=label.ClientSize.Height+2,description+" text fits its allocated height (needs "+preferred.Height+", has "+label.ClientSize.Height+")");}
 }
 static async Task LayoutSaved(){
  var deadline=DateTime.UtcNow.AddSeconds(20);while(shell.WindowLayoutPending){if(DateTime.UtcNow>deadline)throw new TimeoutException("Window layout remains pending: "+shell.WindowLayoutFailure);await Task.Delay(30);}await Ready();
 }
 static async Task WindowLayoutTraffic(){
  await LayoutSaved();var api=Field<LocalApi>("api");var window=Tool("local");var original=window.Bounds;
  byte[] labels=await Task.Factory.StartNew(()=>api.Bytes("labels"));
  window.Left=original.Left+1;
  var timer=Field<Timer>("windowSaveTimer");typeof(Timer).GetMethod("OnTick",BindingFlags.NonPublic|BindingFlags.Instance).Invoke(timer,new object[]{EventArgs.Empty});
  Check(Field<bool>("windowSaveInFlight")&&shell.IsReady,"Layout timer starts a background write without reserving the input boundary");
  window.Left=original.Left+2;var latest=window.Bounds;
  await Command("operation-new");await LayoutSaved();
  var status=await Task.Factory.StartNew(()=>api.Get("status"));
  var saved=Map(Map(Map(Map(Map(status["prefs"])["layout"])["magic600_experiment"])["view"])["windows"]);
  var local=Map(saved["local"]);
  Check(Convert.ToInt32(local["x"])==latest.X&&Convert.ToInt32(local["y"])==latest.Y&&Convert.ToInt32(local["width"])==latest.Width&&Convert.ToInt32(local["height"])==latest.Height,"Latest bounds changed during the first request are durably saved");
  var after=await Task.Factory.StartNew(()=>api.Bytes("labels"));Check(labels.SequenceEqual(after),"Background layout writes preserve all 259800 labels");
  Check(shell.WindowLayoutFailure==null,"No layout persistence error remains");
  window.Bounds=original;await LayoutSaved();
 }
 static bool KeyboardExtras(){return (bool)typeof(ExperimentShell).GetProperty("KeyboardExtrasVisible",BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell,null);}
 static async Task KeyboardAccess(string key){
  var panel=Field<FlowLayoutPanel>("keyboard");var window=Tool("keyboard");var input=Field<ExperimentInput>("input");
  window.Activate();panel.Focus();await Task.Delay(40);
  var serializer=new JavaScriptSerializer();string hash=Hash(),draft=serializer.Serialize(Work()["draft"]),next=serializer.Serialize(Work()["next"]),bank=Convert.ToString(Work()["bank"]);var grip=input.ActiveGripCell;
  Check(!KeyboardExtras(),"Supplementary chords start collapsed");
  var keys=Field<Dictionary<Button,string>>("physicalKeyboardKeys");
  Check(keys.Where(p=>!p.Value.Contains("+")).All(p=>p.Key.Visible),"All unmodified assigned and main keys remain discoverable without Extra keys");
  var standard=Field<HashSet<string>>("standardKeyboardCodes");Check(keys.Where(p=>standard.Contains(p.Value)).All(p=>FullyExposed(p.Key)),"Five compact main rows are fully visible without scrolling");
  ExposedControl(Field<Label>("keyboardContext"),"Compact key set, Grip and input context");
  foreach(var button in keys.Keys.Where(b=>b.Visible)){Check(button.Width>=44&&button.Height>=44,"Compact key keeps a usable target: "+keys[button]);}
  foreach(var label in Descendants(window).OfType<Label>().Where(c=>c.Parent.GetType().Name=="WorkReadout"))ExposedControl(label,"Compact keyboard identity/protection");
  await KeyboardAccessVisible(key+"-collapsed");Image("keyboard-compact-collapsed",window);
  await Key("Backquote");Check(KeyboardExtras(),"Visible Backquote route expands supplementary keys");
  await KeyboardAccessVisible(key+"-expanded");Image("keyboard-compact-expanded",window);
  await Key("Backquote");Check(!KeyboardExtras(),"The same explicit key collapses supplementary keys");
  bool handled=input.HandleKeyDown("Backquote",false,false,false,false,true,false);input.HandleKeyUp("Backquote",true);
  Check(!handled&&!KeyboardExtras(),"Typing Backquote does not open keyboard tools");
  Check(Hash()==hash&&serializer.Serialize(Work()["draft"])==draft&&serializer.Serialize(Work()["next"])==next&&Convert.ToString(Work()["bank"])==bank&&input.ActiveGripCell==grip,"Extra-key disclosure preserves model, draft, locked Next, bank and Grip");
 }
 static async Task KeyboardAccessVisible(string key){
  var panel=Field<FlowLayoutPanel>("keyboard");var buttons=Field<Dictionary<Button,string>>("physicalKeyboardKeys");var origin=panel.AutoScrollPosition;
  report[key+"-extent"]=new{minimum=panel.AutoScrollMinSize,desired=panel.Padding.Vertical+Field<List<Control>>("keyboardRows").Where(c=>c.Visible).Sum(c=>c.Height+c.Margin.Vertical),rows=Field<List<Control>>("keyboardRows").Select(c=>new{height=c.Height,margin=c.Margin.Vertical,visible=c.Visible}).ToArray()};
  var initial=buttons.Select(pair=>new{code=pair.Value,visible=FullyExposed(pair.Key),bounds=BoundsRecord(pair.Key.RectangleToScreen(pair.Key.ClientRectangle))}).ToArray();
  var rows=buttons.Keys.GroupBy(button=>button.Parent).Select((group,index)=>new{row=index+1,keys=group.Select(b=>buttons[b]).ToArray(),fullyVisible=group.All(FullyExposed),partlyVisible=group.Any(b=>!ExposedBounds(b).IsEmpty)}).ToArray();
  var scrolled=new List<string>();report[key]=new{viewport=BoundsRecord(panel.RectangleToScreen(panel.ClientRectangle)),keys=initial,rows=rows,scrollAccess=scrolled};Check(buttons.Count>0,"Keyboard has real key controls");
  try{foreach(var pair in buttons.Where(pair=>pair.Key.Visible)){if(FullyExposed(pair.Key))continue;Check(panel.AutoScroll&&(panel.HorizontalScroll.Visible||panel.VerticalScroll.Visible),"Clipped key "+pair.Value+" has an explicit scrollbar");panel.ScrollControlIntoView(pair.Key);await Task.Delay(15);if(!FullyExposed(pair.Key)){Image("keyboard-scroll-failure",Tool("keyboard"));var lineage=new List<object>();for(Control c=pair.Key;c!=null;c=c.Parent)lineage.Add(new{type=c.GetType().Name,text=c.Text,disposed=c.IsDisposed,visible=c.Visible,handle=c.IsHandleCreated,client=BoundsRecord(c.ClientRectangle),bounds=BoundsRecord(c.Bounds)});report["failed-key"]=new{code=pair.Value,lineage=lineage,scroll=panel.AutoScrollPosition,display=BoundsRecord(panel.DisplayRectangle),maximum=panel.VerticalScroll.Maximum,largeChange=panel.VerticalScroll.LargeChange,scrollValue=panel.VerticalScroll.Value,padding=panel.Padding.ToString()};}Check(FullyExposed(pair.Key),"Scrolling exposes the complete "+pair.Value+" key target");scrolled.Add(pair.Value);}}
  finally{panel.AutoScrollPosition=new Point(-origin.X,-origin.Y);}
  await Task.Delay(30);
  Console.WriteLine("Keyboard rows initially fully visible: "+rows.Count(r=>r.fullyVisible)+"/"+rows.Length+"; keys reached through scrolling: "+scrolled.Count);
 }
 static void WorksheetExposure(Form sheet,string stage){
  var labels=Descendants(sheet).OfType<Label>().ToArray();var required=new List<Control>();
  foreach(string field in new[]{"Current piece","Destination position"}){var row=labels.Where(l=>(l.AccessibleName??"").StartsWith(field+" · ",StringComparison.Ordinal)).ToArray();Check(row.Length==3,"Worksheet retains saved and current "+field+" comparison");required.AddRange(row);}
  foreach(string name in new[]{"Work sheet locked Next bookmark","Current protection for work sheet"})required.Add(labels.Single(l=>l.AccessibleName==name));
  var actions=Descendants(sheet).OfType<Button>().Where(b=>b.Text.StartsWith("Load fixed steps",StringComparison.Ordinal)||b.Text=="Refresh comparison"||b.Text=="Cancel").ToArray();Check(actions.Length==3,"Worksheet retains load, refresh and cancel controls");required.AddRange(actions);
  report[stage]=required.Select(c=>new{name=c.AccessibleName??c.Text,bounds=BoundsRecord(c.RectangleToScreen(c.ClientRectangle)),exposed=BoundsRecord(ExposedBounds(c))}).ToArray();
  foreach(var control in required)ExposedControl(control,stage+" · "+(control.AccessibleName??control.Text));
 }
 static async Task<Form> OpenToolDialog(string command,string title){form.BeginInvoke((Action)delegate{Invoke(command);});var start=DateTime.UtcNow;while(true){var window=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text.StartsWith(title,StringComparison.Ordinal));if(window!=null)return window;if((DateTime.UtcNow-start).TotalSeconds>30)throw new TimeoutException("Tool did not open: "+title);await Task.Delay(40);}}
 static async Task MacroLibraryWorkflow(){
  string labels=Hash(),draft=new JavaScriptSerializer().Serialize(Work()["draft"]),bookmark=new JavaScriptSerializer().Serialize(Work()["next"]);
  Invoke("bank-macro-1");await Ready();string selected=Field<string>("selectedMacro");Check(selected=="o33-n0-inverse","Explicit bank macro selects its canonical library entry");
  var details=await OpenToolDialog("macro-details","Macro details");details.Size=details.MinimumSize;await Task.Delay(100);Check(Descendants(details).OfType<TabControl>().Single().TabPages.Count==3,"Macro details retain readable effects, canonical record and explicit comparison");foreach(var readout in Descendants(details).OfType<Label>().Where(c=>c.Parent.GetType().Name=="WorkReadout"))ExposedControl(readout,"Minimum-width macro identity/protection");Image("13-macro-details",details);((IButtonControl)details.CancelButton).PerformClick();await Ready();
  var editor=await OpenToolDialog("macro-label","Edit macro · ");var fields=Descendants(editor).OfType<TextBox>().ToArray();var name=fields.Single(c=>c.AccessibleName=="Macro human name");string oldName=name.Text;name.Text="Uncommitted edit";
  Check(!Descendants(editor).OfType<Button>().Single(b=>b.Text=="Export…").Enabled&&!Descendants(editor).OfType<Button>().Single(b=>b.Text=="Import…").Enabled,"Unsaved metadata cannot be discarded by file navigation");Descendants(editor).OfType<Button>().Single(b=>b.Text=="Cancel").PerformClick();await Ready();
  Check(Convert.ToString(Items(shell.Work["library"]).Select(Map).Single(m=>Convert.ToString(m["id"])==selected)["name"])==oldName,"Cancelling metadata retains saved name");
  editor=await OpenToolDialog("macro-label","Edit macro · ");fields=Descendants(editor).OfType<TextBox>().ToArray();fields.Single(c=>c.AccessibleName=="Macro usage notes").Text="Explicit two-buffer insertion witness";fields.Single(c=>c.AccessibleName=="Macro tags separated by commas").Text="manual, Straße";Descendants(editor).OfType<Button>().Single(b=>b.Text=="Save labels").PerformClick();await Ready();Invoke("macro-pin");await Ready();
  var record=Items(shell.Work["library"]).Select(Map).Single(m=>Convert.ToString(m["id"])==selected);Check(Convert.ToBoolean(record["pinned"])&&Items(record["tags"]).Select(Convert.ToString).Contains("Straße"),"Native metadata and pin actions preserve original tags");
  var filter=await OpenToolDialog("macro-filter","Filter Macro Base");Descendants(filter).OfType<ComboBox>().Single(c=>c.AccessibleName=="Macro exact action type").SelectedIndex=1;Descendants(filter).OfType<TextBox>().Single(c=>c.AccessibleName=="All required macro tags").Text="STRASSE";Descendants(filter).OfType<ComboBox>().Single(c=>c.AccessibleName=="Likely solving use filter").SelectedIndex=0;Descendants(filter).OfType<Button>().Single(b=>b.Text=="Apply filter").PerformClick();await Ready();
  Check(Field<ListBox>("macros").Items.Count==1&&Field<string>("selectedMacro")==selected,"Native filter uses authoritative Unicode keys and verified star membership");Image("09-macro-filtered",Tool("macro"));
  filter=await OpenToolDialog("macro-filter","Filter Macro Base");Descendants(filter).OfType<ComboBox>().Single(c=>c.AccessibleName=="Macro exact action type").SelectedIndex=7;Descendants(filter).OfType<TextBox>().Single(c=>c.AccessibleName=="All required macro tags").Text="";Descendants(filter).OfType<Button>().Single(b=>b.Text=="Apply filter").PerformClick();await Ready();
  Check(Field<string>("selectedMacro")==selected&&Field<ListBox>("macros").SelectedIndex==-1,"Hiding the inspected macro preserves its canonical selection");
  filter=await OpenToolDialog("macro-filter","Filter Macro Base");Descendants(filter).OfType<Button>().Single(b=>b.Text=="Clear facets").PerformClick();Descendants(filter).OfType<Button>().Single(b=>b.Text=="Apply filter").PerformClick();await Ready();
  Check(Hash()==labels&&new JavaScriptSerializer().Serialize(Work()["draft"])==draft&&new JavaScriptSerializer().Serialize(Work()["next"])==bookmark,"Macro browsing and labels retain all puzzle labels, draft and locked Next");
 }
 static async Task<Form> Sheet(string name){typeof(ExperimentShell).GetMethod("ShowWorkSheet",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,new object[]{name});var start=DateTime.UtcNow;while(true){var window=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Text=="Reuse work sheet · "+name);if(window!=null&&window.Visible)return window;if((DateTime.UtcNow-start).TotalSeconds>30)throw new TimeoutException("Worksheet comparison did not open: "+Field<Label>("feedback").Text);await Task.Delay(40);}}

 static string FileSha(string path){using(var sha=SHA256.Create())using(var stream=File.OpenRead(path))return BitConverter.ToString(sha.ComputeHash(stream)).Replace("-","").ToLowerInvariant();}
 static async Task<byte[]> FullLabels(){var api=Field<LocalApi>("api");return await Task.Factory.StartNew(()=>api.Bytes("labels"));}
 static object CanonicalTestValue(object value){var map=value as IDictionary<string,object>;if(map!=null)return new SortedDictionary<string,object>(map.ToDictionary(p=>p.Key,p=>CanonicalTestValue(p.Value)),StringComparer.Ordinal);var array=value as object[];return array==null?value:array.Select(CanonicalTestValue).ToArray();}
 static string WorkInvariant(){var w=new Dictionary<string,object>(Work());w.Remove("bank");w.Remove("previous_bank");var view=new Dictionary<string,object>(Map(w["view"]));view.Remove("local_center");view.Remove("windows");w["view"]=view;return Field<LocalApi>("api").Json(CanonicalTestValue(new object[]{Hash(),w,shell.Work["pending"],shell.Work["protected"],shell.Work["position_locks"]}));}
 static void ExportNativeWitness(){
  var bridge=Field<ExperimentBridge>("bridge");var api=Field<LocalApi>("api");var geometry=bridge.Geometry();var profile=Map(api.Parse(File.ReadAllText(Path.Combine(output,"session","native_profile.json"))));
  Check(Convert.ToString(profile["profile_sha256"])==Convert.ToString(bridge.Profile["profile_sha256"]),"Full persisted handshake profile matches the native wire profile");
  Check(Convert.ToInt32(profile["matched_stickers"])==259800&&Convert.ToInt32(profile["matched_generators"])==1200,"Export uses the actual complete native handshake");
  Check(Convert.ToString(geometry["executable_sha256"])==Convert.ToString(profile["native_executable_sha256"])&&Convert.ToString(profile["model_id"])==Convert.ToString(shell.Work["model"]),"Native geometry witness matches this actual runtime and model");
  string geometryPath=Path.Combine(output,"native-geometry.json"),profilePath=Path.Combine(output,"native-profile.json");File.WriteAllText(geometryPath,api.Json(geometry));File.WriteAllText(profilePath,api.Json(profile));
  var witness=LocalApi.D("status","exported","scope","Actual bridge.Geometry export and already-verified native handshake only; not human solving, visual acceptance or latency evidence","model",shell.Work["model"],"profile_sha256",profile["profile_sha256"],"matched_stickers",259800,"matched_generators",1200,"native_executable_sha256",geometry["executable_sha256"],"geometry_file","native-geometry.json","geometry_file_sha256",FileSha(geometryPath),"profile_file","native-profile.json","profile_file_sha256",FileSha(profilePath));
  File.WriteAllText(Path.Combine(output,"native-handshake-witness.json"),api.Json(witness));report["native_handshake_witness"]=witness;
 }
 static async Task DisplayFocused(){
  Check(Object.Equals(Map(shell.Work["session"])["raw_full_home"],true)&&shell.Work["pending"]==null,"Display fixture starts in a fresh disposable Home session");byte[] original=await FullLabels();
  await Command("filter","expression","active");await Command("operation-new");await Command("goal","goal","prepare");await Command("draft","phase","prepare","recipe",new object[]{LocalApi.D("kind","word","moves",new object[]{1})});await Command("review");await Command("preview","review_id",Map(shell.Work["review"])["id"]);
  Check(original.Length==259800*4&&(await FullLabels()).SequenceEqual(original),"Explicit legal [1] Prepare preview has not executed any label change");ExportNativeWitness();
  await DisplayNativeChecks.Run(shell,form,Ready,Check,Image);Check(shell.Work["pending"]!=null&&(await FullLabels()).SequenceEqual(original),"Focused display validation leaves the actual preview staged and every Home label intact");
 }
 static async Task RecordedSetup(List<object> transcript,string action,params object[] args){string before=Hash();object head=shell.Work["head"];var request=LocalApi.D("action",action);for(int i=0;i<args.Length;i+=2)request[(string)args[i]]=args[i+1];await Command(action,args);transcript.Add(LocalApi.D("request",request,"accepted",true,"before_hash",before,"after_hash",Hash(),"before_head",head,"after_head",shell.Work["head"]));}
 static async Task LatencyDiagnostic(Dictionary<string,object> environment){
  var api=Field<LocalApi>("api");byte[] baseline=await FullLabels();string stable=WorkInvariant(),oldBank=Convert.ToString(Work()["bank"]);object oldCenter=Map(Work()["view"])["local_center"];var setup=new List<object>();var measured=new List<object>();var surfaces=new Dictionary<string,object>();Exception failure=null;
  try{
   Invoke("keyboard");await Ready();var keyboard=Field<Form>("keyboardWindow");keyboard.Activate();await Task.Delay(150);surfaces["bank_window"]=BoundsRecord(keyboard.Bounds);using(var graphics=keyboard.CreateGraphics())surfaces["bank_dpi"]=new{x=graphics.DpiX,y=graphics.DpiY};await RecordedSetup(setup,"bank","id","Macro");await RecordedSetup(setup,"bank","id","Keyboard");
   for(int i=0;i<5;i++){string target=i%2==0?"Macro":"Keyboard";CheckDiagnosticForeground(keyboard,"Diagnostic bank window retains foreground");int timing=commandTimings.Count;await Command("bank","id",target);Check(Convert.ToString(Work()["bank"])==target&&Field<Label>("keyboardContext").Text.StartsWith("SET "+target+" ·",StringComparison.Ordinal),"Diagnostic bank response names the requested set");measured.Add(LocalApi.D("kind","bank","index",i,"request",LocalApi.D("action","bank","id",target),"timing",commandTimings[timing],"state_hash",Hash()));}
   Invoke("local");await Ready();var local=Field<NativeCellView>("local");local.FindForm().Activate();await Task.Delay(150);surfaces["local_window"]=BoundsRecord(local.FindForm().Bounds);using(var graphics=local.CreateGraphics())surfaces["local_dpi"]=new{x=graphics.DpiX,y=graphics.DpiY};await RecordedSetup(setup,"settings","view",LocalApi.D("local_center",1));await RecordedSetup(setup,"settings","view",LocalApi.D("local_center",7));
   for(int i=0;i<5;i++){int target=i%2==0?1:7;CheckDiagnosticForeground(local.FindForm(),"Diagnostic Local window retains foreground");int timing=commandTimings.Count;await Command("settings","view",LocalApi.D("local_center",target));int[] labels=(int[])typeof(NativeCellView).GetField("cutLabels",BindingFlags.Instance|BindingFlags.NonPublic).GetValue(local);Check(local.CenterCell==target&&local.LocalStateHash==Hash()&&labels.Length==433&&labels.Select((label,index)=>(uint)label==BitConverter.ToUInt32(baseline,((target-1)*433+index)*4)).All(v=>v),"Diagnostic Local response contains the requested center and its 433 exact labels");measured.Add(LocalApi.D("kind","local-center","index",i,"request",LocalApi.D("action","settings","view",LocalApi.D("local_center",target)),"timing",commandTimings[timing],"state_hash",Hash()));}
   Check(WorkInvariant()==stable&&(await FullLabels()).SequenceEqual(baseline),"Diagnostic navigation preserves all 259800 labels, working identities, drafts, protection and pending state");
  }catch(Exception error){failure=error;}
  try{await Ready();await Command("bank","id",oldBank);await Command("settings","view",LocalApi.D("local_center",oldCenter));Check(WorkInvariant()==stable&&(await FullLabels()).SequenceEqual(baseline),"Diagnostic restores starting bank and Local center without a mechanical change");}catch(Exception cleanup){failure=failure==null?cleanup:new AggregateException(failure,cleanup);}
  var result=LocalApi.D("status",failure==null?"diagnostic-only":"failed","scope","Five bank and five Local Send/adoption diagnostics; no matching Present/WM_PAINT receipt, no p95 or S14 acceptance","environment",environment,"surfaces",surfaces,"warmup_and_surface_setup",setup,"samples",measured,"error",failure==null?null:failure.ToString());File.WriteAllText(Path.Combine(output,"native-latency-diagnostic.json"),api.Json(result));if(failure!=null)throw failure;
 }
 static async Task LatencyFocused(){
  string series=Environment.GetEnvironmentVariable("MAGIC600_LATENCY_SERIES"),path=Environment.GetEnvironmentVariable("MAGIC600_LATENCY_ENVIRONMENT");Check((series=="diagnostic"||series=="full")&&!String.IsNullOrEmpty(path)&&File.Exists(path),"Latency series and captured environment are explicitly supplied");
  var environment=Map(Field<LocalApi>("api").Parse(File.ReadAllText(path)));Check(Convert.ToString(environment["series"])==series,"Latency environment agrees with the explicit series");
  Check(Object.Equals(Map(shell.Work["session"])["raw_full_home"],true)&&shell.Work["pending"]==null,"Latency fixture starts from fresh disposable Home");var setup=new List<object>();byte[] home=await FullLabels();
  await RecordedSetup(setup,"native-word","moves",new object[]{1,3},"destination","live","state_hash",Hash());await RecordedSetup(setup,"protect","orbits",new object[0]);await RecordedSetup(setup,"filter","expression","active");
  Check(!Object.Equals(Map(shell.Work["session"])["raw_full_home"],true)&&!(await FullLabels()).SequenceEqual(home),"Recorded legal [1,3] setup actually establishes a non-Home baseline");
  Check(shell.Work["pending"]==null&&Items(shell.Work["protected"]).Length==0&&Items(shell.Work["position_locks"]).Length==0,"Measured baseline has no preview or mechanical protection");
  environment["recorded_setup"]=setup;environment["setup_scope"]="Real explicit chronological [1,3] outside measurement; no target search, scramble or completion suppression";environment["baseline_hash"]=Hash();ExportNativeWitness();environment["handshake_witness"]=report["native_handshake_witness"];
  if(series=="full")await NativeLatencyChecks.Run(shell,form,Ready,Check,output,environment);else await LatencyDiagnostic(environment);
 }

 static async Task Run(){try{
  await Ready();form.WindowState=FormWindowState.Normal;form.Size=new Size(1280,720);await Task.Delay(200);
  var registry=Field<Dictionary<string,Action>>("commands");var routes=new HashSet<string>();var single=new HashSet<string>();var utilities=new HashSet<string>();foreach(var bank in Items(shell.Work["banks"]).Select(Map)){if(bank.ContainsKey("legacy")&&Object.Equals(bank["legacy"],true))continue;if(bank.ContainsKey("functions_only"))foreach(var command in Items(bank["functions_only"]))utilities.Add(Convert.ToString(command));foreach(var pair in Map(bank["commands"])){routes.Add(Convert.ToString(pair.Value));if(!pair.Key.Contains("+"))single.Add(Convert.ToString(pair.Value));}}var missing=registry.Keys.Where(k=>!routes.Contains(k)).OrderBy(k=>k).ToArray();var missingCore=registry.Keys.Where(k=>!utilities.Contains(k)&&!single.Contains(k)).ToArray();report["unmapped_commands"]=missing;report["unknown_mapped_commands"]=routes.Where(k=>!registry.ContainsKey(k)).ToArray();
  Check(missing.Length==0,"Every registry command has an explicit editable set route: "+String.Join(",",missing));Check(missingCore.Length==0,"Every direct solving command retains a single-key route: "+String.Join(",",missingCore));Check(routes.All(registry.ContainsKey),"Every default route resolves an existing command");
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="display"){await DisplayFocused();exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="latency"){await LatencyFocused();exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="session"){await SessionNativeChecks.Run(shell,form,Ready,Check,Image);await SessionNativeChecks.RunFresh(shell,form,Ready,Check,Image);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="compatibility"){await SessionLogNativeChecks.Run(shell,form,Ready,Check,Image,output);await KeymapFileNativeChecks.Run(shell,form,Ready,Check,Image,output);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="continuous"){await ContinuousSolverNativeChecks.Run(shell,form,Ready,Check,Image,CaptureDesktopAsync,Environment.GetEnvironmentVariable("MAGIC600_CONTINUOUS_CASES"));exitCode=0;return;}
  await Command("fixture","name","e1");string startHash=Hash();
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="protection"){await ProtectionPickerNativeChecks.Run(shell,form,Ready,Check,Image);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="keymap-files"){await KeymapFileNativeChecks.Run(shell,form,Ready,Check,Image,output);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="cycles"){await CycleNativeChecks.Run(shell,form,Ready,Check,Image);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="intents"){await WorkIntentNativeChecks.Run(shell,form,Ready,Check,Image);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="recommendation"){await CurrentScoreNativeChecks.Run(shell,form,Ready,Check,Image,CaptureDesktopAsync);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="endgame"){await EndgameNativeChecks.Run(shell,form,Ready,Check,Image,CaptureDesktopAsync);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="names"){await MathematicalNameNativeChecks.Run(shell,form,Ready,Check);exitCode=0;return;}
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="macro-use"){Invoke("keyboard");await Ready();await MacroUseNativeChecks.Run(shell,form,Ready,Check,Image);exitCode=0;return;}
  await StopAcknowledgementNativeChecks.Run(shell,Ready,Check);
  await KeyboardBatchNativeChecks.Run(shell,form,Ready,Check);
  if(mode=="g2"&&Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="keyboard"){
   Invoke("keyboard");await Ready();
   await Command("bank","id","33-A");await Command("capture","cells",new[]{55});
   await KeyboardCompatibilityNativeChecks.Run(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
   await KeyboardFeedbackNativeChecks.Run(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
   Image("keyboard-batch-final",Tool("keyboard"));exitCode=0;return;
  }
  if(mode=="g1"){
   Invoke("keyboard");await Ready();Check(Tool("keyboard").Visible,"G1 independent keyboard opens");WindowWithinScreen("g1-first-keyboard-bounds",Tool("keyboard"));await KeyboardAccess("g1-first-keyboard-access");
   await Command("bank","id","33-A");await Command("capture","cells",new[]{55});
   await KeyboardCompatibilityNativeChecks.Run(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
   await KeyboardFeedbackNativeChecks.Run(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
   Tool("keyboard").Activate();Field<FlowLayoutPanel>("keyboard").Focus();var g1Input=Field<ExperimentInput>("input");
   Check(g1Input.SetPointerGrip(55),"G1 explicit cap selected");Check(g1Input.Twist("T1",false),"G1 forward entered");await Ready();
   Check(g1Input.SetPointerGrip(55),"G1 cap selected again");
   var twistKey=Field<Dictionary<Button,string>>("physicalKeyboardKeys").Single(pair=>pair.Value=="KeyF").Key;var axis=Items(shell.Work["frame_notation"]).Select(Map).Single(row=>Convert.ToString(row["axis"])=="T1");
   g1Input.HandleKeyDown("ShiftLeft",true,false,false,false,false);Check(g1Input.InverseShiftHeld&&twistKey.AccessibleName.Contains("Twist "+Convert.ToString(axis["inverse"])),"Actual Keyboard immediately displays the exact Shift inverse cycle");Image("keyboard-shift-inverse",Tool("keyboard"));
   Check(g1Input.HandleKeyDown("KeyF",true,false,false,false,false),"G1 inverse entered through the actual Shift Twist route");g1Input.HandleKeyUp("KeyF");g1Input.HandleKeyUp("ShiftLeft");await Ready();
   Check(!g1Input.InverseShiftHeld&&twistKey.AccessibleName.Contains("Twist "+Convert.ToString(axis["forward"])),"Shift release restores the ordinary cycle in the actual Keyboard");
   await Command("review");Check(Convert.ToInt32(Map(Map(shell.Work["review"])["effect"])["slots"])==0,"G1 forward and inverse full net identity");Check(Hash()==startHash,"G1 draft does not execute");
   Image("01-keyboard",Tool("keyboard"));Tool("keyboard").Close();Check(!Tool("keyboard").IsDisposed&&!Tool("keyboard").Visible,"G1 keyboard user close preserves reusable window");Invoke("keyboard");await Ready();Check(Tool("keyboard").Visible,"G1 keyboard reopens");exitCode=0;return;
  }
  foreach(string id in new[]{"local","global","keyboard","operation-focus","macro-search"}){Invoke(id);await Ready();}
  foreach(string id in new[]{"local","global","keyboard","solve"}){Check(Tool(id).Visible,"Independent "+id+" window visible");WindowWithinScreen("first-"+id+"-bounds",Tool(id));}
  await WindowLayoutTraffic();
  await SolveWindowNativeChecks.Run(shell,form,Ready,Check,Image);
  await FunctionsNativeChecks.Run(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
  Exception keyboardAccessFailure=null;try{await KeyboardAccess("g2-first-keyboard-access");}catch(Exception error){keyboardAccessFailure=error;report["keyboard-access-failure"]=error.ToString();}

  await WorkspaceFeedbackNativeChecks.Grip(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
  await WorkspaceFeedbackNativeChecks.Help(shell,form,Ready,Check,Image);
  if(keyboardAccessFailure!=null)throw keyboardAccessFailure;
  Check(Hash()==startHash,"Opening tools did not mutate complete model");
  await Command("settings","view",LocalApi.D("local_center",55));
  await Command("bank","id","33-A");await Command("capture","cells",new[]{55});
  Tool("keyboard").Activate();Field<FlowLayoutPanel>("keyboard").Focus();
  var input=Field<ExperimentInput>("input");Check(input.SetPointerGrip(55),"Explicit onscreen cap selected");Check(input.Twist("T1",false),"Explicit forward corner turn entered");await Ready();
  Check(input.SetPointerGrip(55),"Explicit cap retained after update");Check(input.Twist("T1",true),"Exact inverse corner turn entered");await Ready();
  await Command("review");Check(Convert.ToInt32(Map(Map(shell.Work["review"])["effect"])["slots"])==0,"Forward plus inverse has zero full-label net effect");
  Check(Hash()==startHash,"Draft input did not execute");
  Image("01-keyboard",Tool("keyboard"));Image("02-local",Tool("local"));Image("03-hub",form);
  await Command("bank","id","Views");string stableBank=Convert.ToString(Work()["bank"]);
  await Command("focus","identity",17810);Check(Convert.ToString(Work()["bank"])==stableBank,"Changing Current preserves explicit key set");
  Check(Field<NativeCellView>("local").CenterCell==55,"Changing Current preserves Local center");
  await Command("filter","expression","cell(C55)&piece(17810)");Image("04-filtered-local",Tool("local"));
  Check(Hash()==startHash,"View and filter changes preserve complete labels");
  await Command("focus","identity",35778);await Command("bank","id","33-I");await Command("operation-new");
  await MacroLibraryWorkflow();
  await MacroUseNativeChecks.Run(shell,form,Ready,Check,Image);
  await MathematicalNameNativeChecks.Run(shell,form,Ready,Check);
  await CycleNativeChecks.Run(shell,form,Ready,Check,Image);
  string chosenVariant=await MacroVariantsNativeChecks.Run(shell,form,Ready,payload=>shell.Send(payload),Check,Image);
  await Command("settings","phase","macro");Invoke("macro-insert");await Ready();Check(Field<string>("selectedMacro")==chosenVariant&&new JavaScriptSerializer().Serialize(Work()["draft_sources"]).Contains(chosenVariant),"Explicit Add inserts the saved inverse with fixed source identity");
  await Command("review");Check(Map(shell.Work["review"])["status"].ToString()=="Ready","First explicit insertion checked");
  Check(!Field<Label>("effectText").Text.Contains("Applicability"),"Complete-operation readout does not present macro-body applicability");form.BeginInvoke((Action)delegate{typeof(Control).GetMethod("OnClick",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(Field<Label>("effectText"),new object[]{EventArgs.Empty});});var effectDeadline=DateTime.UtcNow.AddSeconds(20);Form operationDetails=null;while(operationDetails==null){operationDetails=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text=="Operation check");if(DateTime.UtcNow>effectDeadline)throw new TimeoutException("Complete effect click did not open operation check");await Task.Delay(30);}Check(true,"Complete-operation effect opens complete-operation findings");((IButtonControl)operationDetails.CancelButton).PerformClick();await Ready();
  await Command("template-save","name","First insertion method");string sheetDraft=new JavaScriptSerializer().Serialize(Work()["draft"]);string sheetNext=new JavaScriptSerializer().Serialize(Work()["next"]);
  var sheet=await Sheet("First insertion method");await Task.Delay(100);WindowWithinScreen("worksheet-default-bounds",sheet);WorksheetExposure(sheet,"worksheet-default-exposure");Image("07-work-sheet",sheet);
  Check(Descendants(sheet).OfType<Label>().Any(l=>l.Text.Contains("Current piece")),"Work sheet exposes Current binding");
  Descendants(sheet).OfType<Button>().Single(b=>b.Text=="Cancel").PerformClick();await Ready();
  Check(sheetDraft==new JavaScriptSerializer().Serialize(Work()["draft"])&&sheetNext==new JavaScriptSerializer().Serialize(Work()["next"])&&Hash()==startHash,"Cancel worksheet preserves draft, Next and all labels");
  await Command("preview","review_id",Map(shell.Work["review"])["id"]);
  string pendingToken=TextPending();await MacroVariantsNativeChecks.PendingClose(shell,form,Ready,Check);sheet=await Sheet("First insertion method");sheet.Size=new Size(690,520);await Task.Delay(100);WindowWithinScreen("worksheet-compact-bounds",sheet);
  var detailTabs=Descendants(sheet).OfType<TabControl>().Single(t=>t.AccessibleName=="Fixed work-sheet method details");Check(detailTabs.SelectedTab.Text=="Macro","Worksheet initially selects its first nonempty Macro phase");
  Descendants(sheet).OfType<CheckBox>().Single(c=>c.Text.StartsWith("Show fixed steps")).Checked=true;await Task.Delay(100);WindowWithinScreen("worksheet-expanded-bounds",sheet);WorksheetExposure(sheet,"worksheet-expanded-exposure");Image("08-work-sheet-details",sheet);
  detailTabs.SelectedTab=detailTabs.TabPages.Cast<TabPage>().Single(t=>t.Text=="Macro revisions");string oldConfirmation=Convert.ToString(Field<Dictionary<string,object>>("lastCommandResult")["confirmation"]);
  Descendants(sheet).OfType<Button>().Single(b=>b.Text=="Refresh comparison").PerformClick();await Ready();Check(Convert.ToString(Field<Dictionary<string,object>>("lastCommandResult")["confirmation"])!=oldConfirmation,"Worksheet refresh obtains a new explicit confirmation");Check(detailTabs.SelectedTab.Text=="Macro revisions","Worksheet preserves the user's chosen detail tab after refresh");WorksheetExposure(sheet,"worksheet-refreshed-exposure");
  Descendants(sheet).OfType<Button>().Single(b=>b.Text.StartsWith("Load fixed steps")).PerformClick();await Ready();
  Check(shell.Work["review"]==null&&!Convert.ToBoolean(shell.Work["executable"]),"Loading worksheet removes old execution authority");Check(TextPending()==pendingToken,"Loading worksheet preserves unrelated authoritative preview");
  Check(sheetNext==new JavaScriptSerializer().Serialize(Work()["next"])&&Convert.ToString(Work()["bank"])=="33-I","Worksheet load preserves Next and selected key set");
  await Command("cancel-preview");await Command("review");await Command("preview","review_id",Map(shell.Work["review"])["id"]);
  Tool("operation").Activate();Invoke("operation-focus");await Key("KeyM");
  Check(Convert.ToString(shell.Work["operation_state"])=="executed","Single visible insertion-set key executes staged operation in Operation window");
  Check(Work()["next"]==null&&Convert.ToInt32(Work()["current"])==26789,"Successful insertion explicitly advances to the locked Next identity");
  await Command("block-protect","position",35778,"enabled",true);
  Check(Convert.ToString(Work()["bank"])=="33-I","Next activation leaves selected key set unchanged");
  await Command("operation-new");await Command("insert-macro","id","o33-n11-inverse","phase","macro","append",true);
  await Command("goal","goal","endgame");await Command("review");Check(Convert.ToBoolean(Map(shell.Work["review"])["goal_met"]),"Second insertion finishes declared orbit goal");
  await Command("preview","review_id",Map(shell.Work["review"])["id"]);await Command("commit");
  Check(Items(shell.Work["block"]).Select(Map).All(m=>Convert.ToBoolean(m["satisfied"])),"Both block requirements met while first protected");
  await Command("checkpoint","name","postapproval-two-insertions");string completed=Hash();await Command("undo");await Command("redo");Check(Hash()==completed,"Undo redo retains exact completed state");
  await Command("filter","expression","all");await Command("focus","identity",17810);await Command("settings","view",LocalApi.D("local_center",55));
  await Command("bank","id","Views");Tool("local").Activate();Field<NativeCellView>("local").Focus();Image("05-twenty-cell-local",Tool("local"));Image("06-global",Tool("global"));
  var localWindow=Tool("local");var localArea=Screen.FromControl(localWindow).WorkingArea;int localWidth=Math.Min(localArea.Width,Math.Max(localWindow.MinimumSize.Width,560)),localHeight=Math.Min(localArea.Height,Math.Max(localWindow.MinimumSize.Height,460));localWindow.Bounds=new Rectangle(localArea.Left+Math.Min(37,localArea.Width-localWidth),localArea.Top+Math.Min(29,localArea.Height-localHeight),localWidth,localHeight);await Task.Delay(750);await Ready();Rectangle manualLocalBounds=localWindow.Bounds;WindowWithinScreen("manual-local-bounds",localWindow);
  foreach(string id in new[]{"local","global","keyboard","solve"}){var window=Tool(id);window.Close();Check(!window.IsDisposed&&!window.Visible,"User close hides reusable "+id+" tool");}Invoke("local");await Ready();
  Check(Tool("local").Bounds==manualLocalBounds,"Reopening Local preserves the solver's exact chosen window bounds");WindowWithinScreen("reopened-local-bounds",Tool("local"));
  Check(Field<NativeCellView>("local").CenterCell==55&&Hash()==completed,"Reopening Local preserves center and committed state");
  // Independent focused scopes join the complete run. Resets are disclosed here;
  // the separate continuous workflow performs no reset between its work cycles.
  foreach(string tool in new[]{"local","global","keyboard","solve"})Tool(tool).Hide();
  await Command("reset");await Command("fixture","name","e1");
  await WorkIntentNativeChecks.Run(shell,form,Ready,Check,Image);
  await Command("reset");await Command("fixture","name","e1");
  await CurrentScoreNativeChecks.Run(shell,form,Ready,Check,Image,CaptureDesktopAsync);
  await Command("reset");await Command("fixture","name","e1");
  await EndgameNativeChecks.Run(shell,form,Ready,Check,Image,CaptureDesktopAsync);
  report["state_hash"]=Hash();report["final_workspace"]=Work();exitCode=0;
 }catch(Exception e){report["error"]=e.ToString();Console.WriteLine(e);}finally{
  report["checks"]=checks;report["exit_code"]=exitCode;report["scope"]="Agent-operated native WinForms/MPUlt and exact engine. Synthetic legal fixture. PNGs are GDI control renders except desktop-*.png, which are actual Windows desktop captures. Synthetic key route, not physical typing.";
  report["command_timings"]=commandTimings;report["command_timing_scope"]="Stopwatch from explicit command dispatch through Send acceptance and IsReady, including native adoption/control updates and 70 ms Ready padding plus polling. Not physical input-to-GPU latency or human task time. No performance acceptance threshold.";
  File.WriteAllText(Path.Combine(output,"report.json"),new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(report));form.Close();
 }}
 static string TextPending(){return shell.Work["pending"]==null?null:new JavaScriptSerializer().Serialize(shell.Work["pending"]);}
 [STAThread] static int Main(string[] args){if(args.Length!=5)return 2;mode=args[4];output=Path.GetFullPath(args[3]);Directory.CreateDirectory(output);Program.UseEnglishUi();string exe=Path.GetFullPath(args[0]);Directory.SetCurrentDirectory(Path.GetDirectoryName(exe));AppDomain.CurrentDomain.AssemblyResolve+=delegate(object sender,ResolveEventArgs e){string p=Path.Combine(Path.GetDirectoryName(exe),new AssemblyName(e.Name).Name+".dll");return File.Exists(p)?Assembly.LoadFrom(p):null;};Application.EnableVisualStyles();Application.SetCompatibleTextRenderingDefault(false);var assembly=Assembly.LoadFrom(exe);form=(Form)Activator.CreateInstance(assembly.GetType("_3dedit.Form1",true));shell=new ExperimentShell(form,exe,args[1],args[2],mode);form.Shown+=async delegate{await Run();};Application.ThreadException+=delegate(object sender,System.Threading.ThreadExceptionEventArgs e){uiFailure=e.Exception;};Application.Run(form);return exitCode;}
}
