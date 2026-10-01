// Actual Shell response checks; injected routes, not physical keyboard latency.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class KeyboardBatchNativeChecks {
 static T Field<T>(object owner,string name){for(Type type=owner.GetType();type!=null;type=type.BaseType){var field=type.GetField(name,BindingFlags.Instance|BindingFlags.NonPublic|BindingFlags.DeclaredOnly);if(field!=null)return (T)field.GetValue(owner);}throw new MissingFieldException(owner.GetType().Name,name);}
 static object Call(ExperimentShell shell,string name,params object[] values){return typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,values);}
 static void RestoreLayoutFlag(ExperimentShell shell,bool value){typeof(ExperimentShell).GetField("restoringWindows",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,value);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static string Json(object value){return new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(value);}
 static string Context(ExperimentShell shell){var w=Map(shell.Work["workspace"]);return Json(new object[]{shell.Work["hash"],w["current"],w["next"],w["target"],w["roles"],w["reference"],w["draft"],w["block"],w["goal"],shell.Work["protected"],shell.Work["position_locks"],shell.Work["pending"]});}
 static string Keyboard(ExperimentShell shell){
  var tips=Field<ToolTip>(shell,"tips");var keys=Field<Dictionary<Button,string>>(shell,"physicalKeyboardKeys");
  return Json(keys.OrderBy(pair=>pair.Value,StringComparer.Ordinal).Select(pair=>new object[]{pair.Value,pair.Key.Text,pair.Key.Enabled,pair.Key.Visible,pair.Key.AccessibleName,pair.Key.AccessibleDescription,tips.GetToolTip(pair.Key),pair.Key.Bounds.ToString(),pair.Key.BackColor.ToArgb(),Field<Color>(pair.Key,"CapColor").ToArgb(),Field<bool>(pair.Key,"SelectedGrip")}).ToArray());
 }
 static void CheckFinal(ExperimentShell shell,Action<bool,string> check,string action){
  string shown=Keyboard(shell);Call(shell,"RenderPhysicalKeyboard");string refreshed=Keyboard(shell);
  if(refreshed!=shown){string output=Environment.GetEnvironmentVariable("MAGIC600_NATIVE_OUTPUT");if(!String.IsNullOrEmpty(output)){File.WriteAllText(Path.Combine(output,"keyboard-batch-before.json"),shown);File.WriteAllText(Path.Combine(output,"keyboard-batch-after.json"),refreshed);}}
  check(refreshed==shown,action+" task completion already exposes the final full keyboard state; an immediate reference render changes nothing");
  check(Field<int>(shell,"keyboardBatchDepth")==0&&!Field<bool>(shell,"keyboardRefreshPending"),action+" leaves no pending keyboard batch");
 }

 static Control FocusedControl(Control root){if(root.Focused)return root;foreach(Control child in root.Controls){var found=FocusedControl(child);if(found!=null)return found;}return null;}
 static string HubImage(Image image){
  if(image==null)return null;
  using(var bitmap=new Bitmap(image)){
   var pixels=new int[bitmap.Width*bitmap.Height];for(int y=0;y<bitmap.Height;y++)for(int x=0;x<bitmap.Width;x++)pixels[y*bitmap.Width+x]=bitmap.GetPixel(x,y).ToArgb();
   var bytes=new byte[pixels.Length*4];Buffer.BlockCopy(pixels,0,bytes,0,bytes.Length);
   using(var hash=System.Security.Cryptography.SHA256.Create())return bitmap.Width+"x"+bitmap.Height+":"+Convert.ToBase64String(hash.ComputeHash(bytes));
  }
 }
 static object HubTag(Button button){var tag=button.Tag;return tag==null?null:new object[]{Field<string>(tag,"Kind"),Field<int>(tag,"Id"),Field<bool>(tag,"Forecast")};}
 static string HubActual(ExperimentHub hub){
  var tips=Field<ToolTip>(hub,"tip");var canvas=Field<Panel>(hub,"canvas");var tracking=Field<Panel>(hub,"tracking");
  var buttons=Field<List<Button>>(hub,"objects");
  return Json(new object[]{
   buttons.Select(button=>new object[]{button.IsDisposed,HubTag(button),button.Text,button.AccessibleName,button.AccessibleDescription,(int)button.AccessibleRole,tips.GetToolTip(button),
    button.Parent==canvas?"canvas":button.Parent==tracking?"tracking":"unexpected",button.Parent.Controls.GetChildIndex(button),button.Bounds.ToString(),button.TabIndex,button.TabStop,button.Enabled,button.Visible,button.Focused,
    button.BackColor.ToArgb(),button.ForeColor.ToArgb(),button.FlatAppearance.BorderColor.ToArgb(),button.FlatAppearance.BorderSize,button.FlatAppearance.MouseOverBackColor.ToArgb(),button.FlatAppearance.MouseDownBackColor.ToArgb(),
    button.Padding.ToString(),button.TextAlign.ToString(),button.ImageAlign.ToString(),button.Font.ToString(),HubImage(button.Image),
    Field<bool>(button,"IdentityToken"),Field<bool>(button,"Forecast"),Field<string>(button,"RoleMark"),Field<Color>(button,"RoleColor").ToArgb(),Field<string>(button,"PolicyMark"),Field<Color>(button,"PolicyColor").ToArgb(),Field<string>(button,"BindingMark"),Field<bool?>(button,"FilterMatch"),Field<bool>(button,"GripHighlighted"),Field<bool>(button,"EvidenceHighlighted")}).ToArray(),
   new[]{canvas,tracking}.Select(parent=>parent.Controls.Cast<Control>().Select(child=>new object[]{child.GetType().Name,child is Button?HubTag((Button)child):null,child.Text,child.TabIndex,child.TabStop,child.Visible,child.Bounds.ToString()}).ToArray()).ToArray()});
 }
 static bool HubImageDisposed(Image image){if(image==null)return true;try{int width=image.Width;return false;}catch(ArgumentException){return true;}}
 static async Task HubAgainstFresh(ExperimentShell shell,Func<Task> ready,Action<bool,string> check,string action){
  await ready();var hub=Field<ExperimentHub>(shell,"hub");var menu=Field<ContextMenuStrip>(hub,"objectMenu");
  // This comparison deliberately runs at a closed, settled menu boundary; never close a user's live menu to make it pass.
  check(!menu.Visible&&Field<object>(hub,"pendingMenu")==null&&!Field<bool>(hub,"menuPosted")&&Field<int>(hub,"inspectedCycleSource")<0,action+" Hub differential starts without pending menu or inspected edge state");
  var old=Field<List<Button>>(hub,"objects").ToArray();check(old.Length>0&&old.All(button=>!button.IsDisposed),action+" Hub has live actual controls for a nonempty differential");
  var active=Form.ActiveForm;var focused=Application.OpenForms.Cast<Form>().Select(FocusedControl).FirstOrDefault(control=>control!=null);bool actualFocus=old.Any(button=>button==focused);
  string context=Json(shell.Work),shown=HubActual(hub);var rebuild=typeof(ExperimentHub).GetMethod("Rebuild",BindingFlags.Instance|BindingFlags.NonPublic);
  rebuild.Invoke(hub,null);await ready();var cached=Field<List<Button>>(hub,"objects").ToArray();
  check(cached.Length==old.Length&&cached.Select((button,index)=>Object.ReferenceEquals(button,old[index])).All(value=>value)&&HubActual(hub)==shown,action+" unchanged actual presentation takes the real reuse path without changing visible output");
  var images=cached.Select(button=>button.Image).ToArray();
  foreach(var button in cached)button.GetType().GetField("PresentationKey",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(button,null);
  rebuild.Invoke(hub,null);await ready();var fresh=Field<List<Button>>(hub,"objects").ToArray();
  check(fresh.Length==cached.Length&&fresh.All(button=>!button.IsDisposed&&!cached.Contains(button))&&cached.All(button=>button.IsDisposed)&&images.All(HubImageDisposed),action+" forced fresh construction replaces every old actual control and releases its image");
  check(HubActual(hub)==shown,action+" cached and forced fresh actual controls agree on text, tooltips, accessibility, order, bounds, styles, tags, highlights and exact bitmap pixels");
  check(Json(shell.Work)==context&&Form.ActiveForm==active&&(actualFocus||focused==null||focused.Focused)&&!menu.Visible&&Field<object>(hub,"pendingMenu")==null,action+" differential preserves authoritative state, logical focus and closed-menu ownership");
 }

 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check){
  await ready();var originalWindow=Field<Form>(shell,"keyboardWindow");bool visible=originalWindow!=null&&!originalWindow.IsDisposed&&originalWindow.Visible;var active=Form.ActiveForm;bool restoring=Field<bool>(shell,"restoringWindows");
  var api=Field<LocalApi>(shell,"api");var workspace=Map(shell.Work["workspace"]);
  string bank=Convert.ToString(workspace["bank"]),previousBank=Convert.ToString(workspace["previous_bank"]),stable=Context(shell);object center=Map(workspace["view"])["local_center"];Exception failure=null;
  byte[] labels=await Task.Factory.StartNew(()=>api.Bytes("labels"));
  try{
   RestoreLayoutFlag(shell,true);try{Call(shell,"OpenFloatingKeyboard",false);}finally{RestoreLayoutFlag(shell,restoring);}
   check(await shell.Send(LocalApi.D("action","bank","id","33-A")),"Batch feedback uses an explicitly selected real orbit bank");await ready();
   var input=Field<ExperimentInput>(shell,"input");var panel=Field<FlowLayoutPanel>(shell,"keyboard");var window=Field<Form>(shell,"keyboardWindow");
   window.Activate();panel.Focus();input.Reset("Batch feedback begins with released input.");
   var state=(ExperimentInputState)Call(shell,"InputState");string grip=state.GripKeys.First(pair=>pair.Value>=1&&pair.Value<=600).Key;
   var key=Field<Dictionary<Button,string>>(shell,"physicalKeyboardKeys").Single(pair=>pair.Value==grip).Key;
   input.HandleKeyDown(grip,false,false,false,false,false);
   check(Field<bool>(key,"PhysicalPressed")&&input.ActiveGripCell.HasValue,"Grip feedback is live before batching begins");
   Call(shell,"BeginKeyboardBatch");
   try{
    input.HandleKeyDown("ShiftLeft",true,false,false,false,false);
    check(input.InverseShiftHeld&&Field<bool>(shell,"keyboardRefreshPending"),"Shift updates routing immediately while its full key legend refresh is pending");
    input.HandleKeyUp(grip);input.HandleKeyUp("ShiftLeft");
    check(!Field<bool>(key,"PhysicalPressed")&&!input.InverseShiftHeld,"Key-up and Shift release clear live feedback inside the synchronous batch");
   }finally{Call(shell,"EndKeyboardBatch");input.Reset("Batch feedback ended with released input.");}
   CheckFinal(shell,check,"Immediate input feedback");
   foreach(string id in new[]{"33-A","Macro","Keyboard"}){
    check(await shell.Send(LocalApi.D("action","bank","id",id)),"Keyboard batching accepts explicit bank "+id);CheckFinal(shell,check,"Bank "+id);await ready();await HubAgainstFresh(shell,ready,check,"Bank "+id);
   }
   foreach(int cell in new[]{1,7}){
    check(await shell.Send(LocalApi.D("action","settings","view",LocalApi.D("local_center",cell))),"Keyboard batching accepts explicit Local C"+cell);CheckFinal(shell,check,"Local C"+cell);await ready();await HubAgainstFresh(shell,ready,check,"Local C"+cell);
   }
   string prior=Context(shell);check(!await shell.Send(LocalApi.D("action","bank","id","missing-native-keyboard-regression")),"Invalid bank is rejected through the real recovery boundary");CheckFinal(shell,check,"Rejected bank");await ready();await HubAgainstFresh(shell,ready,check,"Rejected bank");
   check(Context(shell)==prior,"Rejected bank recovery preserves identities, draft, goal, protection and pending state");
  }catch(Exception error){failure=error;}
  try{
   if(previousBank!=bank){check(await shell.Send(LocalApi.D("action","bank","id",previousBank)),"Restore previous key set after batch checks");await ready();}
   check(await shell.Send(LocalApi.D("action","bank","id",bank)),"Restore the explicitly selected bank after batch checks");await ready();
   check(await shell.Send(LocalApi.D("action","settings","view",LocalApi.D("local_center",center))),"Restore Local center after batch checks");await ready();
   var window=Field<Form>(shell,"keyboardWindow");RestoreLayoutFlag(shell,true);try{if(!visible&&window!=null&&!window.IsDisposed)window.Hide();}finally{RestoreLayoutFlag(shell,restoring);}
   if(active!=null&&!active.IsDisposed&&active.Visible)active.Activate();
  }catch(Exception cleanup){if(failure!=null)throw new AggregateException(failure,cleanup);throw;}
  if(failure!=null)throw failure;
  check(Context(shell)==stable&&(await Task.Factory.StartNew(()=>api.Bytes("labels"))).SequenceEqual(labels),"Batch and error-recovery checks preserve every label and the complete solving context");
 }
}
