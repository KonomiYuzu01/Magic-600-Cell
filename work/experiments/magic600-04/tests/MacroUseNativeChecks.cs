// Agent-operated native routes with real backend facts; no human-solving claim.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;
internal static class MacroUseNativeChecks {
 static T Field<T>(ExperimentShell s,string n){return (T)typeof(ExperimentShell).GetField(n,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(s);}
 static void Invoke(ExperimentShell s,string n,params object[] args){typeof(ExperimentShell).GetMethod(n,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(s,args);}
 static Dictionary<string,object> Map(object x){return x as Dictionary<string,object>;}
 static object[] Items(object x){return x as object[]??new object[0];}
 static string Stable(ExperimentShell s){var w=Map(s.Work["workspace"]);return new JavaScriptSerializer().Serialize(new object[]{s.Work["hash"],w["current"],w["next"],w["draft"],w["roles"],w["reference"],w["bank"],w["target"],w["block"],w["prefix"],s.Work["pending"],s.Work["protected"],s.Work["position_locks"]});}
 static string ChoiceId(object item){return Convert.ToString(item.GetType().GetField("Id",BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Instance).GetValue(item));}
 static string UseStatus(Dictionary<string,object> record){return Convert.ToString(Map(record["use"])["status"]);}
 static string UseFacts(ExperimentShell s){return new JavaScriptSerializer().Serialize(Items(s.Work["library"]).Select(Map).Select(m=>new object[]{m["id"],m["use"]}).ToArray());}
 static bool Exposed(Control control){if(!control.Visible||!control.IsHandleCreated||control.Width<=0||control.Height<=0)return false;var whole=control.RectangleToScreen(control.ClientRectangle);var visible=whole;for(Control parent=control.Parent;parent!=null;parent=parent.Parent){if(!parent.Visible)return false;visible=Rectangle.Intersect(visible,parent.RectangleToScreen(parent.ClientRectangle));}return whole==Rectangle.Intersect(visible,Screen.FromControl(control).WorkingArea);}
 static string Geometry(Control control){return control.GetType().Name+" bounds="+control.Bounds+" client="+control.ClientRectangle+" preferred="+control.GetPreferredSize(new Size(Math.Max(1,control.Width),0))+" parent="+control.Parent.ClientRectangle+" exposed="+Exposed(control);}
 static Button LibraryButton(ExperimentShell s,Form solve){return Field<Dictionary<Button,string>>(s,"commandButtons").Single(p=>!p.Key.IsDisposed&&solve.Contains(p.Key)&&p.Value=="macro-check-library").Key;}
 static void DiscoveryLayout(ExperimentShell s,Form solve,Action<bool,string> check,string stage){
  var picker=Field<ComboBox>(s,"macroUsePicker");var button=LibraryButton(s,solve);var toolbar=(FlowLayoutPanel)picker.Parent;var state=Field<Label>(s,"macroState");
  Console.WriteLine("LAYOUT "+stage+" Solve="+solve.Bounds+" toolbar="+Geometry(toolbar)+" status="+Geometry(state));
  foreach(Control control in toolbar.Controls)Console.WriteLine("LAYOUT "+stage+" "+control.Text+" "+Geometry(control));Console.Out.Flush();
  check(button.Parent==toolbar&&button.Enabled&&button.ContextMenuStrip!=null&&button.ContextMenuStrip.Items.Cast<ToolStripItem>().Any(x=>x.Text.StartsWith("Change key")),stage+" one editable Check library action shares the use-filter toolbar");
  check(Field<TableLayoutPanel>(s,"solveMacroBody").AutoScrollPosition==Point.Empty,stage+" discovery is tested on the first screen without scrolling");
  check(Exposed(Field<TextBox>(s,"search"))&&toolbar.Controls.Cast<Control>().All(Exposed),stage+" search, use picker, Filter, Check library and Edit are fully exposed");
  int textHeight=state.GetPreferredSize(new Size(Math.Max(1,state.ClientSize.Width),0)).Height;
  check(Exposed(state)&&state.ClientSize.Height>=textHeight,stage+" empty-result text fits without ellipsis: needs "+textHeight+", available "+state.ClientSize.Height);
  foreach(System.Collections.DictionaryEntry pair in Field<System.Collections.IDictionary>(s,"lanes"))check(Exposed((Control)pair.Value),stage+" wrapped library toolbar preserves "+pair.Key+" phase control");
  var commands=Field<Dictionary<Button,string>>(s,"commandButtons");var strip=Field<FlowLayoutPanel>(s,"operationStrip");
  foreach(string id in new[]{"review","preview","commit","cancel-preview"}){var action=commands.Single(p=>p.Value==id&&p.Key.Parent==strip).Key;check(Exposed(action),stage+" wrapped library toolbar preserves "+id+" control · "+Geometry(action));}
 }
 static async Task DiscoveryBeforeCheck(ExperimentShell s,Form solve,Action<string> choose,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  // Only the focused run reaches this pristine E1 state. The full suite has already inspected macros.
  string before=Stable(s),selected=Field<string>(s,"selectedMacro"),facts=UseFacts(s);var initial=Items(s.Work["library"]).Select(Map).ToArray();
  int active=Convert.ToInt32(Map(s.Work["workspace"])["orbit"]),uncheckedCount=initial.Count(m=>UseStatus(m)=="Unchecked"),frameCount=initial.Count(m=>UseStatus(m)=="AwaitingVerification");
  check(selected==null&&uncheckedCount>0&&initial.All(m=>UseStatus(m)!="Checked"),"Fresh E1 discovery starts without a selected macro or invented checked effects");
  var search=Field<TextBox>(s,"search");var list=Field<ListBox>(s,"macros");var state=Field<Label>(s,"macroState");string query=search.Text;
  string fullName=Convert.ToString(typeof(ExperimentShell).GetMethod("OrbitName",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(s,new object[]{active}));
  choose("all");search.Text=fullName;await ready();
  var expected=initial.Where(m=>Convert.ToInt32(m["orbit"])==active).Select(m=>Convert.ToString(m["id"])).ToArray();var shown=list.Items.Cast<object>().Select(ChoiceId).ToArray();
  check(expected.Length>0&&expected.All(shown.Contains),"Full current-orbit mathematical name finds its unchecked library rows before analysis");
  search.Text="19 cap";await ready();shown=list.Items.Cast<object>().Select(ChoiceId).ToArray();
  check(fullName.Contains("19 cap")&&expected.All(shown.Contains),"Displayed 19 cap fragment finds current-orbit rows whose short captions omit that prefix");
  check(list.SelectedIndex==-1&&Field<string>(s,"selectedMacro")==selected&&Stable(s)==before&&UseFacts(s)==facts,"Name search does not select, add, check effects or alter Current, Next, draft and protection");
  image("macro-discovery-name-before-check",solve);search.Text="";choose("current");await ready();
  check(list.Items.Count==0&&state.Text.StartsWith("No verified matches")&&state.Text.Contains("Library: "+uncheckedCount+" effects not checked"),"Current-use empty result explains the library-wide unchecked prerequisite");
  check(frameCount==0?!state.Text.Contains("need a verified frame"):state.Text.Contains("Library: "+frameCount+" need a verified frame"),"Frame-verification count stays separate from unchecked effects");
  image("macro-discovery-current-empty",solve);DiscoveryLayout(s,solve,check,"Initial empty catalogue");
  var bounds=solve.Bounds;try{solve.Size=solve.MinimumSize;await ready();image("macro-discovery-current-empty-minimum",solve);DiscoveryLayout(s,solve,check,"Minimum empty catalogue");}finally{solve.Bounds=bounds;}
  choose("all");search.Text="no-such-macro-discovery-19c7";await ready();
  check(list.Items.Count==0&&state.Text.StartsWith("No search/filter matches")&&!state.Text.StartsWith("No verified matches")&&state.Text.Contains("Library: "+uncheckedCount+" effects not checked"),"All uses plus a nonexistent name is a search miss, not a promised verified match");
  search.Text=query;choose("current");await ready();
  check(Field<string>(s,"selectedMacro")==selected&&list.SelectedIndex==-1&&Stable(s)==before&&UseFacts(s)==facts,"Empty-result recovery retains full context and leaves selection and Add explicit");
 }
 [DllImport("user32.dll")]static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")]static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
 static void FocusEvidence(string step){var active=Form.ActiveForm;uint owner;GetWindowThreadProcessId(GetForegroundWindow(),out owner);string process="unavailable";try{process=Process.GetProcessById((int)owner).ProcessName;}catch(ArgumentException){}Console.WriteLine("FOCUS "+step+" · "+(active==null?"No active application window":active.Text)+" · "+(active==null||active.ActiveControl==null?"none":active.ActiveControl.GetType().Name)+" · foreground process "+process+" · own="+(owner==Process.GetCurrentProcess().Id));Console.Out.Flush();}
 internal static async Task Run(ExperimentShell s,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();string before=Stable(s),selected=Field<string>(s,"selectedMacro");
  Invoke(s,"RunCommand","solve-macros");await ready();FocusEvidence("opened Solve");
  var windows=Field<Dictionary<string,Form>>(s,"workWindows");var solve=windows["solve"];var picker=Field<ComboBox>(s,"macroUsePicker");string prior=Field<string>(s,"macroUseFilter");
  Action<string> choose=id=>{for(int i=0;i<picker.Items.Count;i++){if(ChoiceId(picker.Items[i])==id){picker.SelectedIndex=i;typeof(ComboBox).GetMethod("OnSelectionChangeCommitted",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(picker,new object[]{EventArgs.Empty});return;}}throw new Exception("Missing use filter: "+id);};
  if(Environment.GetEnvironmentVariable("MAGIC600_NATIVE_FOCUS")=="macro-use")await DiscoveryBeforeCheck(s,solve,choose,ready,check,image);
  var checkLibrary=LibraryButton(s,solve);check(checkLibrary.Enabled&&Exposed(checkLibrary),"Existing Check library button is reachable before analysis without selecting a macro");checkLibrary.PerformClick();await ready();FocusEvidence("library checked");
  var receipt=Field<Dictionary<string,object>>(s,"lastCommandResult");check(receipt.ContainsKey("checked_ids")&&Convert.ToInt32(receipt["remaining_count"])==0,"Explicit library check completes existing entries");
  check(Stable(s)==before&&Field<string>(s,"selectedMacro")==selected,"Library analysis preserves full state, draft, Next, bank and macro selection");
  var library=Items(s.Work["library"]).Select(Map).ToArray();var record=library.Single(x=>Convert.ToString(x["id"])=="o33-n0-inverse");
  check(Items(Map(record["use"])["memberships"]).Select(Map).Any(x=>Convert.ToInt32(x["orbit"])==33),"Real inverse Star belongs to its likely solving orbit");
  choose("current");check(Field<ListBox>(s,"macros").Items.Count>0,"Current-orbit use list is populated from checked facts");
  choose("26");var collateral=library.Single(x=>Convert.ToString(x["id"])=="o25-n0-forward");var summary=typeof(ExperimentShell).GetMethod("MacroUseSummary",BindingFlags.Instance|BindingFlags.NonPublic);check(Convert.ToString(summary.Invoke(s,new object[]{collateral})).StartsWith("Possible use"),"Below-threshold orbit candidate remains visibly available");
  var composed=new object[]{LocalApi.D("kind","star","orbit",33,"node",0,"sign",1),LocalApi.D("kind","star","orbit",33,"node",11,"sign",1)};
  check(await s.Send(LocalApi.D("action","save-macro","name","Two transpositions use-check","recipe",composed)),"Save explicitly supplied two-cycle composition in isolated test library");await ready();FocusEvidence("saved composition");
  var composedRecord=Items(s.Work["library"]).Select(Map).Single(x=>Convert.ToString(x["name"])=="Two transpositions use-check");check(await s.Send(LocalApi.D("action","macro-check-library","limit",1)),"Check real two-transposition composition without selecting it");await ready();
  composedRecord=Items(s.Work["library"]).Select(Map).Single(x=>Convert.ToString(x["name"])=="Two transpositions use-check");var classify=typeof(ExperimentShell).GetMethod("MacroClassificationSummary",BindingFlags.Instance|BindingFlags.NonPublic);check(Convert.ToString(classify.Invoke(s,new object[]{composedRecord}))=="Position only","Multiple pure position cycles receive the broad exact class");
  choose("unchecked");check(Field<ListBox>(s,"macros").Items.Count==0,"Checked catalogue has no invented unchecked results");
  check(Field<string>(s,"selectedMacro")==selected,"Use filter does not replace selected canonical macro");choose(prior);
  FocusEvidence("before explicit Solve activation");windows["solve"].Activate();await Task.Delay(100);FocusEvidence("before Solve capture");image("solve-macro-use",windows["solve"]);
  var w0=Map(s.Work["workspace"]);string bank=Convert.ToString(w0["bank"]);check(await s.Send(LocalApi.D("action","bank","id","Macro")),"Activate editable shared Macro key set");await ready();
  var input=Field<ExperimentInput>(s,"input");var bankProperty=typeof(ExperimentShell).GetProperty("Bank",BindingFlags.Instance|BindingFlags.NonPublic);var bindings=Map(Map(bankProperty.GetValue(s,null))["commands"]);
  object libraryRoute;check(bindings.TryGetValue("Digit1",out libraryRoute)&&Convert.ToString(libraryRoute)=="macro-check-library","Moving Check library preserves the editable Macro.Digit1 route");
  var keycaps=Field<Dictionary<Button,string>>(s,"physicalKeyboardKeys");foreach(string action in new[]{"solve-macros","solve-prepare","solve-protection","macro-check-library","solve-locate-a","solve-locate-b","solve-locate-target"}){string code=bindings.Single(x=>Convert.ToString(x.Value)==action).Key;var cap=keycaps.Single(x=>x.Value==code).Key;check(ExperimentShell.CompactCommand(action)!="Action"&&cap.AccessibleName.Contains(action.StartsWith("solve-locate")?"Inspect fixed":action=="macro-check-library"?"Check existing":"Open Solve"),"Visible keycap describes "+action);}
  foreach(string page in new[]{"prepare","protection","macros"}){
   string key=bindings.Single(x=>Convert.ToString(x.Value)=="solve-"+page).Key;check(!key.Contains("+"),"Solve "+page+" has an unmodified key in the visible Macro set");
   Invoke(s,"FocusWorkspace");input.HandleKeyDown(key,false,false,false,false,false,false);input.HandleKeyUp(key);await ready();check(s.ActiveSolvePage==page,"Actual input route opens Solve "+page);
  }
  check(await s.Send(LocalApi.D("action","bank","id",bank)),"Restore previous key set explicitly");await ready();check(Stable(s)==before,"Use filtering and explicit key-set round trip preserve solve context");
 }
}
