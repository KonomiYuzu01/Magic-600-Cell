// Agent-operated checks of actual native bindings and controls; no physical typing claim.
// The caller supplies an isolated 33-A workspace with the explicitly captured cap C55.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class KeyboardCompatibilityNativeChecks {
 static T Field<T>(ExperimentShell shell,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static string Json(object value){return new JavaScriptSerializer().Serialize(value);}
 static Dictionary<string,object> Copy(object value){return Map(new JavaScriptSerializer().DeserializeObject(Json(value)));}
 static Dictionary<string,object> Section(Dictionary<string,object> record,string key){object value;if(!record.TryGetValue(key,out value)||value==null){value=new Dictionary<string,object>();record[key]=value;}return Map(value);}
 static ExperimentInputState State(ExperimentShell shell){return (ExperimentInputState)typeof(ExperimentShell).GetMethod("InputState",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,null);}
 static bool Extras(ExperimentShell shell){return (bool)typeof(ExperimentShell).GetProperty("KeyboardExtrasVisible",BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell,null);}
 static void Command(ExperimentShell shell,string id){typeof(ExperimentShell).GetMethod("RunCommand",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,new object[]{id});}
 static Dictionary<string,object> LegacyBindings(Dictionary<string,object> original){
  var result=Copy(original);var local=Section(Section(result,"banks"),"33-A");
  Section(result,"commands").Remove("Backquote");Section(local,"commands").Remove("Backquote");
  Section(result,"twists").Remove("Backquote");Section(local,"twists").Remove("Backquote");
  local.Remove("grips");result["grips"]=new[]{"Digit1"};return result;
 }
 static bool FullyExposed(Control control){
  if(control.IsDisposed||!control.Visible||!control.IsHandleCreated||control.Width<=0||control.Height<=0)return false;
  Rectangle bounds=control.RectangleToScreen(control.ClientRectangle),visible=bounds;
  for(Control parent=control.Parent;parent!=null;parent=parent.Parent){if(!parent.Visible)return false;visible=Rectangle.Intersect(visible,parent.RectangleToScreen(parent.ClientRectangle));}
  return Rectangle.Intersect(visible,Screen.FromControl(control).WorkingArea)==bounds;
 }
 static async Task Apply(Func<Dictionary<string,object>,Task<bool>> send,Func<Task> ready,Action<bool,string> check,Dictionary<string,object> bindings,string description){
  check(await send(LocalApi.D("action","settings","keybinds",bindings)),"Compatibility fixture accepts "+description);await ready();
 }
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Func<Dictionary<string,object>,Task<bool>> send,Action<bool,string> check,Action<string,Form> image){
  var input=Field<ExperimentInput>(shell,"input");var panel=Field<FlowLayoutPanel>(shell,"keyboard");var keyboard=Field<Dictionary<string,Form>>(shell,"workWindows")["keyboard"];
  var api=Field<LocalApi>(shell,"api");var workspace=Map(shell.Work["workspace"]);var original=Copy(workspace["keybinds"]);
  string hash=Convert.ToString(shell.Work["hash"]),draft=Json(workspace["draft"]),next=Json(workspace["next"]),current=Json(shell.Work["current"]);
  byte[] labels=await Task.Factory.StartNew(()=>api.Bytes("labels"));bool originalExtras=Extras(shell);Point scroll=panel.AutoScrollPosition;Exception failure=null;
  try{
   check(Convert.ToString(workspace["bank"])=="33-A","Compatibility fixture retains the explicitly selected 33-A set");
   var bank=((object[])shell.Work["banks"]).Select(Map).Single(row=>Convert.ToString(row["id"])=="33-A");
   check(((object[])bank["slots"]).Select(value=>Convert.ToInt32(value)).SequenceEqual(new[]{55}),"Compatibility fixture uses only explicitly captured C55");
   check(keyboard.Visible&&keyboard.IsHandleCreated,"Compatibility checks use the actual open Keyboard window");
   input.Reset("Compatibility fixture starts with released input.");

   var twist=LegacyBindings(original);Section(twist,"twists")["Backquote"]="H1";
   await Apply(send,ready,check,twist,"legacy Backquote Twist");
   var state=State(shell);check(state.TwistKeys.ContainsKey("Backquote")&&state.TwistKeys["Backquote"]=="H1"&&!state.GripKeys.ContainsKey("Backquote")&&!state.CommandKeys.ContainsKey("Backquote"),"Existing Backquote Twist remains effective instead of the new convenience command");

   var grip=LegacyBindings(original);grip["grips"]=new[]{"Backquote"};
   await Apply(send,ready,check,grip,"legacy Backquote Grip");
   state=State(shell);check(state.GripKeys.ContainsKey("Backquote")&&state.GripKeys["Backquote"]==55&&!state.CommandKeys.ContainsKey("Backquote"),"Existing Backquote Grip still identifies captured C55 without a command override");

   var explicitCommand=Copy(grip);Section(explicitCommand,"commands")["Backquote"]="keyboard-extra";
   await Apply(send,ready,check,explicitCommand,"explicit Backquote command override");
   state=State(shell);check(state.CommandKeys.ContainsKey("Backquote")&&state.CommandKeys["Backquote"]=="keyboard-extra"&&state.GripKeys["Backquote"]==55,"An explicit user command retains precedence over the same physical Grip key");
   keyboard.Activate();panel.Focus();input.Reset("Compatibility command dispatch starts released.");bool before=Extras(shell);
   check(input.HandleKeyDown("Backquote",false,false,false,false,false),"Explicit Backquote command is accepted by the actual input router");input.HandleKeyUp("Backquote");await Task.Delay(30);
   check(Extras(shell)!=before&&input.ActiveGripCell==null&&!input.Pending,"Backquote dispatch toggles extra keys without selecting a Grip or entering a turn");

   var numpad=LegacyBindings(original);numpad["grips"]=new[]{"Numpad1"};Section(numpad,"commands").Remove("Numpad1");Section(Section(Section(numpad,"banks"),"33-A"),"commands").Remove("Numpad1");
   await Apply(send,ready,check,numpad,"unmodified Numpad1 Grip");
   if(Extras(shell))Command(shell,"keyboard-extra");
   state=State(shell);check(!Extras(shell)&&state.GripKeys.ContainsKey("Numpad1")&&state.GripKeys["Numpad1"]==55&&!state.CommandKeys.ContainsKey("Numpad1"),"Numpad1 retains its exact Grip mapping with modified extras collapsed");
   var key=Field<Dictionary<Button,string>>(shell,"physicalKeyboardKeys").Single(pair=>pair.Value=="Numpad1").Key;
   check(key.Visible&&key.Enabled,"A custom unmodified Numpad Grip stays visible and available without opening extra keys");
   keyboard.Activate();panel.ScrollControlIntoView(key);check(key.Focus(),"Actual Numpad1 key accepts focus without executing it");await Task.Delay(30);
   check(!Extras(shell)&&FullyExposed(key),"Numpad1 remains fully reachable while modified extras stay collapsed");
   var cells=((object[])Field<Dictionary<string,object>>(shell,"structure")["cell_names"]).Select(Map);string capName=Convert.ToString(cells.Single(cell=>Convert.ToInt32(cell["canonical_id"])==55)["name"]);
   bank=((object[])shell.Work["banks"]).Select(Map).Single(row=>Convert.ToString(row["id"])=="33-A");
   var frame=((object[])Map(bank["frames"])["55"]).Select(value=>Convert.ToInt32(value)).ToArray();var basis=((object[])Map(bank["frame_bases"])["55"]).Select(value=>Convert.ToInt32(value)).ToArray();
   check(frame.Length==4&&basis.Length==4&&frame.Distinct().Count()==4&&frame.All(basis.Contains),"Captured Grip and retained base contain the same four canonical corners");
   string expectedFrame="Frame abcd ← "+new string(frame.Select(vertex=>"abcd"[Array.IndexOf(basis,vertex)]).ToArray());
   var detail=Field<Label>(shell,"keyboardActionDetail");check(key.AccessibleName.Contains("Canonical cap C55")&&detail.Text.Contains("Grip "+capName)&&detail.Text.Contains(expectedFrame),"Focused Numpad1 exposes the actual C55 name and exact ordered-frame correspondence");
   check(FullyExposed(detail)&&detail.GetPreferredSize(new Size(detail.ClientSize.Width,0)).Height<=detail.ClientSize.Height+2,"Focused Grip meaning remains readable in the actual keyboard window");
   check(input.ActiveGripCell==null&&!input.Pending,"Inspecting a remapped key neither selects a Grip nor submits a turn");image("keyboard-compatibility-numpad",keyboard);
  }catch(Exception error){failure=error;}finally{input.HandleKeyUp("Backquote");input.HandleKeyUp("Numpad1");input.Reset("Compatibility fixture ended; release input.");}
  try{
   if(!await send(LocalApi.D("action","settings","keybinds",original)))throw new InvalidOperationException("Could not restore original keyboard bindings.");await ready();
   if(Extras(shell)!=originalExtras)Command(shell,"keyboard-extra");
   check(Json(Map(shell.Work["workspace"])["keybinds"])==Json(original),"Original binding record is restored through the real settings boundary");
   check(Convert.ToString(shell.Work["hash"])==hash&&Json(Map(shell.Work["workspace"])["draft"])==draft&&Json(Map(shell.Work["workspace"])["next"])==next&&Json(shell.Work["current"])==current,"Compatibility edits and focus preserve hash, draft, Current and locked Next");
   var after=await Task.Factory.StartNew(()=>api.Bytes("labels"));check(labels.SequenceEqual(after),"Compatibility checks leave all 259800 authoritative labels unchanged");
  }catch(Exception restoreError){if(failure!=null)throw new AggregateException("Keyboard compatibility check and restoration both failed.",failure,restoreError);throw;}finally{
   if(!panel.IsDisposed)panel.AutoScrollPosition=new Point(-scroll.X,-scroll.Y);if(!keyboard.IsDisposed&&keyboard.Visible){keyboard.Activate();panel.Focus();}
  }
  if(failure!=null)throw new InvalidOperationException("Keyboard compatibility check failed: "+failure.Message,failure);
 }
}
