// Root-operated native integration checks. No physical-typing or human-solve claim.
using System;using System.Collections.Generic;using System.Linq;using System.Reflection;using System.Threading.Tasks;using System.Web.Script.Serialization;using System.Windows.Forms;
internal static class SessionNativeChecks {
 static T Field<T>(ExperimentShell s,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(s);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static object Value(Dictionary<string,object> data,string name){object value;return data!=null&&data.TryGetValue(name,out value)?value:null;}
 static Dictionary<string,object> Session(ExperimentShell s){return Map(s.Work["session"]);}
 static IEnumerable<Control> Children(Control root){foreach(Control child in root.Controls){yield return child;foreach(Control nested in Children(child))yield return nested;}}
 static string FocusPath(Form dialog){var focused=Children(dialog).FirstOrDefault(c=>c.Focused);if(focused==null)return "(none)";var parts=new List<string>();for(Control c=focused;c!=null;c=c.Parent)parts.Add(c.GetType().Name+":"+(!String.IsNullOrEmpty(c.AccessibleName)?c.AccessibleName:c.Name));parts.Reverse();return String.Join("/",parts.ToArray());}
 static string Stable(ExperimentShell s){var w=Map(s.Work["workspace"]);return new JavaScriptSerializer().Serialize(new object[]{s.Work["hash"],w["draft"],w["current"],w["next"],w["bank"],w["block"],s.Work["pending"]});}
 static void Invoke(ExperimentShell s,string id){typeof(ExperimentShell).GetMethod("RunCommand",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(s,new object[]{id});}
 static async Task<Form> Open(ExperimentShell s,Form host,string id,string title){host.BeginInvoke((Action)delegate{Invoke(s,id);});var deadline=DateTime.UtcNow.AddSeconds(30);Form last=null;while(DateTime.UtcNow<deadline){last=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text==title);if(last!=null&&last.IsHandleCreated&&Form.ActiveForm==last)return last;await Task.Delay(30);}throw new TimeoutException("Session dialog did not activate: "+title+"; visible="+(last!=null)+"; ActiveForm="+(Form.ActiveForm==null?"null":Form.ActiveForm.Text)+"; path="+(last==null?"(no dialog)":FocusPath(last)));}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();var commands=Field<Dictionary<string,Action>>(shell,"commands");
  foreach(string id in new[]{"session","session-new","session-resume","scramble","session-timer-start","session-timer-pause","session-report","session-save-log","session-summary","reset-view","reset-workspace"})check(commands.ContainsKey(id),"Session has editable command "+id);
  string before=Stable(shell);var report=await Open(shell,host,"session","Session");
  check(Children(report).OfType<TextBox>().Any(t=>t.ReadOnly&&t.AccessibleName=="Authoritative session report"),"Session presents a read-only server report");
  var reportText=Children(report).OfType<TextBox>().Single(t=>t.AccessibleName=="Authoritative session report");var refresh=Children(report).OfType<Button>().Single(b=>b.Text=="Refresh");
  string presentation="selection="+reportText.SelectionStart+"/"+reportText.SelectionLength+" refresh.Focused="+refresh.Focused+" CanFocus="+refresh.CanFocus+" ActiveForm="+(Form.ActiveForm==null?"null":Form.ActiveForm.Text)+" ActiveControl="+(report.ActiveControl==null?"null":report.ActiveControl.GetType().Name)+" path="+FocusPath(report);
  Console.WriteLine("SESSION PRESENTATION "+presentation);Console.Out.Flush();image("session-report-before-focus-assert",report);
  check(reportText.SelectionLength==0&&refresh.Focused,"Session report opens without a blue select-all and focuses Refresh: "+presentation);
  ((IButtonControl)report.CancelButton).PerformClick();await ready();check(Stable(shell)==before,"Closing Session preserves work and every label");
  var scramble=await Open(shell,host,"scramble","Stage scramble");var count=Children(scramble).OfType<NumericUpDown>().Single(c=>c.AccessibleName=="Scramble length");var seed=Children(scramble).OfType<TextBox>().Single(c=>c.AccessibleName=="Scramble seed");
  count.Value=17;seed.Text="not a number";Children(scramble).OfType<Button>().Single(b=>b.AccessibleName=="Stage scramble in Prepare").PerformClick();await Task.Delay(100);
  check(scramble.Visible&&seed.Text=="not a number"&&Stable(shell)==before,"Invalid scramble seed stays editable without mutation");
  seed.Text="73";((IButtonControl)scramble.CancelButton).PerformClick();await ready();check(Stable(shell)==before,"Cancelling scramble never stages or executes the entered word");
  foreach(string id in new[]{"session-new","reset-workspace"}){var dialog=await Open(shell,host,id,id=="session-new"?"New solve":"Reset workspace");check(Children(dialog).OfType<Label>().Any(l=>l.AccessibleName=="Session action scope"),"Destructive presentation change names its scope before Apply");((IButtonControl)dialog.CancelButton).PerformClick();await ready();check(Stable(shell)==before,"Cancel "+id+" preserves work");}
  var shown=await Open(shell,host,"session-report","Session");image("session-report",shown);((IButtonControl)shown.CancelButton).PerformClick();await ready();
 }
 // Call only after a real successful commit which returned a new completion.
 // This helper never creates a receipt or invokes the popup itself.
 internal static async Task<string> CloseCompletion(ExperimentShell shell,Func<Task> ready,Action<bool,string> check,Action<string,Form> image,string imageName){
  var deadline=DateTime.UtcNow.AddSeconds(120);Form summary=null;
  while(DateTime.UtcNow<deadline){summary=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text=="Solve summary");if(summary!=null&&summary.IsHandleCreated&&Form.ActiveForm==summary)break;await Task.Delay(40);}
  check(summary!=null&&Form.ActiveForm==summary,"A real new full-Home commit opens and activates the completion summary");
  var data=Session(shell);var saved=Map(Value(data,"recorded_completion"));string id=Convert.ToString(Value(saved,"id"));string stable=Stable(shell);
  check(id.Length>0&&Value(data,"completion")==null,"The automatic summary acknowledges the durable completion exactly once");
  check(Convert.ToString(saved["state_hash"])==Convert.ToString(shell.Work["hash"])&&Convert.ToString(saved["head"])==Convert.ToString(data["head"]),"Completion belongs to the actual current journal head and full state");
  var text=Children(summary).OfType<TextBox>().Single(t=>t.AccessibleName=="Certified completion report");
  check(text.ReadOnly&&text.Text.Contains(Convert.ToString(saved["predicate"])),"The summary displays the actual exact completion predicate");
  check(text.SelectionLength==0&&Children(summary).OfType<Button>().Single(b=>b.Text=="Close").Focused,"Completion opens with unselected readable text and explicit Close focus");
  check(Object.Equals(Value(Map(saved["source"]),"human_solve_certified"),false),"Agent practice is not reported as a certified human solve");
  if(image!=null&&imageName!=null)image(imageName,summary);((IButtonControl)summary.CancelButton).PerformClick();await ready();
  check(Stable(shell)==stable,"Closing a completion summary never changes work or labels");return id;
 }
 static async Task Send(ExperimentShell shell,Func<Task> ready,Action<bool,string> check,string action,params object[] values){var body=LocalApi.D(values);body["action"]=action;check(await shell.Send(body),"Session fixture accepted "+action);await ready();}
 static async Task NoPopup(Func<Task> ready,Action<bool,string> check,string cause){await ready();await Task.Delay(140);check(!Application.OpenForms.Cast<Form>().Any(f=>f.Visible&&f.Text=="Solve summary"),cause+" does not fabricate or repeat a completion popup");}
 static async Task ApplyNew(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check){
  var dialog=await Open(shell,host,"session-new","New solve");Children(dialog).OfType<Button>().Single(b=>b.Text=="Start solved").PerformClick();await ready();
  check(!dialog.Visible&&Object.Equals(Session(shell)["raw_full_home"],true),"Explicit New starts at Home and closes its scope confirmation");await NoPopup(ready,check,"New");
 }
 static async Task StageStar(ExperimentShell shell,Func<Task> ready,Action<bool,string> check,int sign){
  await Send(shell,ready,check,"operation-new");await Send(shell,ready,check,"goal","goal","prepare");
  await Send(shell,ready,check,"draft","phase","prepare","recipe",new object[]{LocalApi.D("kind","star","orbit",33,"node",0,"sign",sign)});
  await Send(shell,ready,check,"review");await Send(shell,ready,check,"preview","review_id",Map(shell.Work["review"])["id"]);
 }
 static object Camera(ExperimentBridge bridge){return Reflect.Property(Reflect.Property(bridge.Viewport,"Scene"),"Camera");}
 static string CameraFrame(ExperimentBridge bridge){var camera=Camera(bridge);return new JavaScriptSerializer().Serialize(new object[]{((double[,])Reflect.Get(Reflect.Get(camera,"Trans"),"M")).Cast<double>().ToArray(),Reflect.Get(camera,"R"),Reflect.Property(camera,"Angle")});}
 static void ChangeCamera(ExperimentBridge bridge){
  var camera=Camera(bridge);var matrix=(double[,])((double[,])Reflect.Get(Reflect.Get(camera,"Trans"),"M")).Clone();
  for(int row=0;row<matrix.GetLength(0);row++){double first=matrix[row,0];matrix[row,0]=-matrix[row,1];matrix[row,1]=first;}
  lock(bridge.Viewport){Reflect.Set(Reflect.Get(camera,"Trans"),"M",matrix);Reflect.Set(camera,"R",Convert.ToDouble(Reflect.Get(camera,"R"))*1.15);Reflect.SetProperty(camera,"Angle",Convert.ToDouble(Reflect.Property(camera,"Angle"))*.9);Reflect.Call(camera,"SetChanged");Reflect.Call(bridge.Viewport,"SetSceneChanged");}
 }
 // Destructive only within the root launcher's disposable Session fixture.
 internal static async Task RunFresh(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();await ApplyNew(shell,host,ready,check);
  var bridge=Field<ExperimentBridge>(shell,"bridge");string defaultFrame=CameraFrame(bridge),beforeView=Stable(shell);ChangeCamera(bridge);
  check(CameraFrame(bridge)!=defaultFrame,"The camera fixture changes the actual retained viewport transform, radius and angle");
  Invoke(shell,"reset-view");await ready();check(CameraFrame(bridge)==defaultFrame&&Stable(shell)==beforeView,"Reset view restores the observed default camera without changing puzzle or work");
  await Send(shell,ready,check,"settings","view",LocalApi.D("local_center",55));ChangeCamera(bridge);await ApplyNew(shell,host,ready,check);
  check(CameraFrame(bridge)==defaultFrame&&Field<NativeCellView>(shell,"local").CenterCell==1,"New restores the actual default camera and Local center one");
  string initial=Stable(shell),attempt=Convert.ToString(Session(shell)["attempt_id"]);
  await Send(shell,ready,check,"session-resume");check(Stable(shell)==initial,"Resume preserves the saved puzzle and workspace");await NoPopup(ready,check,"Resume");
  await Send(shell,ready,check,"session-timer","command","start");check(Object.Equals(Map(Session(shell)["timer"])["running"],true),"Start uses the shared engine timer");
  await Send(shell,ready,check,"session-timer","command","pause");check(Object.Equals(Map(Session(shell)["timer"])["running"],false),"Pause stops the same engine timer");
  string hash=Convert.ToString(shell.Work["hash"]);var scramble=await Open(shell,host,"scramble","Stage scramble");
  Children(scramble).OfType<NumericUpDown>().Single(c=>c.AccessibleName=="Scramble length").Value=1;
  Children(scramble).OfType<TextBox>().Single(c=>c.AccessibleName=="Scramble seed").Text="73";
  Children(scramble).OfType<Button>().Single(c=>c.AccessibleName=="Stage scramble in Prepare").PerformClick();await ready();
  var draft=Map(Map(shell.Work["workspace"])["draft"]);
  check(!scramble.Visible&&Convert.ToString(shell.Work["hash"])==hash&&((object[])draft["prepare"]).Length>0&&shell.Work["pending"]==null,"A real scramble is staged in Prepare without moving or previewing the puzzle");await NoPopup(ready,check,"Scramble staging");
  await StageStar(shell,ready,check,1);await Send(shell,ready,check,"commit");await NoPopup(ready,check,"The non-Home legal commit");
  check(!Object.Equals(Session(shell)["raw_full_home"],true),"The first legal Star really leaves full Home");
  await StageStar(shell,ready,check,-1);
  // Controlled receipt race: a real owned editor stays open while the fixture
  // submits a real commit. This is not a claim that user keys bypass modal input.
  var receiptGate=await Open(shell,host,"session-report","Session");
  check(await shell.Send(LocalApi.D("action","commit")),"The explicit inverse commits through the normal transaction");
  await ready();check(receiptGate.Visible&&Value(Session(shell),"completion")!=null&&!Application.OpenForms.Cast<Form>().Any(f=>f.Visible&&f.Text=="Solve summary"),"A real completion waits unacknowledged while an owned editor remains open");
  ((IButtonControl)receiptGate.CancelButton).PerformClick();
  string receipt=await CloseCompletion(shell,ready,check,image,"session-completion");
  foreach(string action in new[]{"session-report","session-resume","undo","redo"}){await Send(shell,ready,check,action);await NoPopup(ready,check,action);}
  check(Convert.ToString(Map(Session(shell)["recorded_completion"])["id"])==receipt,"Redo exposes the same recorded completion without making a new record");
  var reopened=await Open(shell,host,"session-summary","Solve summary");check(Children(reopened).OfType<TextBox>().Single(t=>t.AccessibleName=="Certified completion report").Text.Contains("not a new completion"),"Last completion reopens the acknowledged record with explicit scope");((IButtonControl)reopened.CancelButton).PerformClick();await ready();
  await ApplyNew(shell,host,ready,check);check(Convert.ToString(Session(shell)["attempt_id"])!=attempt&&Value(Session(shell),"recorded_completion")==null,"A new attempt cannot present a previous attempt's completion as current");
  await Send(shell,ready,check,"session-reset","scope","puzzle");await NoPopup(ready,check,"Reset puzzle");
  await Send(shell,ready,check,"session-reset","scope","workspace");await NoPopup(ready,check,"Reset workspace");
 }
}
