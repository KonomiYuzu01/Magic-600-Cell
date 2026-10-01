// Agent-operated native controls with exact model effects; no human trial.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;
internal static class CycleNativeChecks {
 static T Field<T>(object value,string name){return (T)value.GetType().GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(value);}
 static object Call(object value,string name,params object[] args){return value.GetType().GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(value,args);}
 static Dictionary<string,object> Map(object value){return value as Dictionary<string,object>;}
 static object[] Items(object value){return value as object[]??new object[0];}
 static string Stable(ExperimentShell shell){var w=Map(shell.Work["workspace"]);return new JavaScriptSerializer().Serialize(new object[]{shell.Work["hash"],w["current"],w["next"],w["roles"],w["reference"],shell.Work["protected"]});}
 static string FocusDescription(Form host){Control active=host;while(active is ContainerControl){var child=((ContainerControl)active).ActiveControl;if(child==null)break;active=child;}return "Foreground="+(Form.ActiveForm==host)+", active="+active.GetType().Name+" / "+active.AccessibleName+" / "+active.Text;}
 static bool FullyVisible(Control control){var bounds=control.RectangleToScreen(control.ClientRectangle);for(Control parent=control.Parent;parent!=null;parent=parent.Parent)if(!parent.RectangleToScreen(parent.ClientRectangle).Contains(bounds))return false;return control.Visible;}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();string before=Stable(shell);var hub=Field<ExperimentHub>(shell,"hub");
  var savedDraft=Map(Map(shell.Work["workspace"])["draft"]);var savedDraftText=new JavaScriptSerializer().Serialize(savedDraft);var oldSize=host.Size;
  check(savedDraft.Values.All(v=>Items(v).Length==0),"Cycle regression starts with an explicitly empty operation");
  Call(shell,"RunCommand","cycles-current");await ready();
  using(var screen=host.CreateGraphics())Console.WriteLine("Native window environment: GDI dpi "+screen.DpiX+" x "+screen.DpiY+"; process screen "+Screen.FromControl(host).Bounds+"; window "+host.Bounds+"; font "+hub.Font);
  check(hub.CycleProjection!=null&&Convert.ToString(hub.CycleProjection["mode"])=="Current"&&Convert.ToString(hub.CycleProjection["relationKind"])=="home-ownership","Current displays authoritative residual ownership, not an operation");
  check(!Field<ComboBox>(hub,"cycleOrbitPicker").Enabled&&!Field<ComboBox>(hub,"effectScopePicker").Visible,"Current stays bound to the working orbit without a misleading macro scope");
  var currentEdges=hub.DetailedEdges.Cast<object>().Select(Map).ToArray();
  check(currentEdges.Length>0&&currentEdges.All(edge=>Convert.ToInt32(Map(edge["source"])["piece"])==Convert.ToInt32(edge["destination_position"])),"Current arrows end at the actual occupant's canonical Home");
  Call(shell,"RunCommand","cycles-after");await ready();
  check(hub.CycleProjection!=null&&!Convert.ToBoolean(hub.CycleProjection["projection_available"])&&Map(hub.CycleProjection["cycle_index"])["total"]==null,"Empty After is explicitly unavailable, not a solved empty graph");
  check(Field<List<Button>>(hub,"forecastObjects").Count==0,"Unavailable After cannot fall back to valid phase data and draw fake forecast tokens");
  check(Stable(shell)==before,"Read-only residual mode changes preserve Current, Next, protection and every label");
  var recipe=new object[]{LocalApi.D("kind","star","orbit",33,"node",0,"sign",1),LocalApi.D("kind","star","orbit",33,"node",11,"sign",1)};
  Call(shell,"RunCommand","cycles-steps");await ready();
  using(var empty=new Bitmap(hub.Width,hub.Height))using(var g=Graphics.FromImage(empty)){Call(hub,"PaintCanvas",hub,new PaintEventArgs(g,new Rectangle(Point.Empty,empty.Size)));hub.DrawToBitmap(empty,new Rectangle(Point.Empty,empty.Size));}
  check(hub.CycleProjection!=null&&Map(hub.CycleProjection["selected"])==null,"Empty operation paints without an invalid cycle index or an old effect");
  check(await shell.Send(LocalApi.D("action","draft","phase","macro","recipe",recipe)),"Explicitly compose two legal stars for graph inspection");await ready();
  var projection=hub.CycleProjection;check(projection!=null,"Composed steps return a current cycle projection");
  var cycles=Items(Map(projection["cycle_index"])["items"]).Select(Map).ToArray();
  check(cycles.Length==2&&cycles.All(c=>Convert.ToInt32(c["length"])==2),"Composite graph shows actual two transpositions, not component triangles");
  Call(shell,"RunCommand","cycles-after");await ready();
  check(hub.CycleProjection!=null&&Convert.ToString(hub.CycleProjection["mode"])=="After"&&Convert.ToString(hub.CycleProjection["source_boundary"])=="after-complete","After depicts the full copied-state operation result");
  check(Convert.ToString(hub.CycleProjection["source_hash"])==Convert.ToString(shell.Work["hash"])&&Convert.ToString(hub.CycleProjection["source_state_hash"])!=Convert.ToString(shell.Work["hash"]),"Predicted residual keeps actual and simulated state hashes distinct");
  check(Field<Label>(hub,"heading").Text.Contains("not executed")&&Stable(shell)==before,"After is labelled as prediction and performs no puzzle operation");
  image("cycles-after",host);
  Call(shell,"RunCommand","cycles-steps");await ready();
  var picker=Field<ComboBox>(hub,"cyclePicker");int second=Convert.ToInt32(cycles[1]["anchor"]);
  picker.SelectedIndex=1;await ready();
  check(Convert.ToInt32(Map(hub.CycleProjection["selected"])["anchor"])==second,"First selection after editing chooses the requested second cycle");
  var chosen=Map(hub.CycleProjection["selected"]);var edges=Items(chosen["edges"]).Select(Map).ToArray();
  check(edges.Length==2&&Convert.ToInt32(edges[0]["destination_position"])==Convert.ToInt32(edges[1]["source_position"]),"Directed graphical edges follow exact source/destination positions");
  host.Activate();check(picker.Focus()&&picker.ContainsFocus,"Cycle selection is a visible keyboard focus target");typeof(Control).GetMethod("OnKeyDown",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(picker,new object[]{new KeyEventArgs(Keys.Enter)});
  check(Field<Panel>(hub,"entryPane").Visible&&Field<Control>(hub,"cycleSlotGraphic").Visible,"Enter exposes the selected edge's graphical sticker correspondence");
  check(Stable(shell)==before,"Cycle selection and exact correspondence preserve full labels, Current and locked Next");
  image("cycles-composite-correspondence",host);
  Field<Button>(hub,"entryClose").PerformClick();
  var rings=Field<Panel>(hub,"cycleRings");var ring=rings.Controls.OfType<Button>().Last();int ringAnchor=Field<int>(Field<object>(ring,"Choice"),"Anchor");
  check(ring.Focus()&&ring.Focused,"Graphical cycle ring receives actual native focus");ring.PerformClick();await ready();
  check(rings.Controls.OfType<Button>().Any(b=>Field<int>(Field<object>(b,"Choice"),"Anchor")==ringAnchor&&b.Focused),"Graphical cycle selection restores focus to the same canonical ring · "+FocusDescription(host)+", pending="+Field<int>(hub,"pendingCycleFocus"));
  ring=rings.Controls.OfType<Button>().Last();check(ring.Focus(),"Ring accepts focus before a deliberate focus change");ring.PerformClick();
  var workOrbit=Field<ComboBox>(shell,"orbit");check(workOrbit.Focus(),"Solver deliberately focuses another main-window control while inspection runs");await ready();
  check(workOrbit.Focused,"Cycle response does not steal focus from the orbit control");
  host.Activate();ring=rings.Controls.OfType<Button>().Last();check(ring.Focus(),"Ring accepts focus before auxiliary-window check");ring.PerformClick();
  using(var other=new Form{Text="Test editor focus",Size=new Size(320,130)}){var text=new TextBox{Dock=DockStyle.Top};other.Controls.Add(text);other.Show(host);other.Activate();check(text.Focus(),"Independent test editor owns focus during inspection");await ready();check(Form.ActiveForm==other&&text.Focused,"Cycle response does not steal another window's text focus");other.Close();}host.Activate();
  var orbitPicker=Field<ComboBox>(hub,"cycleOrbitPicker");orbitPicker.SelectedIndex=34;
  typeof(ComboBox).GetMethod("OnSelectionChangeCommitted",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(orbitPicker,new object[]{EventArgs.Empty});await ready();
  check(orbitPicker.SelectedIndex==34,"Explicit current-orbit override remains visibly explicit");
  orbitPicker.SelectedIndex=0;typeof(ComboBox).GetMethod("OnSelectionChangeCommitted",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(orbitPicker,new object[]{EventArgs.Empty});await ready();
  check(Field<int?>(shell,"cycleOrbit")==null,"Working orbit option explicitly restores follow-work behavior");
  check(await shell.Send(LocalApi.D("action","draft","phase","prepare","recipe",new[]{LocalApi.D("kind","star","orbit",33,"node",0,"sign",1)})),"Supply an explicit Prepare that changes entry occupants");await ready();
  var entryEdge=hub.DetailedEdges.Cast<object>().Select(Map).First(e=>Convert.ToInt32(Map(e["source"])["piece"])!=Convert.ToInt32(Map(e["actual_source"])["piece"]));
  Field<TextBox>(hub,"frameReadout").Text="";typeof(ExperimentHub).GetField("inspectedCycleSource",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(hub,Convert.ToInt32(entryEdge["source_position"]));typeof(ExperimentHub).GetField("inspectedCycleDestination",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(hub,Convert.ToInt32(entryEdge["destination_position"]));Call(hub,"RefreshFrameReadout");
  string detail=Field<TextBox>(hub,"frameReadout").Text;check(detail.Contains("Source entry")&&detail.Contains("Destination entry")&&!detail.Contains("Destination token is its entry occupant"),"Entry comparison identifies prepared occupants separately from actual tokens");
  check(await shell.Send(LocalApi.D("action","draft","phase","prepare","recipe",new object[0])),"Remove the explicit test Prepare");await ready();
  Call(shell,"RunCommand","cycles-complete");await ready();check(Convert.ToString(hub.CycleProjection["scope"])=="complete","Complete scope inspects the full supplied operation");
  var serializer=new JavaScriptSerializer{MaxJsonLength=64*1024*1024};var exactProjection=hub.CycleProjection;
  int member=Items(Map(exactProjection["selected"])["edges"]).Select(Map).Select(edge=>Convert.ToInt32(edge["source_position"])).First(p=>p!=Convert.ToInt32(Map(exactProjection["selected"])["anchor"]));
  bool memberAccepted=await (Task<bool>)Call(shell,"InspectOperationEvidence",33,member);await ready();exactProjection=hub.CycleProjection;
  bool memberShown=exactProjection!=null&&Object.Equals(exactProjection["requested_position"],member)&&Convert.ToInt32(Map(exactProjection["selected"])["anchor"])!=member&&Convert.ToInt32(Map(shell.Work["workspace"])["inspected_position"])==member;
  bool wrongEchoRejected=true;
  if(memberShown){foreach(object echo in new object[]{null,true,member.ToString(),Map(exactProjection["selected"])["anchor"]}){var mismatch=Map(serializer.DeserializeObject(serializer.Serialize(exactProjection)));mismatch["requested_position"]=echo;Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",mismatch));wrongEchoRejected=wrongEchoRejected&&hub.CycleProjection==null;}var missing=Map(serializer.DeserializeObject(serializer.Serialize(exactProjection)));missing.Remove("requested_position");Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",missing));wrongEchoRejected=wrongEchoRejected&&hub.CycleProjection==null;Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",exactProjection));}
  check(memberAccepted&&memberShown&&wrongEchoRejected&&Stable(shell)==before,"Non-anchor evidence selects its canonical member while rejecting missing, wrong-type and mismatched echoes without changing the solve state");
  check(Convert.ToString(exactProjection["review_context_id"])==Convert.ToString(Map(shell.Work["review_context"])["id"]),"Cycle and current workspace share one versioned ReviewContext");
  var obsolete=Map(serializer.DeserializeObject(serializer.Serialize(exactProjection)));obsolete["review_context_id"]="old-context";
  Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",obsolete));check(hub.CycleProjection==null,"A cycle from another ReviewContext is rejected even when its state hash matches");
  Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",exactProjection));
  obsolete=Map(serializer.DeserializeObject(serializer.Serialize(exactProjection)));Map(obsolete["actual_piece_filter"])["context_hash"]="old-filter";
  Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",obsolete));check(hub.CycleProjection==null,"Old display filtering is rejected without changing mathematical permission");
  Call(shell,"AdoptCycleInspection",LocalApi.D("cycle_projection",exactProjection));
  var phaseResult=Field<Dictionary<string,object>>(shell,"phaseInspection");check(phaseResult!=null,"The linked physical view has a current read-only analysis");
  string phaseMacro=phaseResult["macro_id"]==null?null:Convert.ToString(phaseResult["macro_id"]);
  check(ExperimentShell.PhaseInspectionMatches(phaseResult,shell.Work,Convert.ToString(phaseResult["boundary"]),phaseMacro),"The linked physical view matches the current ReviewContext");
  var oldPhase=Map(serializer.DeserializeObject(serializer.Serialize(phaseResult)));oldPhase["review_context_id"]="old-context";
  check(!ExperimentShell.PhaseInspectionMatches(oldPhase,shell.Work,Convert.ToString(oldPhase["boundary"]),phaseMacro),"A physical view from another ReviewContext is rejected");
  var visibleTools=Field<Dictionary<string,Form>>(shell,"workWindows").Values.Where(window=>window.Visible).Distinct().ToArray();foreach(var window in visibleTools)window.Hide();
  host.Activate();
  await Task.Delay(100);image("cycles-complete",host);host.Size=new Size(1000,650);await Task.Delay(100);
  Field<Panel>(hub,"scroll").AutoScrollPosition=new Point(0,200);await Task.Delay(70);
  check(Field<List<Button>>(hub,"cycleModeButtons").All(FullyVisible)&&FullyVisible(picker),"Compact scrolled scene keeps mode switches and cycle selection completely visible");
  image("cycles-compact",host);host.Size=oldSize;
  foreach(var window in visibleTools)window.Show(host);host.Activate();
  string bank=Convert.ToString(Map(shell.Work["workspace"])["bank"]);Call(shell,"RunCommand","bank-Cycles");await ready();
  var input=Field<ExperimentInput>(shell,"input");Call(shell,"FocusWorkspace");input.HandleKeyDown("KeyW",false,false,false,false,false,false);input.HandleKeyUp("KeyW");await ready();
  check(Convert.ToString(hub.CycleProjection["scope"])=="macro-steps","Visible Cycles set provides a real single-key scope action");
  Call(shell,"FocusWorkspace");input.HandleKeyDown("KeyZ",false,false,false,false,false,false);input.HandleKeyUp("KeyZ");await ready();
  check(Convert.ToString(hub.CycleProjection["mode"])=="Current","Cycles Z directly opens Current with visible residual semantics");
  Call(shell,"FocusWorkspace");input.HandleKeyDown("KeyC",false,false,false,false,false,false);input.HandleKeyUp("KeyC");await ready();
  check(Convert.ToString(hub.CycleProjection["mode"])=="After","Cycles C directly opens After without changing the selected key set");
  check(await shell.Send(LocalApi.D("action","bank","id",bank)),"Restore original key set after cycle inspection");await ready();
  foreach(string phase in new[]{"prepare","macro","cleanup"}){check(await shell.Send(LocalApi.D("action","draft","phase",phase,"recipe",savedDraft[phase])),"Restore explicit "+phase+" test draft");await ready();}
  Call(shell,"RunCommand","cycles-macro");await ready();check(Stable(shell)==before&&new JavaScriptSerializer().Serialize(Map(shell.Work["workspace"])["draft"])==savedDraftText,"Graph scope, keyboard and draft round trip preserve solving identities and full state");
 }
}
