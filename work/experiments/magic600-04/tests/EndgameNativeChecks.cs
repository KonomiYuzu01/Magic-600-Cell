// Agent-operated native controls on explicit legal witnesses; never a human solve claim.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class EndgameNativeChecks {
 static T Field<T>(ExperimentShell shell,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static void Call(ExperimentShell shell,string name,params object[] args){typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,args);}
 static Dictionary<string,object> Map(object value){return value as Dictionary<string,object>;}
 static object[] Items(object value){return value as object[]??new object[0];}
 static string Json(object value){return new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(value);}
 static string Id(object item){return Convert.ToString(item.GetType().GetField("Id",BindingFlags.Instance|BindingFlags.Public|BindingFlags.NonPublic).GetValue(item));}
 static Dictionary<string,object> Work(ExperimentShell shell){return Map(shell.Work["workspace"]);}
 static string Stable(ExperimentShell shell){var w=Work(shell);return Json(new object[]{shell.Work["hash"],w["current"],w["target"],w["next"],w["roles"],w["reference"],w["draft"],w["draft_sources"],w["block"],w["bank"],shell.Work["protected"],shell.Work["pending"]});}
 static void Invoke(ExperimentShell shell,string command){Call(shell,"RunCommand",command);}
 static IEnumerable<Control> Children(Control parent){foreach(Control child in parent.Controls){yield return child;foreach(Control nested in Children(child))yield return nested;}}
 static void Select(ComboBox combo,string id){combo.SelectedItem=combo.Items.Cast<object>().Single(item=>Id(item)==id);}
 static Button FindButton(Control parent,string text){return Children(parent).OfType<Button>().Single(b=>b.Text==text);}
 static bool Exposed(Control control){if(!control.Visible||!control.IsHandleCreated)return false;var actual=control.RectangleToScreen(control.ClientRectangle);var visible=actual;for(Control p=control.Parent;p!=null;p=p.Parent)visible=Rectangle.Intersect(visible,p.RectangleToScreen(p.ClientRectangle));return actual.Width>0&&actual.Height>0&&visible==actual;}
 static async Task Reveal(Control control){for(Control p=control.Parent;p!=null;p=p.Parent){var scroll=p as ScrollableControl;if(scroll!=null&&scroll.AutoScroll)scroll.ScrollControlIntoView(control);}await Task.Delay(90);}
 static async Task Send(ExperimentShell shell,Func<Task> ready,Action<bool,string> check,string action,params object[] values){var body=LocalApi.D(values);body["action"]=action;check(await shell.Send(body),"Explicit native fixture command: "+action);await ready();}
 static async Task Key(ExperimentShell shell,Form host,Func<Task> ready,string code){host.Activate();Call(shell,"FocusWorkspace");var input=Field<ExperimentInput>(shell,"input");input.HandleKeyDown(code,false,false,false,false,false,false);input.HandleKeyUp(code);await ready();}
 static async Task EndgameIdle(ExperimentShell shell,Func<Task> ready){await ready();var deadline=DateTime.UtcNow.AddSeconds(120);while(Field<bool>(shell,"endgamePending")){if(DateTime.UtcNow>=deadline)throw new TimeoutException(Field<Label>(shell,"endgameStatus").Text);await Task.Delay(30);}await ready();}
 static async Task<Form> Open(ExperimentShell shell,Form owner,string command,string title){owner.BeginInvoke((Action)delegate{Invoke(shell,command);});var deadline=DateTime.UtcNow.AddSeconds(30);while(DateTime.UtcNow<deadline){var dialog=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text==title);if(dialog!=null)return dialog;await Task.Delay(30);}throw new TimeoutException("Missing dialog: "+title);}

 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image,Func<string,Form,Task> desktop){
  await ready();check(host.Size==new Size(1280,720),"Endgame comparison scene uses the existing 1280 × 720 native host");
  await Send(shell,ready,check,"bank","id","Macro");string stable=Stable(shell);
  await Key(shell,host,ready,"Digit6");
  var solve=Field<Dictionary<string,Form>>(shell,"workWindows")["solve"];var panel=Field<TableLayoutPanel>(shell,"endgamePanel");var family=Field<ComboBox>(shell,"endgameFamily");var q=Field<ComboBox>(shell,"endgameQ");var r=Field<ComboBox>(shell,"endgameR");
  check(solve.Visible&&shell.ActiveSolvePage=="macros"&&panel.Visible,"The single-key Endgame route expands inside existing Solve Macros");
  check(family.SelectedIndex==0&&q.SelectedIndex==0&&r.SelectedIndex==0&&!Field<Button>(shell,"endgameCheck").Enabled,"No family or orientation parameter is silently selected");
  check(Stable(shell)==stable,"Opening Endgame preserves Current, locked Next, draft, protection and all labels");
  await Send(shell,ready,check,"inspect-position","position",35778);await Key(shell,host,ready,"Digit7");await EndgameIdle(shell,ready);
  var x=Field<Dictionary<string,object>>(shell,"endgameX");check(x!=null&&Convert.ToInt32(x["position"])==35778,"Use inspected as X binds the explicitly chosen canonical position");
  check(q.Items.Count==Items(x["choices"]).Length+1&&q.SelectedIndex==0,"Actual group choices arrive without choosing q");
  check(Field<Label>(shell,"endgameRoles").Text.Contains("match current roles"),"Retained A / B are explicitly compared with current work roles");
  await Send(shell,ready,check,"inspect-position","position",26789);check(Convert.ToInt32(Field<Dictionary<string,object>>(shell,"endgameX")["position"])==35778,"Inspecting another object never silently retargets X");
  Select(family,"final-b");q.SelectedIndex=1;check(!Field<Button>(shell,"endgameCheck").Enabled,"Last buffer B requires a separately chosen Y and r");
  await Send(shell,ready,check,"inspect-position","position",35778);await Key(shell,host,ready,"Digit8");await EndgameIdle(shell,ready);r.SelectedIndex=1;check(!Field<Button>(shell,"endgameCheck").Enabled,"The same position cannot silently serve as both X and Y");
  // 115 is the explicit second O33 auxiliary recorded in the audited all-orbit matrix.
  await Send(shell,ready,check,"inspect-position","position",115);await Key(shell,host,ready,"Digit8");await EndgameIdle(shell,ready);r.SelectedIndex=1;check(Field<Button>(shell,"endgameCheck").Enabled,"Two explicit distinct auxiliaries permit checking the chosen parameters");
  Select(family,"placement-star");check(r.Enabled==false,"Families without Y/r do not expose them as active parameters");
  stable=Stable(shell);await Key(shell,host,ready,"Minus");await EndgameIdle(shell,ready);var proof=Field<Dictionary<string,object>>(shell,"endgameProof");
  check(proof!=null&&Convert.ToString(proof["family"])=="placement-star"&&Convert.ToInt32(Map(proof["parameters"])["x"])==35778,"Single-key Check returns the exact explicitly chosen family");
  check(Stable(shell)==stable&&!Convert.ToBoolean(shell.Work["executable"]),"Checking a known family neither stages nor executes it");
  var name=Field<TextBox>(shell,"endgameName");name.Text="Native explicit retained placement";var save=Field<Button>(shell,"endgameSave");await Reveal(save);check(Exposed(save)&&save.Enabled,"Save is reachable after exact effect inspection and an explicit name");save.PerformClick();await EndgameIdle(shell,ready);
  var saved=Field<Dictionary<string,object>>(shell,"endgameSaved");check(saved!=null&&Json(saved["recipe"])==Json(proof["recipe"]),"Save keeps the inspected complete legal recipe in a new library entry");
  check(!save.Enabled&&Stable(shell)==stable,"Completed save disables duplicate submission and retains work/preview boundaries");
  solve.Activate();await Reveal(Field<Label>(shell,"endgameStatus"));image("endgame-solve-saved",solve);await desktop("endgame-solve-desktop",solve);
  var originalSize=solve.Size;solve.Size=solve.MinimumSize;await Task.Delay(150);
  foreach(Control control in new Control[]{family,q,Field<Label>(shell,"endgameXName"),Field<Label>(shell,"endgameRoles"),Field<Button>(shell,"endgameSelect")}){await Reveal(control);check(Exposed(control),"Compact Solve exposes endgame "+(control.AccessibleName??control.GetType().Name)+" through its real viewport");var label=control as Label;if(label!=null)check(label.PreferredHeight<=label.Height+2,"Compact mathematical label retains its complete measured height");}
  image("endgame-solve-compact",solve);solve.Size=originalSize;await Task.Delay(100);
  await Key(shell,host,ready,"BracketLeft");check(Field<string>(shell,"selectedMacro")==Convert.ToString(saved["id"])&&Stable(shell)==stable,"Select saved is explicit and does not insert any steps");
  await Send(shell,ready,check,"settings","phase","macro");await Key(shell,host,ready,"KeyR");check(Json(Map(Work(shell)["draft_sources"])["macro"]).Contains(Convert.ToString(saved["id"])),"Only explicit Add inserts the saved canonical macro revision");
  await Send(shell,ready,check,"operation-new");
  await GeometryChooser(shell,host,ready,check,image,desktop);
  await CandidateComparison(shell,host,ready,check,image,desktop);
  await SameOrbitMacroRestore(shell,ready,check);

  // Both lock modes use the same actual occupied position, not a Home-ID guess.
  await Send(shell,ready,check,"inspect-position","position",35778);await Send(shell,ready,check,"bank","id","Workspace");string hash=Convert.ToString(shell.Work["hash"]);
  await Key(shell,host,ready,"KeyK");check(Items(shell.Work["position_locks"]).Select(Map).Any(row=>Convert.ToInt32(row["position"])==35778&&Convert.ToString(row["mode"])=="position"),"Hold place key captures actual occupant while allowing orientation");
  await Key(shell,host,ready,"KeyG");check(Items(shell.Work["position_locks"]).Select(Map).Count(row=>Convert.ToInt32(row["position"])==35778)==2,"Adding exact protection preserves the distinct position-only requirement");
  await Key(shell,host,ready,"KeyL");check(Items(shell.Work["position_locks"]).Select(Map).Any(row=>Convert.ToInt32(row["position"])==35778&&Convert.ToString(row["mode"])=="exact"),"Free place removes only the requested weaker lock");
  await Key(shell,host,ready,"KeyB");check(!Items(shell.Work["position_locks"]).Select(Map).Any(row=>Convert.ToInt32(row["position"])==35778)&&Convert.ToString(shell.Work["hash"])==hash,"Free exact releases the remaining policy without a mechanical turn");

  // Explicit E1 inverse insertion, then the user-selected cross-orbit bookmark.
  await Send(shell,ready,check,"focus","identity",16043);await Send(shell,ready,check,"settings","view",LocalApi.D("local_center",91));
  await Send(shell,ready,check,"focus","identity",35778);await Send(shell,ready,check,"settings","view",LocalApi.D("local_center",13));
  await Send(shell,ready,check,"next-pin","identity",16043,"target",16043,"replace",true);
  await Send(shell,ready,check,"insert-macro","id","o33-n0-inverse","phase","macro");await Send(shell,ready,check,"bank","id","Operation");
  string sourceDraft=Json(Work(shell)["draft"]),sourceSources=Json(Work(shell)["draft_sources"]),bank=Convert.ToString(Work(shell)["bank"]);await Send(shell,ready,check,"review");
  check(Convert.ToBoolean(Map(shell.Work["review"])["goal_met"]),"The explicit legal E1 insertion actually meets its declared target");await Send(shell,ready,check,"preview","review_id",Map(shell.Work["review"])["id"]);await Send(shell,ready,check,"commit");
  check(Convert.ToInt32(Work(shell)["orbit"])==22&&Convert.ToInt32(Work(shell)["current"])==16043&&Work(shell)["next"]==null,"Successful explicit commit consumes locked Next and follows its orbit");
  check(Convert.ToString(Work(shell)["bank"])==bank&&Convert.ToInt32(Map(Work(shell)["view"])["local_center"])==91,"Cross-orbit activation retains the visible key set and restores destination Local context");
  var old=Map(Map(Work(shell)["contexts"])["33"]);check(Json(old["draft"])==sourceDraft&&Json(old["draft_sources"])==sourceSources,"Original orbit retains exact steps and macro revision after the transition");
  Invoke(shell,"solve-prepare");await ready();image("endgame-cross-orbit-solve",solve);await desktop("endgame-cross-orbit-desktop",solve);
  await Send(shell,ready,check,"orbit","orbit",33);check(Json(Work(shell)["draft"])==sourceDraft&&Convert.ToInt32(Map(Work(shell)["view"])["local_center"])==13,"Returning to the previous orbit resumes its own draft and Local center");
  var residual=await Open(shell,host,"residual-details","Current residual");check(Children(residual).OfType<TextBox>().Any(t=>t.ReadOnly&&t.Text.Contains("Necessary conditions")),"Residual details disclose necessary invariants without claiming a solution");((IButtonControl)residual.CancelButton).PerformClick();await ready();
  var delta=await Open(shell,host,"journal-delta","Last committed step");check(Children(delta).OfType<ListBox>().Any(l=>l.Items.Count>0),"The existing journal exposes actual changed positions after orbit return");((IButtonControl)delta.CancelButton).PerformClick();await ready();
 }

 static async Task CandidateComparison(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image,Func<string,Form,Task> desktop){
  var search=Field<TextBox>(shell,"search");search.Text="o33-n";await ready();var list=Field<ListBox>(shell,"macros");check(list.Items.Count>1&&list.Items.Count<=12,"An explicit catalogue filter chooses a bounded comparison set");
  string stable=Stable(shell),selected=Field<string>(shell,"selectedMacro");var chosen=list.Items.Cast<object>().Select(Id).ToArray();var timer=System.Diagnostics.Stopwatch.StartNew();await Key(shell,host,ready,"Semicolon");
  var deadline=DateTime.UtcNow.AddSeconds(120);while(Field<bool>(shell,"candidateChecking")){if(DateTime.UtcNow>=deadline)throw new TimeoutException("Candidate comparison did not finish");await Task.Delay(30);}await ready();timer.Stop();
  var batch=Map(shell.Work["candidates"]);check(batch!=null&&Items(batch["candidates"]).Length==chosen.Length,"Single-key comparison returns a result for every explicitly visible candidate");
  check(Stable(shell)==stable&&Field<string>(shell,"selectedMacro")==selected,"Candidate ordering preserves identities, selected macro, draft, protection and pending operation");
  var order=Items(batch["order"]).Select(Convert.ToString).ToArray();check(list.Items.Cast<object>().Select(Id).SequenceEqual(order),"Macro Base renders the computed canonical candidate order");
  Console.WriteLine("MEASURE candidate comparison: "+chosen.Length+" entries; "+timer.ElapsedMilliseconds+" ms; "+System.Text.Encoding.UTF8.GetByteCount(Json(batch))+" result bytes; agent-operated native fixture");
  Call(shell,"RunCommand","solve-macros");await ready();var solve=Field<Dictionary<string,Form>>(shell,"workWindows")["solve"];image("endgame-candidate-order",solve);await desktop("endgame-candidate-order-desktop",solve);
  search.Text="no matching macro fixture";await ready();check(list.Items.Count==0&&Field<string>(shell,"selectedMacro")==selected&&Stable(shell)==stable,"An empty catalogue filter retains selected identity and work");
  search.Text="o33-n";await ready();check(Map(shell.Work["candidates"])!=null,"A display filter does not invalidate mathematical candidate evidence");
  await Send(shell,ready,check,"goal","goal","prepare");check(shell.Work["candidates"]==null,"A changed work goal withdraws the previous current-use comparison");
  await Send(shell,ready,check,"goal","goal","insert");search.Text="";await ready();
 }

 static async Task SameOrbitMacroRestore(ExperimentShell shell,Func<Task> ready,Action<bool,string> check){
  string original=Field<string>(shell,"selectedMacro");check(original!=null&&Convert.ToString(Map(Work(shell)["selected_macro"])["id"])==original,"The selected macro is bound to the authoritative orbit context before saving");
  await Send(shell,ready,check,"checkpoint","name","native-endgame-selected-macro");
  var list=Field<ListBox>(shell,"macros");list.SelectedItem=list.Items.Cast<object>().First(item=>Id(item)!=original);await ready();string changed=Field<string>(shell,"selectedMacro");check(changed!=original&&Convert.ToString(Map(Work(shell)["selected_macro"])["id"])==changed,"A separate explicit library selection persists its actual canonical macro");
  await Send(shell,ready,check,"restore","name","native-endgame-selected-macro");
  check(Convert.ToString(Map(Work(shell)["selected_macro"])["id"])==original,"Same-orbit restore returns the saved authoritative macro binding");
  check(Field<string>(shell,"selectedMacro")==original&&list.SelectedItem!=null&&Id(list.SelectedItem)==original,"Same-orbit restore shows the saved macro in the native catalogue before Add");
  check(Field<Dictionary<string,object>>(shell,"selectedEffect")==null&&Field<Dictionary<string,object>>(shell,"phaseInspection")==null,"Restore withdraws the previous macro's body effect and mismatched forecast");
  string stable=Stable(shell);Call(shell,"Draw",new object[]{null});check(Stable(shell)==stable&&Field<string>(shell,"selectedMacro")==original,"A plain redraw preserves the restored authoritative selection");
  Call(shell,"UseSelectedMacro",true);await ready();check(Json(Map(Work(shell)["draft_sources"])[Convert.ToString(Work(shell)["phase"])]).Contains(original),"Add after restore uses the saved canonical macro rather than the previous screen selection");
  await Send(shell,ready,check,"operation-new");
 }

 static async Task GeometryChooser(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image,Func<string,Form,Task> desktop){
  string before=Stable(shell);var dialog=await Open(shell,host,"macro-geometry","Map a macro to another frame");
  var from=Children(dialog).OfType<ComboBox>().Single(c=>c.AccessibleName=="Geometry source cell");var to=Children(dialog).OfType<ComboBox>().Single(c=>c.AccessibleName=="Geometry destination cell");var sf=Children(dialog).OfType<ComboBox>().Single(c=>c.AccessibleName=="Geometry source frame");var df=Children(dialog).OfType<ComboBox>().Single(c=>c.AccessibleName=="Geometry destination frame");
  check(from.SelectedIndex==0&&to.SelectedIndex==0&&!FindButton(dialog,"Check reference").Enabled,"Reference editor never assumes the macro's missing source frame");Select(from,"1");await ready();Select(to,"7");await ready();
  check(sf.Items.Count==12&&df.Items.Count==12&&sf.SelectedIndex<0&&df.SelectedIndex<0,"Both explicit cells expose twelve actual proper frames without selecting one");sf.SelectedIndex=0;df.SelectedIndex=0;FindButton(dialog,"Check reference").PerformClick();await ready();
  var receipt=Field<Dictionary<string,object>>(shell,"lastCommandResult");check(Convert.ToBoolean(Map(receipt["proof"])["complete_action_equal"])&&Convert.ToInt32(Map(receipt["proof"])["labels"])==259800,"Explicit frame pair compiles legal turns and checks the complete relabelled action");
  check(Stable(shell)==before,"Reference check preserves Session, Current, Next and operation draft");
  dialog.Activate();await Task.Delay(60);var solve=Field<Dictionary<string,Form>>(shell,"workWindows")["solve"];
  Console.WriteLine("FOCUS before foreground capture: "+(Form.ActiveForm==null?"none":Form.ActiveForm.Text)+" / "+String.Join(" | ",Children(solve).OfType<Label>().Where(label=>label.Text.Contains("Focus:")).Select(label=>label.Text.Replace('\n',' ')).ToArray()));
  image("endgame-reference-explicit",dialog);await desktop("endgame-reference-desktop",dialog);
  Console.WriteLine("FOCUS after foreground capture: "+(Form.ActiveForm==null?"none":Form.ActiveForm.Text)+" / "+String.Join(" | ",Children(solve).OfType<Label>().Where(label=>label.Text.Contains("Focus:")).Select(label=>label.Text.Replace('\n',' ')).ToArray()));
  check(Children(solve).OfType<Label>().Any(label=>label.Text.Contains("Focus: Map a macro to another frame")),"Solve identifies the verified foreground reference editor instead of reporting outside focus");
  ((IButtonControl)dialog.CancelButton).PerformClick();await ready();check(Stable(shell)==before,"Closing without saving or selecting a reference variant retains the original workspace");
  for(int repeat=0;repeat<3;repeat++){
   var again=await Open(shell,host,"macro-geometry","Map a macro to another frame");again.Activate();await Task.Delay(150);
   Console.WriteLine("FOCUS repeat "+repeat+": "+(Form.ActiveForm==null?"none":Form.ActiveForm.Text)+" / "+String.Join(" | ",Children(solve).OfType<Label>().Where(label=>label.Text.Contains("Focus:")).Select(label=>label.Text.Replace('\n',' ')).ToArray()));
   check(Form.ActiveForm==again&&Children(solve).OfType<Label>().Any(label=>label.Text.Contains("Focus: Map a macro to another frame")),"Repeated reference-editor entry keeps actual focus and Solve readout consistent");
   ((IButtonControl)again.CancelButton).PerformClick();await ready();check(Stable(shell)==before,"Repeated editor cancel keeps the original work without changing a macro or reference");
  }
 }
}
